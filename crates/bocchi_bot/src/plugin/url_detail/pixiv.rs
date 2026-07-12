use std::{sync::LazyLock, time::Duration};

use anyhow::{Context, Result, bail};
use async_tempfile::TempFile;
use bocchi::schema::{MessageContent, MessageSegment};
use futures::{StreamExt, stream::FuturesOrdered};
use reqwest::header;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use super::{RecognizedContent, RecognizedMessage};
use crate::utils::HTTP_CLIENT;

const PIXIV_ORIGIN: &str = "https://www.pixiv.net";
const MAX_IMAGES: usize = 10;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/138 Safari/537.36";

static PIXIV_ARTWORK_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"https?://(?:www\.)?pixiv\.net/(?:[a-z]{2}/)?artworks/(\d+)\b").unwrap());

#[derive(Debug, Clone)]
struct PixivLink {
    url: String,
    illust_id: String,
}

#[derive(Deserialize)]
struct AjaxResponse {
    error: bool,
    message: String,
    body: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IllustDetail {
    illust_title: String,
    illust_type: u8,
    page_count: usize,
    user_name: String,
    create_date: chrono::DateTime<chrono::FixedOffset>,
    tags: IllustTags,
    x_restrict: u8,
    ai_type: u8,
    bookmark_count: u64,
    like_count: u64,
    view_count: u64,
    comment_count: u64,
}

#[derive(Deserialize)]
struct IllustTags {
    tags: Vec<IllustTag>,
}

#[derive(Deserialize)]
struct IllustTag {
    tag: String,
}

#[derive(Deserialize)]
struct IllustPage {
    urls: PageUrls,
}

#[derive(Deserialize)]
struct PageUrls {
    original: String,
}

struct ParsedArtwork {
    segments: Vec<MessageSegment>,
    temp_files: Vec<TempFile>,
}

pub(crate) async fn recognizer(text: &str) -> Option<RecognizedMessage> {
    let links = parse_links(text);
    match links.len() {
        0 => None,
        1 => {
            let link = links.into_iter().next()?;
            let artwork = recognize_or_error(&link).await;
            Some(RecognizedMessage::new(
                RecognizedContent::Normal(artwork.segments),
                artwork.temp_files,
            ))
        }
        _ => Some(recognize_many(links).await),
    }
}

fn parse_links(text: &str) -> Vec<PixivLink> {
    PIXIV_ARTWORK_REGEX
        .captures_iter(text)
        .filter_map(|captures| {
            Some(PixivLink {
                url: captures.get(0)?.as_str().to_owned(),
                illust_id: captures.get(1)?.as_str().to_owned(),
            })
        })
        .collect()
}

async fn recognize_many(links: Vec<PixivLink>) -> RecognizedMessage {
    let mut futures = links
        .into_iter()
        .map(|link| async move {
            let artwork = recognize_or_error(&link).await;
            (link.url, artwork)
        })
        .collect::<FuturesOrdered<_>>();

    let mut messages = Vec::with_capacity(futures.len());
    let mut temp_files = Vec::new();
    while let Some((url, mut artwork)) = futures.next().await {
        artwork.segments.push(MessageSegment::Text {
            text: format!("\n链接：{url}"),
        });
        messages.push(MessageContent::Segment(artwork.segments));
        temp_files.append(&mut artwork.temp_files);
    }
    RecognizedMessage::new(RecognizedContent::Forward(messages), temp_files)
}

async fn recognize_or_error(link: &PixivLink) -> ParsedArtwork {
    match recognize_one(link).await {
        Ok(artwork) => artwork,
        Err(error) => {
            warn!("Pixiv 链接解析失败: {}, {error:#}", link.url);
            ParsedArtwork {
                segments: vec![MessageSegment::Text {
                    text: format!("Pixiv 链接解析失败：{error:#}"),
                }],
                temp_files: Vec::new(),
            }
        }
    }
}

async fn recognize_one(link: &PixivLink) -> Result<ParsedArtwork> {
    let detail: IllustDetail = fetch_ajax(&format!("{PIXIV_ORIGIN}/ajax/illust/{}?lang=zh", link.illust_id))
        .await
        .context("获取作品信息失败")?;

    if detail.illust_type != 0 {
        bail!("当前仅支持插画作品");
    }
    if detail.x_restrict != 0 {
        bail!("暂不支持 R18 作品");
    }

    let pages: Vec<IllustPage> = fetch_ajax(&format!("{PIXIV_ORIGIN}/ajax/illust/{}/pages?lang=zh", link.illust_id))
        .await
        .context("获取插画页面失败")?;
    if pages.is_empty() {
        bail!("作品没有可用图片");
    }

    let total_page_count = detail.page_count.max(pages.len());
    let shown_count = pages.len().min(MAX_IMAGES);
    let mut downloads = pages
        .into_iter()
        .take(shown_count)
        .enumerate()
        .map(|(index, page)| async move { download_image(index, &page.urls.original).await })
        .collect::<FuturesOrdered<_>>();

    let mut segments = Vec::with_capacity(shown_count + 2);
    let mut temp_files = Vec::with_capacity(shown_count);
    while let Some(result) = downloads.next().await {
        let (segment, temp_file) = result.context("下载插画失败")?;
        segments.push(segment);
        temp_files.push(temp_file);
    }

    let tags = detail
        .tags
        .tags
        .into_iter()
        .map(|tag| format!("#{}", tag.tag))
        .collect::<Vec<_>>()
        .join(" ");
    let mut detail_text = format!("标题：{}\n作者：{}", detail.illust_title, detail.user_name);
    if !tags.is_empty() {
        detail_text.push_str(&format!("\n标签：{tags}"));
    }
    if detail.ai_type == 2 {
        detail_text.push_str("\n标签：AI 生成");
    }
    detail_text.push_str(&format!(
        "\n🕒 {} | 💬 {} 🔖 {} ❤️ {} 👁 {}",
        detail.create_date.with_timezone(&chrono::Local),
        detail.comment_count,
        detail.bookmark_count,
        detail.like_count,
        detail.view_count,
    ));
    if total_page_count > MAX_IMAGES {
        detail_text.push_str(&format!(
            "\n作品共 {} 张，仅展示前 {} 张，后续图片请通过链接查看：{}",
            total_page_count, MAX_IMAGES, link.url
        ));
    }
    segments.push(MessageSegment::Text { text: detail_text });

    Ok(ParsedArtwork { segments, temp_files })
}

async fn fetch_ajax<T: for<'de> Deserialize<'de>>(url: &str) -> Result<T> {
    let response = HTTP_CLIENT
        .get(url)
        .header(header::USER_AGENT, USER_AGENT)
        .header(header::REFERER, PIXIV_ORIGIN)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await?
        .error_for_status()?
        .json::<AjaxResponse>()
        .await?;
    if response.error {
        bail!("Pixiv 返回错误：{}", response.message);
    }
    serde_json::from_value(response.body).context("Pixiv 返回的数据格式不正确")
}

async fn download_image(index: usize, url: &str) -> Result<(MessageSegment, TempFile)> {
    let response = HTTP_CLIENT
        .get(url)
        .header(header::USER_AGENT, USER_AGENT)
        .header(header::REFERER, PIXIV_ORIGIN)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await?
        .error_for_status()?;
    let is_image = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("image/"));
    if !is_image {
        bail!("第 {} 张图片返回了非图片内容", index + 1);
    }
    let bytes = response.bytes().await?;
    let mut temp_file = TempFile::new().await?;
    temp_file.write_all(&bytes).await?;
    temp_file.flush().await?;
    let segment = MessageSegment::Image {
        file: format!("file://{}", temp_file.file_path().to_string_lossy()),
        r#type: None,
        url: None,
        cache: Some(false),
        proxy: None,
        timeout: Some(15),
    };
    debug!("Pixiv 第 {} 张图片已写入临时文件", index + 1);
    Ok((segment, temp_file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_links() {
        let links =
            parse_links("https://www.pixiv.net/artworks/100000000 和 https://pixiv.net/en/artworks/12345678?foo=bar");
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].illust_id, "100000000");
        assert_eq!(links[1].illust_id, "12345678");
        assert_eq!(links[1].url, "https://pixiv.net/en/artworks/12345678");
    }

    #[test]
    fn test_ignore_non_artwork_links() {
        assert!(parse_links("https://www.pixiv.net/users/57190277").is_empty());
    }
}
