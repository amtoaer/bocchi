use std::{sync::LazyLock, time::Duration};

use bocchi::schema::{MessageContent, MessageSegment};
use serde::Deserialize;

use crate::utils::HTTP_CLIENT;

static BILIBILI_AV_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"https?://(?:www\.)?bilibili\.com/video/av(\d+)").unwrap());
static BILIBILI_BV_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"https?://(?:www\.)?bilibili\.com/video/(BV[a-zA-Z0-9_-]{10})").unwrap());
static BILIBILI_B23_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(https?://(?:www\.)?b23\.tv/[a-zA-Z0-9_-]{7})").unwrap());

enum VideoID {
    AV(String),
    BV(String),
}

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

fn parse_raw_video_id(text: &str) -> Option<VideoID> {
    if let Some(caps) = BILIBILI_AV_REGEX.captures(text) {
        Some(VideoID::AV(caps.get(1)?.as_str().to_string()))
    } else if let Some(caps) = BILIBILI_BV_REGEX.captures(text) {
        Some(VideoID::BV(caps.get(1)?.as_str().to_string()))
    } else {
        None
    }
}

async fn parse_video_id(text: &str) -> Option<VideoID> {
    // 尝试直接从文本解析
    let res = parse_raw_video_id(text);
    if res.is_some() {
        return res;
    }
    let caps = BILIBILI_B23_REGEX.captures(text)?;
    let url = caps.get(1)?.as_str();
    // FIXME: 此处的需求只是获取 302 后的 URL，但由于 reqwest 会自动重定向，这里会多一次最终的 200 请求
    // 可以在 ClientBuilder 中设置 redirect_policy 为 none 来禁用自动重定向，但是这样会导致其它地方的重定向也失效
    // 暂时先这样处理，后续如果 https://github.com/seanmonstar/reqwest/pull/2440 被合并，可以单独为此处请求设置不重定向然后取 Location Header
    let resp = HTTP_CLIENT
        .get(url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    let url_after_redirect = resp.url().as_str();
    parse_raw_video_id(url_after_redirect)
}

pub(crate) async fn recognizer(text: &str, message_id: i32) -> Option<MessageContent> {
    let video_id = parse_video_id(text).await?;
    let url = match video_id {
        VideoID::AV(id) => format!("https://api.bilibili.com/x/web-interface/view?aid={}", id),
        VideoID::BV(id) => format!("https://api.bilibili.com/x/web-interface/view?bvid={}", id),
    };
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
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_regex() {
        let text = "https://www.bilibili.com/video/BV12T2mYiEyy/";
        let caps = BILIBILI_BV_REGEX.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "BV12T2mYiEyy");

        let text = "https://www.bilibili.com/video/av113385693775046?p=1";
        let caps = BILIBILI_AV_REGEX.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "113385693775046");

        let text = "https://www.bilibili.com/video/BV12T2mYiEyya/";
        let caps = BILIBILI_BV_REGEX.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "BV12T2mYiEyy");

        let text = "https://www.bilibili.com/video/BV12T2mYiEy";
        assert!(BILIBILI_BV_REGEX.captures(text).is_none());

        let text = "http://bilibili.com/video/BV12T2mYiEyy/";
        let caps = BILIBILI_BV_REGEX.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "BV12T2mYiEyy");

        let text = "https://b23.tv/8iZPsI3";
        let caps = BILIBILI_B23_REGEX.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "https://b23.tv/8iZPsI3");
    }
}
