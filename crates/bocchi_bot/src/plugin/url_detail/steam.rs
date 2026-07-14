use std::{collections::HashMap, sync::LazyLock, time::Duration};

use anyhow::{Context, Result, bail};
use bocchi::schema::MessageSegment;
use scraper::Html;
use serde::Deserialize;

use super::{RecognizedContent, RecognizedMessage};
use crate::utils::HTTP_CLIENT;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

static STEAM_APP_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"https?://store\.steampowered\.com/(?:agecheck/)?app/(\d+)\b").unwrap());

#[derive(Debug, Clone)]
struct SteamLink {
    url: String,
    app_id: String,
}

#[derive(Deserialize)]
struct AppDetailsResponse {
    success: bool,
    data: Option<AppDetails>,
}

#[derive(Deserialize)]
struct AppDetails {
    name: String,
    header_image: Option<String>,
    short_description: String,
    #[serde(default)]
    developers: Vec<String>,
    #[serde(default)]
    publishers: Vec<String>,
    is_free: bool,
    price_overview: Option<PriceOverview>,
    release_date: Option<ReleaseDate>,
    #[serde(default)]
    genres: Vec<Genre>,
    metacritic: Option<Metacritic>,
}

#[derive(Deserialize)]
struct PriceOverview {
    discount_percent: u8,
    initial_formatted: String,
    final_formatted: String,
}

#[derive(Deserialize)]
struct ReleaseDate {
    coming_soon: bool,
    date: String,
}

#[derive(Deserialize)]
struct Genre {
    description: String,
}

#[derive(Deserialize)]
struct Metacritic {
    score: u8,
}

#[derive(Deserialize)]
struct ReviewResponse {
    success: u8,
    query_summary: Option<ReviewSummary>,
}

#[derive(Deserialize)]
struct ReviewSummary {
    review_score_desc: String,
    total_positive: u64,
    total_reviews: u64,
}

pub(crate) async fn recognizer(text: &str) -> Option<RecognizedMessage> {
    let link = parse_link(text)?;
    Some(RecognizedMessage::new(
        RecognizedContent::Normal(recognize_or_error(&link).await),
        Vec::new(),
    ))
}

fn parse_link(text: &str) -> Option<SteamLink> {
    STEAM_APP_REGEX.captures(text).and_then(|captures| {
        Some(SteamLink {
            url: captures.get(0)?.as_str().to_owned(),
            app_id: captures.get(1)?.as_str().to_owned(),
        })
    })
}

async fn recognize_or_error(link: &SteamLink) -> Vec<MessageSegment> {
    match recognize_one(link).await {
        Ok(segments) => segments,
        Err(error) => {
            warn!("Steam 链接解析失败: {}, {error:#}", link.url);
            vec![MessageSegment::Text {
                text: format!("Steam 链接解析失败：{error:#}"),
            }]
        }
    }
}

async fn recognize_one(link: &SteamLink) -> Result<Vec<MessageSegment>> {
    let url = format!(
        "https://store.steampowered.com/api/appdetails?appids={}&cc=cn&l=schinese",
        link.app_id
    );
    let mut response = HTTP_CLIENT
        .get(url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await?
        .error_for_status()?
        .json::<HashMap<String, AppDetailsResponse>>()
        .await?;
    let response = response.remove(&link.app_id).context("Steam 未返回该应用的信息")?;
    if !response.success {
        bail!("应用不存在、已下架或当前地区不可用");
    }
    let detail = response.data.context("Steam 返回了空的应用信息")?;
    let (all_reviews, schinese_reviews) = tokio::join!(
        fetch_review_summary(&link.app_id, "all"),
        fetch_review_summary(&link.app_id, "schinese"),
    );
    let all_reviews = all_reviews.unwrap_or_else(|error| {
        warn!("获取 Steam 全部评测摘要失败: {}, {error:#}", link.url);
        None
    });
    let schinese_reviews = schinese_reviews.unwrap_or_else(|error| {
        warn!("获取 Steam 简体中文评测摘要失败: {}, {error:#}", link.url);
        None
    });

    let mut segments = Vec::with_capacity(2);
    if let Some(header_image) = detail.header_image {
        segments.push(MessageSegment::Image {
            file: header_image,
            r#type: None,
            url: None,
            cache: Some(true),
            proxy: Some(false),
            timeout: Some(10),
        });
    }

    let description = clean_text(&detail.short_description);
    let mut information = vec![format!("🎮 {}", detail.name)];
    match (detail.developers.is_empty(), detail.publishers.is_empty()) {
        (false, false) if detail.developers == detail.publishers => {
            information.push(format!("🏢 {}", detail.developers.join(", ")));
        }
        (false, false) => {
            information.push(format!("🛠️ {}", detail.developers.join(", ")));
            information.push(format!("🏢 {}", detail.publishers.join(", ")));
        }
        (false, true) => information.push(format!("🛠️ {}", detail.developers.join(", "))),
        (true, false) => information.push(format!("🏢 {}", detail.publishers.join(", "))),
        (true, true) => {}
    }
    if !detail.genres.is_empty() {
        information.push(format!(
            "🏷️ {}",
            detail
                .genres
                .into_iter()
                .map(|genre| genre.description)
                .collect::<Vec<_>>()
                .join(" / ")
        ));
    }

    let mut purchase = vec![format!(
        "💰 {}",
        format_price(detail.is_free, detail.price_overview.as_ref())
    )];
    if let Some(release_date) = detail.release_date {
        let date = if release_date.date.is_empty() {
            "尚未公布".to_owned()
        } else {
            release_date.date
        };
        let prefix = if release_date.coming_soon { "预计 " } else { "" };
        purchase.push(format!("📅 {prefix}{date}"));
    }
    information.push(purchase.join(" ｜ "));

    let mut stats = Vec::new();
    if let Some(metacritic) = detail.metacritic {
        stats.push(format!("⭐ Metacritic {}", metacritic.score));
    }
    if let Some(review) = all_reviews.as_ref().and_then(|review| format_review("🌐", review)) {
        stats.push(review);
    }
    if let Some(review) = schinese_reviews.as_ref().and_then(|review| format_review("🇨🇳", review)) {
        stats.push(review);
    }
    if !stats.is_empty() {
        information.push(stats.join(" ｜ "));
    }
    if !description.is_empty() {
        information.push(format!("📝 {description}"));
    }
    let text = information.join("\n");
    segments.push(MessageSegment::Text { text });
    Ok(segments)
}

async fn fetch_review_summary(app_id: &str, language: &str) -> Result<Option<ReviewSummary>> {
    let url = format!(
        "https://store.steampowered.com/appreviews/{app_id}?json=1&language={language}&purchase_type=all&num_per_page=0&l=schinese"
    );
    let response = HTTP_CLIENT
        .get(url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await?
        .error_for_status()?
        .json::<ReviewResponse>()
        .await?;
    if response.success != 1 {
        bail!("Steam 评价接口返回失败状态");
    }
    Ok(response.query_summary)
}

fn format_review(label: &str, review: &ReviewSummary) -> Option<String> {
    if review.total_reviews == 0 {
        return None;
    }
    let positive_rate = review.total_positive as f64 / review.total_reviews as f64 * 100.0;
    Some(format!(
        "{label} {} · {:.1}%（{} 条）",
        review.review_score_desc,
        positive_rate,
        format_number(review.total_reviews)
    ))
}

fn clean_text(html: &str) -> String {
    Html::parse_fragment(html)
        .root_element()
        .text()
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_owned()
}

fn format_price(is_free: bool, price: Option<&PriceOverview>) -> String {
    if is_free {
        return "免费".to_owned();
    }
    let Some(price) = price else {
        return "暂无价格信息".to_owned();
    };
    if price.discount_percent > 0 && !price.initial_formatted.is_empty() {
        format!(
            "{}（原价 {}，-{}%）",
            price.final_formatted, price.initial_formatted, price.discount_percent
        )
    } else {
        price.final_formatted.clone()
    }
}

fn format_number(number: u64) -> String {
    let raw = number.to_string();
    let mut formatted = String::with_capacity(raw.len() + raw.len() / 3);
    for (index, digit) in raw.chars().enumerate() {
        if index > 0 && (raw.len() - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(digit);
    }
    formatted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_link() {
        let link = parse_link(
            "https://store.steampowered.com/app/620/Portal_2/ 和 https://store.steampowered.com/agecheck/app/730/",
        )
        .unwrap();
        assert_eq!(link.app_id, "620");
        assert_eq!(link.url, "https://store.steampowered.com/app/620");
    }

    #[test]
    fn test_ignore_non_app_links() {
        assert!(parse_link("https://store.steampowered.com/search/?term=Portal").is_none());
        assert!(parse_link("https://steamcommunity.com/app/620").is_none());
    }

    #[test]
    fn test_format_price() {
        let discount = PriceOverview {
            discount_percent: 90,
            initial_formatted: "¥ 42.00".to_owned(),
            final_formatted: "¥ 4.20".to_owned(),
        };
        assert_eq!(format_price(true, None), "免费");
        assert_eq!(format_price(false, None), "暂无价格信息");
        assert_eq!(format_price(false, Some(&discount)), "¥ 4.20（原价 ¥ 42.00，-90%）");
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(31_296), "31,296");
        assert_eq!(format_number(1_234_567), "1,234,567");
    }

    #[test]
    fn test_format_review() {
        let review = ReviewSummary {
            review_score_desc: "特别好评".to_owned(),
            total_positive: 8_650,
            total_reviews: 9_161,
        };
        assert_eq!(
            format_review("🇨🇳", &review).as_deref(),
            Some("🇨🇳 特别好评 · 94.4%（9,161 条）")
        );
    }
}
