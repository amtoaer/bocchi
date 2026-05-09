use std::{sync::LazyLock, time::Duration};

use bocchi::schema::MessageSegment;
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

static AVATAR_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".tweet-avatar img").unwrap());

static QUOTE_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".quote").unwrap());

static QUOTE_TEXT_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".quote-text").unwrap());

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

fn extract_quoted_tweet(tweet_body: &scraper::ElementRef) -> Option<String> {
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
    warn!("匹配到 X 链接: @{}/{}", username, status_id);

    let nitter_url = format!("https://nitter.net/{}/status/{}", username, status_id);
    warn!("请求 Nitter URL: {}", nitter_url);

    let resp = HTTP_CLIENT
        .get(&nitter_url)
        .header("User-Agent", FIREFOX_UA)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header("Accept-Language", "zh-CN,en-US;q=0.9,en;q=0.8")
        .header("Upgrade-Insecure-Requests", "1")
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            warn!("请求 Nitter 失败: {:?}", e);
            return None;
        }
    };

    let status = resp.status();
    warn!("Nitter 响应状态: {}", status);

    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => {
            warn!("读取 Nitter 响应体失败: {:?}", e);
            return None;
        }
    };

    warn!("Nitter 响应体长度: {} 字节", body.len());

    let html = Html::parse_document(&body);

    // 定位主推文
    let tweet_body = match html.select(&MAIN_TWEET_SEL).next() {
        Some(b) => b,
        None => {
            warn!("未在页面中找到主推文 (#m .timeline-item .tweet-body)");
            let preview = &body[..body.len().min(500)];
            warn!("页面内容前 500 字符: {}", preview);
            return None;
        }
    };

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

    warn!("解析到作者: @{} ({:?})", author_username, author_fullname);

    // 提取推文内容
    let content = tweet_body
        .select(&CONTENT_SEL)
        .next()
        .map(|el| clean_text(&el.inner_html()))
        .unwrap_or_default();

    warn!(
        "解析到推文内容 ({} 字): {}",
        content.chars().count(),
        &content[..content.len().min(80)]
    );

    // 提取日期
    let published_date = tweet_body
        .select(&DATE_SEL)
        .next()
        .and_then(|el| el.value().attr("title"))
        .map(|s| s.to_owned());

    warn!("解析到发布日期: {:?}", published_date);

    // 提取统计数据
    let (comments, retweets, likes, views) = parse_tweet_stats(&tweet_body);
    warn!(
        "解析到统计: 评论={}, 转发={}, 点赞={}, 浏览={}",
        comments, retweets, likes, views
    );

    // 提取头像
    let avatar_url = tweet_body
        .select(&AVATAR_SEL)
        .next()
        .and_then(|el| el.value().attr("src"))
        .map(|src| format!("https://nitter.net{}", src));

    warn!("解析到头像 URL: {:?}", avatar_url);

    let mut message_segments = Vec::new();

    // 头像图片
    if let Some(ref avatar) = avatar_url {
        message_segments.push(MessageSegment::Image {
            file: avatar.clone(),
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

    // 检查是否有引用的推文
    if let Some(quoted_text) = extract_quoted_tweet(&tweet_body) {
        warn!("解析到引用推文");
        text.push_str(&quoted_text);
    }

    warn!("最终消息长度: {} 字符", text.chars().count());
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
