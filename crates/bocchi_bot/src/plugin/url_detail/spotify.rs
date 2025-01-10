use std::{sync::LazyLock, time::Duration};

use anyhow::Result;
use bocchi::schema::{MessageContent, MessageSegment};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::utils::HTTP_CLIENT;

static SPOTIFY_MUSIC_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new("https?://open.spotify.com/track/([a-zA-Z0-9]+)").unwrap());

static SPOTIFY_CLIENT_ID: LazyLock<String> = LazyLock::new(|| std::env::var("SPOTIFY_CLIENT_ID").unwrap_or_default());
static SPOTIFY_CLIENT_SECRET: LazyLock<String> =
    LazyLock::new(|| std::env::var("SPOTIFY_CLIENT_SECRET").unwrap_or_default());

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

struct Token {
    access_token: String,
    expire_time: chrono::DateTime<chrono::Local>,
}

async fn get_spotify_token() -> Result<String> {
    static SPOTIFY_TOKEN: RwLock<Option<Token>> = RwLock::const_new(None);
    let read_guard = SPOTIFY_TOKEN.read().await;
    if let Some(token) = read_guard.as_ref() {
        if token.expire_time > chrono::Local::now() {
            return Ok(token.access_token.to_owned());
        }
    }
    drop(read_guard);
    let mut write_guard = SPOTIFY_TOKEN.write().await;
    if let Some(token) = write_guard.as_ref() {
        if token.expire_time > chrono::Local::now() {
            return Ok(token.access_token.to_owned());
        }
    }
    let current_time = chrono::Local::now();
    let resp = HTTP_CLIENT
        .post("https://accounts.spotify.com/api/token")
        .timeout(Duration::from_secs(10))
        .form(&[("grant_type", "client_credentials")])
        .basic_auth(SPOTIFY_CLIENT_ID.as_str(), Some(SPOTIFY_CLIENT_SECRET.as_str()))
        .send()
        .await?
        .json::<TokenResponse>()
        .await?;
    *write_guard = Some(Token {
        access_token: resp.access_token.clone(),
        // 此处将过期时间提前 20 秒，留一点冗余
        expire_time: current_time + chrono::Duration::seconds(resp.expires_in as i64 - 20),
    });
    Ok(resp.access_token)
}

#[derive(Deserialize)]
struct TrackDetail {
    name: String,
    album: Album,
}

#[derive(Deserialize)]
struct Album {
    name: String,
    release_date: String,
    release_date_precision: String,
    artists: Vec<Artist>,
    images: Vec<Image>,
}

#[derive(Deserialize)]
struct Artist {
    name: String,
}

#[derive(Deserialize)]
struct Image {
    url: String,
}

impl Album {
    fn get_image(&self) -> Option<&str> {
        // 接口返回的图片按从大到小的顺序排列，尽量取中间大小的，避免太大或太小
        self.images.get(self.images.len() / 2).map(|i| i.url.as_str())
    }

    fn get_artists(&self) -> String {
        self.artists
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<&str>>()
            .join(", ")
    }

    fn get_release_date(&self) -> chrono::DateTime<chrono::Local> {
        let time = match self.release_date_precision.as_str() {
            "year" => format!("{}-01-01T00:00:00Z", self.release_date),
            "month" => format!("{}-01T00:00:00Z", self.release_date),
            "day" => format!("{}T00:00:00Z", self.release_date),
            _ => unreachable!(),
        };
        chrono::DateTime::parse_from_rfc3339(&time)
            .unwrap()
            .with_timezone(&chrono::Local)
    }
}

fn parse_track_id(text: &str) -> Option<&str> {
    SPOTIFY_MUSIC_REGEX
        .captures(text)
        .and_then(|cap| cap.get(1).map(|f| f.as_str()))
}
pub(crate) async fn recognizer(text: &str, message_id: i32) -> Option<MessageContent> {
    let track_id = parse_track_id(text)?;
    let url = format!("https://api.spotify.com/v1/tracks/{}", track_id);
    let token = match get_spotify_token().await {
        Ok(token) => token,
        Err(e) => {
            error!("获取 spotify token 失败: {:?}", e);
            return None;
        }
    };
    let resp = HTTP_CLIENT
        .get(&url)
        .timeout(Duration::from_secs(10))
        .bearer_auth(token)
        .send()
        .await
        .ok()?
        .json::<TrackDetail>()
        .await
        .ok()?;
    let mut message_segment = vec![MessageSegment::Reply {
        id: message_id.to_string(),
    }];
    if let Some(image_url) = resp.album.get_image() {
        message_segment.push(MessageSegment::Image {
            file: image_url.to_owned(),
            r#type: None,
            url: None,
            cache: Some(true),
            proxy: Some(false),
            timeout: Some(10),
        });
    }
    message_segment.push(MessageSegment::Text {
        text: format!(
            "歌曲：{}\n专辑：{}\n艺术家：{}\n发行时间：{}",
            resp.name,
            resp.album.name,
            resp.album.get_artists(),
            resp.album.get_release_date()
        ),
    });
    Some(MessageContent::Segment(message_segment))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_regex() {
        let testcases = [
            (
                "https://open.spotify.com/track/2kLuVbzpjnRAOwaTnp7C2J?si=51b81a04a41441a8",
                Some("2kLuVbzpjnRAOwaTnp7C2J"),
            ),
            (
                "https://open.spotify.com/artist/1cXxia1Q2VDTjWe8X2Jydm?si=WSbwhAK9TBek6C0AL5GEhg",
                None,
            ),
            (
                "https://open.spotify.com/album/66OrMbPN4equuw2hHbjA1X?si=G2cGb_1BQIGOYr1Y3SodrA",
                None,
            ),
        ];
        for (text, expected) in testcases.iter() {
            assert_eq!(parse_track_id(text), *expected);
        }
    }
}
