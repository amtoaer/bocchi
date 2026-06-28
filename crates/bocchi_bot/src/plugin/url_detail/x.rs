use std::{sync::LazyLock, time::Duration};

use bocchi::schema::{MessageContent, MessageSegment};
use futures::{StreamExt, stream::FuturesOrdered};
use reqwest::{StatusCode, header};
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

const NITTER_ORIGIN: &str = "https://nitter.net";

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

fn extract_images(scope: &scraper::ElementRef, sel: &Selector) -> Vec<String> {
    scope
        .select(sel)
        .filter_map(|el| el.value().attr("href"))
        .map(|href| format!("{}{}", NITTER_ORIGIN, href))
        .collect()
}

async fn fetch_nitter_html(url: &str) -> Option<String> {
    const FIREFOX_UA: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:150.0) Gecko/20100101 Firefox/150.0";
    const MAX_ATTEMPTS: usize = 5;
    const RETRY_BACKOFFS: [Duration; MAX_ATTEMPTS - 1] = [
        Duration::from_millis(300),
        Duration::from_millis(700),
        Duration::from_millis(1500),
        Duration::from_millis(3000),
    ];
    for attempt in 1..=MAX_ATTEMPTS {
        let result = HTTP_CLIENT
            .get(url)
            .header(header::USER_AGENT, FIREFOX_UA)
            .header(header::ACCEPT_LANGUAGE, "zh-CN,en-US;q=0.9,en;q=0.8")
            .send()
            .await;
        match result {
            Ok(resp) if resp.status().is_success() => match resp.text().await {
                Ok(text) => return Some(text),
                Err(e) => warn!(
                    "读取 Nitter 响应失败，第 {}/{} 次: {}, {:?}",
                    attempt, MAX_ATTEMPTS, url, e
                ),
            },
            Ok(resp) => {
                let status = resp.status();
                if !matches!(
                    status,
                    StatusCode::TOO_MANY_REQUESTS
                        | StatusCode::INTERNAL_SERVER_ERROR
                        | StatusCode::BAD_GATEWAY
                        | StatusCode::SERVICE_UNAVAILABLE
                        | StatusCode::GATEWAY_TIMEOUT
                ) {
                    warn!("Nitter 返回不可重试状态码 {}: {}", status, url);
                    return None;
                }
                warn!(
                    "Nitter 返回状态码 {}，第 {}/{} 次: {}",
                    status, attempt, MAX_ATTEMPTS, url
                );
            }
            Err(e) => warn!("请求 Nitter 失败，第 {}/{} 次: {}, {:?}", attempt, MAX_ATTEMPTS, url, e),
        }
        if let Some(backoff) = RETRY_BACKOFFS.get(attempt - 1) {
            tokio::time::sleep(*backoff).await;
        }
    }
    None
}

pub(crate) async fn recognizer(text: &str) -> Option<RecognizedMessage> {
    let links: Vec<XLink> = X_REGEX
        .captures_iter(text)
        .filter_map(|caps| {
            Some(XLink {
                url: caps.get(0)?.as_str().to_owned(),
                username: caps.get(1)?.as_str().to_owned(),
                status_id: caps.get(2)?.as_str().to_owned(),
            })
        })
        .collect();

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
    let nitter_url = format!("{}/{}/status/{}", NITTER_ORIGIN, username, status_id);

    let resp = fetch_nitter_html(&nitter_url).await?;
    let html = Html::parse_document(&resp);
    let tweet_body = html.select(&MAIN_TWEET_SEL).next()?;

    let author_fullname = tweet_body
        .select(&FULLNAME_SEL)
        .next()
        .map(|el| el.text().collect::<Vec<_>>().join("").trim().to_owned());
    let author_username = tweet_body
        .select(&USERNAME_SEL)
        .next()
        .map(|el| el.text().collect::<String>().trim().trim_start_matches('@').to_owned())
        .unwrap_or_else(|| username.to_owned());
    let content = tweet_body
        .select(&CONTENT_SEL)
        .next()
        .map(|el| clean_text(&el.inner_html()))
        .unwrap_or_default();
    let published_date = tweet_body
        .select(&DATE_SEL)
        .next()
        .and_then(|el| el.value().attr("title"))
        .map(format_date);
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

    let mut segments = Vec::new();
    let mut last_was_image = false;

    let mut text_top = format!("@{}", author_username);
    if let Some(ref fullname) = author_fullname
        && !fullname.is_empty()
        && *fullname != author_username
    {
        text_top.push_str(&format!(" ({})", fullname));
    }
    text_top.push_str(&format!(":\n{}", content));
    segments.push(MessageSegment::Text { text: text_top });

    for img_url in extract_images(&tweet_body, &MAIN_ATTACHMENT_IMG_SEL) {
        segments.push(MessageSegment::Image {
            file: img_url,
            r#type: None,
            url: None,
            cache: Some(true),
            proxy: Some(false),
            timeout: Some(10),
        });
        last_was_image = true;
    }

    if tweet_body.select(&MAIN_GALLERY_VIDEO_SEL).next().is_some() {
        let br = if last_was_image { "" } else { "\n" };
        segments.push(MessageSegment::Text {
            text: format!("{}[推文中含有视频，当前不支持解析]", br),
        });
        last_was_image = false;
    }

    let br = if last_was_image { "" } else { "\n" };
    let mut line = String::new();
    if let Some(ref date) = published_date {
        line.push_str(&format!("🕒 {}", date));
    }
    line.push_str(&format!(" | 💬 {} 🔄 {} ❤️ {} 👁 {}", comments, retweets, likes, views));
    segments.push(MessageSegment::Text {
        text: format!("{}{}", br, line),
    });

    if let Some(quote_el) = tweet_body.select(&QUOTE_SEL).next() {
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

        if let (Some(username), Some(content)) = (quoted_username, quoted_content) {
            let mut quote_top = format!("\n---\n引用了 @{}", username);
            if let Some(ref fullname) = quoted_fullname
                && !fullname.is_empty()
            {
                quote_top.push_str(&format!(" ({})", fullname));
            }
            quote_top.push_str(&format!(":\n{}", content));
            segments.push(MessageSegment::Text { text: quote_top });

            let mut quote_last_was_image = false;
            if let Some(media_container) = quote_el.select(&QUOTE_MEDIA_SEL).next() {
                for img_url in extract_images(&media_container, &ATTACHMENT_IMG_SEL) {
                    segments.push(MessageSegment::Image {
                        file: img_url,
                        r#type: None,
                        url: None,
                        cache: Some(true),
                        proxy: Some(false),
                        timeout: Some(10),
                    });
                    quote_last_was_image = true;
                }

                if media_container.select(&GALLERY_VIDEO_SEL).next().is_some() {
                    let br = if quote_last_was_image { "" } else { "\n" };
                    segments.push(MessageSegment::Text {
                        text: format!("{}[引用推文中含有视频，当前不支持解析]", br),
                    });
                    quote_last_was_image = false;
                }
            }

            if let Some(ref date) = quoted_date {
                let br = if quote_last_was_image { "" } else { "\n" };
                segments.push(MessageSegment::Text {
                    text: format!("{}🕒 {}", br, date),
                });
            }
        }
    }

    Some(segments)
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
