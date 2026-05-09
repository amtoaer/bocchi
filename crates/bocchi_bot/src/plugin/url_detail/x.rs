use std::{sync::LazyLock, time::Duration};

use bocchi::schema::MessageSegment;
use reqwest::header;
use scraper::{Html, Selector};

use crate::utils::HTTP_CLIENT;

static X_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"https?://(?:www\.)?x\.com/(\w+)/status/(\d+)").unwrap());

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

static GALLERY_VIDEO_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".attachments .gallery-video").unwrap());

static QUOTE_MEDIA_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".quote-media-container").unwrap());

// ---- 辅助函数 ----

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
fn extract_images(scope: &scraper::ElementRef) -> Vec<String> {
    scope
        .select(&ATTACHMENT_IMG_SEL)
        .filter_map(|el| el.value().attr("href"))
        .map(|href| format!("https://nitter.net{}", href))
        .collect()
}

/// 检测指定元素范围内是否存在视频
fn has_video(scope: &scraper::ElementRef) -> bool {
    scope.select(&GALLERY_VIDEO_SEL).next().is_some()
}

fn extract_quoted_tweet(
    tweet_body: &scraper::ElementRef,
    message_segments: &mut Vec<MessageSegment>,
) -> Option<String> {
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
        .map(|s| s.to_owned());

    // 引用推文中的图片
    if let Some(media_container) = quote_el.select(&QUOTE_MEDIA_SEL).next() {
        for img_url in extract_images(&media_container) {
            message_segments.push(MessageSegment::Image {
                file: img_url,
                r#type: None,
                url: None,
                cache: Some(true),
                proxy: Some(false),
                timeout: Some(10),
            });
        }
        // 引用推文中的视频
        if has_video(&media_container) {
            message_segments.push(MessageSegment::Text {
                text: "[引用推文中含有视频，当前不支持解析]".to_owned(),
            });
        }
    }

    match (quoted_username, quoted_content) {
        (Some(username), Some(content)) => {
            let mut text = format!("\n---\n引用了 @{}", username);
            if let Some(ref fullname) = quoted_fullname {
                if !fullname.is_empty() {
                    text.push_str(&format!(" ({})", fullname));
                }
            }
            text.push_str(&format!(":\n{}", content));
            if let Some(ref date) = quoted_date {
                text.push_str(&format!("\n🕒 {}", date));
            }
            Some(text)
        }
        _ => None,
    }
}

// ---- 入口 ----

pub(crate) async fn recognizer(text: &str) -> Option<Vec<MessageSegment>> {
    let caps = X_REGEX.captures(text)?;
    let username = caps.get(1)?.as_str();
    let status_id = caps.get(2)?.as_str();

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
        .map(|s| s.to_owned());

    // 提取统计数据
    let (comments, retweets, likes, views) = parse_tweet_stats(&tweet_body);

    let mut message_segments = Vec::new();

    // 主推文图片
    for img_url in extract_images(&tweet_body) {
        message_segments.push(MessageSegment::Image {
            file: img_url,
            r#type: None,
            url: None,
            cache: Some(true),
            proxy: Some(false),
            timeout: Some(10),
        });
    }

    // 构建正文
    let mut text = format!("@{}", author_username);
    if let Some(ref fullname) = author_fullname {
        if !fullname.is_empty() && *fullname != author_username {
            text.push_str(&format!(" ({})", fullname));
        }
    }
    text.push_str(&format!(":\n{}", content));

    if let Some(ref date) = published_date {
        text.push_str(&format!("\n🕒 {}", date));
    }

    text.push_str(&format!(" | 💬 {} 🔄 {} ❤️ {} 👁 {}", comments, retweets, likes, views));

    // 主推文视频提示
    if has_video(&tweet_body) {
        text.push_str("\n[推文中含有视频，当前不支持解析]");
    }

    // 检查是否有引用的推文
    if let Some(quoted_text) = extract_quoted_tweet(&tweet_body, &mut message_segments) {
        text.push_str(&quoted_text);
    }

    message_segments.push(MessageSegment::Text { text });

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
