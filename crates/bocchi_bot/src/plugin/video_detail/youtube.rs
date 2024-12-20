use std::{sync::LazyLock, time::Duration};

use bocchi::schema::{MessageContent, MessageSegment};
use serde::Deserialize;

use crate::utils::HTTP_CLIENT;

static YOUTUBE_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"https?://(?:www\.)?(?:youtube\.com/(?:watch\?v=|embed/)|youtu\.be/)([a-zA-Z0-9_-]{11})")
        .unwrap()
});
static YOUTUBE_API_KEY: LazyLock<String> = LazyLock::new(|| std::env::var("YOUTUBE_API_KEY").unwrap_or_default());

#[derive(Deserialize)]
struct VideoDetail {
    title: String,
    #[serde(rename = "channelTitle")]
    channel_title: String,
    thumbnails: Thumbnails,
    #[serde(rename = "publishedAt")]
    published_at: chrono::DateTime<chrono::Local>,
}

#[derive(Deserialize)]
struct Thumbnail {
    url: String,
}

#[derive(Deserialize)]
struct Thumbnails {
    default: Option<Thumbnail>,
    medium: Option<Thumbnail>,
    high: Option<Thumbnail>,
    standard: Option<Thumbnail>,
    maxres: Option<Thumbnail>,
}

impl Thumbnails {
    fn get(&self) -> Option<&str> {
        Some(
            self.maxres
                .as_ref()
                .or(self.standard.as_ref())
                .or(self.high.as_ref())
                .or(self.medium.as_ref())
                .or(self.default.as_ref())?
                .url
                .as_str(),
        )
    }
}

pub(crate) async fn recognizer(text: &str, message_id: i32) -> Option<MessageContent> {
    let caps = YOUTUBE_REGEX.captures(text)?;
    let video_id = caps.get(1)?.as_str();
    let url = format!(
        "https://www.googleapis.com/youtube/v3/videos?part=snippet&id={}&key={}",
        video_id,
        YOUTUBE_API_KEY.as_str()
    );
    let resp = HTTP_CLIENT
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()?;
    let first_item = resp.get("items")?.as_array()?.first()?.get("snippet")?.clone();
    let video_detail: VideoDetail = serde_json::from_value(first_item).ok()?;
    let mut message_segment = vec![MessageSegment::Reply {
        id: message_id.to_string(),
    }];
    if let Some(thumbnail_url) = video_detail.thumbnails.get() {
        message_segment.push(MessageSegment::Image {
            file: thumbnail_url.to_owned(),
            r#type: None,
            url: None,
            cache: Some(true),
            proxy: Some(false),
            timeout: Some(10),
        });
    }
    message_segment.push(MessageSegment::Text {
        text: format!(
            "标题：{}\n作者：{}\n发布时间：{}",
            video_detail.title, video_detail.channel_title, video_detail.published_at
        ),
    });
    Some(MessageContent::Segment(message_segment))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_regex() {
        let text = "https://www.youtube.com/watch?v=6g4dkBF5anU";
        let caps = YOUTUBE_REGEX.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "6g4dkBF5anU");

        let text = "https://www.youtube.com/watch?v=6g4dkBF5anUabcdefg";
        let caps = YOUTUBE_REGEX.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "6g4dkBF5anU");

        let text = "https://www.youtube.com/watch?v=6g4dkBF5an";
        assert!(YOUTUBE_REGEX.captures(text).is_none());

        let text = "https://youtu.be/6g4dkBF5anU";
        let caps = YOUTUBE_REGEX.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "6g4dkBF5anU");

        let text = "https://www.youtube.com/embed/6g4dkBF5anU";
        let caps = YOUTUBE_REGEX.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "6g4dkBF5anU");
    }
}
