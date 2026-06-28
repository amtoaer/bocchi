use std::{sync::LazyLock, time::Duration};

use bocchi::schema::{MessageContent, MessageSegment};
use futures::{StreamExt, stream::FuturesOrdered};
use reqwest::header;
use scraper::{Html, Selector};

use super::RecognizedMessage;
use crate::utils::HTTP_CLIENT;

static X_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"https?://(?:www\.)?x\.com/(\w+)/status/(\d+)").unwrap());

#[derive(Debug, Clone)]
struct XLink {
    url: String,
    username: String,
    status_id: String,
}

/// Firefox UA，避免 nitter 返回空内容
const FIREFOX_UA: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:150.0) Gecko/20100101 Firefox/150.0";

// ---- 预编译的 CSS 选择器 ----

static MAIN_TWEET_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse("#m .timeline-item .tweet-body").unwrap());

static FULLNAME_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".fullname-and-username .fullname").unwrap());

static USERNAME_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".fullname-and-username .username").unwrap());

static CONTENT_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".tweet-content").unwrap());

static DATE_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".tweet-date a").unwrap());

static STATS_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".tweet-stats .tweet-stat").unwrap());

static QUOTE_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".quote").unwrap());

static QUOTE_TEXT_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".quote-text").unwrap());

static ATTACHMENT_IMG_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".attachments .gallery-row .attachment a.still-image").unwrap());

static MAIN_ATTACHMENT_IMG_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(":scope > .attachments .gallery-row .attachment a.still-image").unwrap());

static GALLERY_VIDEO_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".attachments .gallery-video").unwrap());

static MAIN_GALLERY_VIDEO_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(":scope > .attachments .gallery-video").unwrap());

static QUOTE_MEDIA_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".quote-media-container").unwrap());

// ---- 辅助函数 ----

/// 将 Nitter 的 UTC 时间字符串转为本地时区显示，解析失败则返回原文
fn format_date(raw: &str) -> String {
    chrono::NaiveDateTime::parse_from_str(raw, "%B %d, %Y · %I:%M %p UTC")
        .ok()
        .map(|dt| dt.and_utc().with_timezone(&chrono::Local).to_string())
        .unwrap_or_else(|| raw.to_owned())
}

fn clean_text(html: &str) -> String {
    let fragment = Html::parse_fragment(html);
    fragment
        .root_element()
        .text()
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_owned()
}

fn parse_tweet_stats(tweet_body: &scraper::ElementRef) -> (String, String, String, String) {
    let stats: Vec<String> = tweet_body
        .select(&STATS_SEL)
        .map(|el| {
            el.text()
                .collect::<String>()
                .trim()
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == ',')
                .collect::<String>()
        })
        .filter(|s| !s.is_empty())
        .collect();
    let comments = stats.first().cloned().unwrap_or_else(|| "0".to_string());
    let retweets = stats.get(1).cloned().unwrap_or_else(|| "0".to_string());
    let likes = stats.get(2).cloned().unwrap_or_else(|| "0".to_string());
    let views = stats.get(3).cloned().unwrap_or_else(|| "0".to_string());
    (comments, retweets, likes, views)
}

/// 从指定元素范围内提取图片 URL 列表（从 still-image 的 href 取原始图），并拼接 nitter 前缀
fn extract_images(scope: &scraper::ElementRef, sel: &Selector) -> Vec<String> {
    scope
        .select(sel)
        .filter_map(|el| el.value().attr("href"))
        .map(|href| format!("https://nitter.net{}", href))
        .collect()
}

/// 检测指定元素范围内是否存在视频
fn has_video(scope: &scraper::ElementRef, sel: &Selector) -> bool {
    scope.select(sel).next().is_some()
}

fn extract_x_links(text: &str) -> Vec<XLink> {
    X_REGEX
        .captures_iter(text)
        .filter_map(|caps| {
            Some(XLink {
                url: caps.get(0)?.as_str().to_owned(),
                username: caps.get(1)?.as_str().to_owned(),
                status_id: caps.get(2)?.as_str().to_owned(),
            })
        })
        .collect()
}

fn extract_quoted_tweet(tweet_body: &scraper::ElementRef, message_segments: &mut Vec<MessageSegment>) -> Option<()> {
    let quote_el = tweet_body.select(&QUOTE_SEL).next()?;

    let quoted_fullname = quote_el
        .select(&FULLNAME_SEL)
        .next()
        .map(|el| el.text().collect::<Vec<_>>().join("").trim().to_owned());

    let quoted_username = quote_el
        .select(&USERNAME_SEL)
        .next()
        .map(|el| el.text().collect::<String>().trim().trim_start_matches('@').to_owned());

    let quoted_content = quote_el
        .select(&QUOTE_TEXT_SEL)
        .next()
        .map(|el| clean_text(&el.inner_html()));

    let quoted_date = quote_el
        .select(&DATE_SEL)
        .next()
        .and_then(|el| el.value().attr("title"))
        .map(format_date);

    let (username, content) = match (quoted_username, quoted_content) {
        (Some(u), Some(c)) => (u, c),
        _ => return None,
    };

    // 引用推文上半：作者 + 内容（前一个 segment 是文本，需要 \n 换行）
    let mut quote_top = format!("\n---\n引用了 @{}", username);
    if let Some(ref fullname) = quoted_fullname
        && !fullname.is_empty() {
            quote_top.push_str(&format!(" ({})", fullname));
        }
    quote_top.push_str(&format!(":\n{}", content));
    message_segments.push(MessageSegment::Text { text: quote_top });
    let mut last_was_image = false;

    // 引用推文中的图片（位于引用内容与引用日期之间）
    if let Some(media_container) = quote_el.select(&QUOTE_MEDIA_SEL).next() {
        for img_url in extract_images(&media_container, &ATTACHMENT_IMG_SEL) {
            message_segments.push(MessageSegment::Image {
                file: img_url,
                r#type: None,
                url: None,
                cache: Some(true),
                proxy: Some(false),
                timeout: Some(10),
            });
            last_was_image = true;
        }
        // 引用推文中的视频
        if has_video(&media_container, &GALLERY_VIDEO_SEL) {
            let br = if last_was_image { "" } else { "\n" };
            message_segments.push(MessageSegment::Text {
                text: format!("{}[引用推文中含有视频，当前不支持解析]", br),
            });
            last_was_image = false;
        }
    }

    // 引用推文下半：日期
    if let Some(ref date) = quoted_date {
        let br = if last_was_image { "" } else { "\n" };
        message_segments.push(MessageSegment::Text {
            text: format!("{}🕒 {}", br, date),
        });
    }

    Some(())
}

// ---- 入口 ----

pub(crate) async fn recognizer(text: &str) -> Option<RecognizedMessage> {
    let links = extract_x_links(text);
    match links.len() {
        0 => None,
        1 => {
            let link = links.into_iter().next()?;
            recognize_one(&link).await.map(RecognizedMessage::Normal)
        }
        _ => recognize_many(links).await.map(RecognizedMessage::Forward),
    }
}

async fn recognize_many(links: Vec<XLink>) -> Option<Vec<MessageContent>> {
    let mut futures = links
        .into_iter()
        .map(|link| async move {
            let result = recognize_one(&link).await;
            (link.url, result)
        })
        .collect::<FuturesOrdered<_>>();

    let mut messages = Vec::with_capacity(futures.len());
    while let Some((url, result)) = futures.next().await {
        let content = result
            .map(|mut segments| {
                // 为了便于识别来源，在末尾加上链接
                segments.push(MessageSegment::Text {
                    text: format!("\n链接：{}", url),
                });
                MessageContent::Segment(segments)
            })
            .unwrap_or_else(|| MessageContent::Text(format!("X 链接解析失败：\n{}", url)));
        messages.push(content);
    }
    Some(messages)
}

async fn recognize_one(link: &XLink) -> Option<Vec<MessageSegment>> {
    let username = link.username.as_str();
    let status_id = link.status_id.as_str();

    let nitter_url = format!("https://nitter.net/{}/status/{}", username, status_id);

    let resp = HTTP_CLIENT
        .get(&nitter_url)
        .header(header::USER_AGENT, FIREFOX_UA)
        .header(header::ACCEPT_LANGUAGE, "zh-CN,en-US;q=0.9,en;q=0.8")
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;

    let html = Html::parse_document(&resp);

    // 定位主推文
    let tweet_body = html.select(&MAIN_TWEET_SEL).next()?;

    // 提取作者信息
    let author_fullname = tweet_body
        .select(&FULLNAME_SEL)
        .next()
        .map(|el| el.text().collect::<Vec<_>>().join("").trim().to_owned());

    let author_username = tweet_body
        .select(&USERNAME_SEL)
        .next()
        .map(|el| el.text().collect::<String>().trim().trim_start_matches('@').to_owned())
        .unwrap_or_else(|| username.to_owned());

    // 提取推文内容
    let content = tweet_body
        .select(&CONTENT_SEL)
        .next()
        .map(|el| clean_text(&el.inner_html()))
        .unwrap_or_default();

    // 提取日期
    let published_date = tweet_body
        .select(&DATE_SEL)
        .next()
        .and_then(|el| el.value().attr("title"))
        .map(format_date);

    // 提取统计数据
    let (comments, retweets, likes, views) = parse_tweet_stats(&tweet_body);

    let mut message_segments = Vec::new();
    // 追踪上一个 segment 是否为图片，以便在文本间插入换行（图片天然分隔）
    let mut last_was_image = false;

    // 构建正文前半：作者 + 内容
    let mut text_top = format!("@{}", author_username);
    if let Some(ref fullname) = author_fullname
        && !fullname.is_empty() && *fullname != author_username {
            text_top.push_str(&format!(" ({})", fullname));
        }
    text_top.push_str(&format!(":\n{}", content));
    message_segments.push(MessageSegment::Text { text: text_top });

    // 主推文图片（位于内容与日期之间）
    for img_url in extract_images(&tweet_body, &MAIN_ATTACHMENT_IMG_SEL) {
        message_segments.push(MessageSegment::Image {
            file: img_url,
            r#type: None,
            url: None,
            cache: Some(true),
            proxy: Some(false),
            timeout: Some(10),
        });
        last_was_image = true;
    }

    // 主推文视频提醒（位于图片后、日期前）
    if has_video(&tweet_body, &MAIN_GALLERY_VIDEO_SEL) {
        let br = if last_was_image { "" } else { "\n" };
        message_segments.push(MessageSegment::Text {
            text: format!("{}[推文中含有视频，当前不支持解析]", br),
        });
        last_was_image = false;
    }

    // 构建正文后半：日期、统计
    {
        let br = if last_was_image { "" } else { "\n" };
        let mut line = String::new();
        if let Some(ref date) = published_date {
            line.push_str(&format!("🕒 {}", date));
        }
        line.push_str(&format!(" | 💬 {} 🔄 {} ❤️ {} 👁 {}", comments, retweets, likes, views));
        message_segments.push(MessageSegment::Text {
            text: format!("{}{}", br, line),
        });
    }

    extract_quoted_tweet(&tweet_body, &mut message_segments);

    Some(message_segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex() {
        let testcases = [
            (
                "https://x.com/ttmokeyjay/status/2053000399548989514",
                Some(("ttmokeyjay", "2053000399548989514")),
            ),
            (
                "https://www.x.com/dearemon/status/2052919236616700334",
                Some(("dearemon", "2052919236616700334")),
            ),
            ("https://twitter.com/someone/status/1234567890", None),
            ("https://x.com/ttmokeyjay/status/abc", None),
        ];
        for (text, expected) in testcases.iter() {
            let caps = X_REGEX.captures(text);
            match expected {
                Some((user, id)) => {
                    assert!(caps.is_some(), "Expected match for: {}", text);
                    let caps = caps.unwrap();
                    assert_eq!(caps.get(1).unwrap().as_str(), *user);
                    assert_eq!(caps.get(2).unwrap().as_str(), *id);
                }
                None => {
                    assert!(caps.is_none(), "Expected no match for: {}", text);
                }
            }
        }
    }
}
