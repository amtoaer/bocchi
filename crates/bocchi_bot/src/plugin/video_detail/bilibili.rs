use std::{sync::LazyLock, time::Duration};

use bocchi::schema::{MessageContent, MessageSegment};
use serde::Deserialize;

use crate::{plugin::video_detail::AsyncMaybeMsg, utils::HTTP_CLIENT};

static BILIBILI_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"https?://(?:www\.)?bilibili\.com/video/([a-zA-Z0-9_-]{12})").unwrap());

#[derive(Deserialize)]
struct VideoDetail {
    title: String,
    pic: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pubdate: chrono::DateTime<chrono::Utc>,
    owner: Owner,
}

#[derive(Deserialize)]
struct Owner {
    name: String,
}

pub(crate) fn recognizer(text: String, message_id: i32) -> AsyncMaybeMsg {
    Box::pin(async move {
        let caps = BILIBILI_REGEX.captures(&text)?;
        let video_id = caps.get(1)?.as_str();
        let url = format!("https://api.bilibili.com/x/web-interface/view?bvid={}", video_id);
        let resp = HTTP_CLIENT
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .ok()?
            .json::<serde_json::Value>()
            .await
            .ok()?;
        let data = resp.get("data")?.clone();
        let video_detail: VideoDetail = serde_json::from_value(data).ok()?;
        let message_segment = vec![
            MessageSegment::Reply {
                id: message_id.to_string(),
            },
            MessageSegment::Image {
                file: video_detail.pic,
                r#type: None,
                url: None,
                cache: Some(true),
                proxy: Some(false),
                timeout: Some(10),
            },
            MessageSegment::Text {
                text: format!(
                    "标题：{}\n作者：{}\n发布时间：{}",
                    video_detail.title,
                    video_detail.owner.name,
                    video_detail.pubdate.with_timezone(&chrono::Local)
                ),
            },
        ];
        Some(MessageContent::Segment(message_segment))
    })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_regex() {
        let text = "https://www.bilibili.com/video/BV12T2mYiEyy/";
        let caps = BILIBILI_REGEX.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "BV12T2mYiEyy");

        let text = "https://www.bilibili.com/video/BV12T2mYiEyya/";
        let caps = BILIBILI_REGEX.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "BV12T2mYiEyy");

        let text = "https://www.bilibili.com/video/BV12T2mYiEy";
        assert!(BILIBILI_REGEX.captures(text).is_none());

        let text = "http://bilibili.com/video/BV12T2mYiEyy/";
        let caps = BILIBILI_REGEX.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "BV12T2mYiEyy");
    }
}
