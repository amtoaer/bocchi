use std::fmt::Display;

use bocchi::{chain::Rule, plugin::Plugin};
use futures::{stream::FuturesOrdered, StreamExt};
use serde::Deserialize;

use crate::utils::HTTP_CLIENT;

#[derive(Deserialize)]
struct HackerStory {
    id: i64,
    title: String,
    url: String,
}

impl Display for HackerStory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "标题：{}\n链接：{}\n评论：https://news.ycombinator.com/item?id={}",
            self.title, self.url, self.id,
        )
    }
}

pub fn hacker_news_plugin() -> Plugin {
    let mut plugin = Plugin::new("Hacker News 插件", "获取 Hacker News 的内容");

    plugin.on(
        "输出 Hacker News top 10",
        i32::default(),
        Rule::on_message() & Rule::on_exact_match("#hn"),
        |ctx| async move {
            let ids = HTTP_CLIENT
                .get("https://hacker-news.firebaseio.com/v0/topstories.json")
                .send()
                .await?
                .error_for_status()?
                .json::<Vec<i64>>()
                .await?;
            let future_ordered = ids
                .into_iter()
                .take(10)
                .map(|id| async move {
                    HTTP_CLIENT
                        .get(format!("https://hacker-news.firebaseio.com/v0/item/{}.json", id))
                        .send()
                        .await?
                        .error_for_status()?
                        .json::<HackerStory>()
                        .await
                })
                .collect::<FuturesOrdered<_>>();
            let res = future_ordered
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .filter_map(|story| story.ok().map(|story| story.to_string()))
                .collect::<Vec<_>>();
            ctx.send_forward(res).await?;
            Ok(true)
        },
    );

    plugin
}
