use std::{path::PathBuf, sync::LazyLock};

use bocchi::{
    chain::Rule,
    plugin::Plugin,
    schema::{MessageContent, MessageSegment, SendMsgParams},
};
use futures::StreamExt;
use rand::seq::IteratorRandom;
use tokio::fs;
use tokio_stream::wrappers::ReadDirStream;

static FOOD_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from("/home/amtoaer/Documents/code/rust/bocchi/crates/bocchi_bot/src/plugin/what_to_eat/foods/")
});

pub fn what_to_eat_plugin() -> Plugin {
    let mut plugin = Plugin::new("随机食物插件", "想想今天吃什么？");

    plugin.on(
        "随机推荐食物",
        i32::default(),
        Rule::on_message() & Rule::on_exact_match("#wte"),
        |ctx| async move {
            let res = async {
                let foods = ReadDirStream::new(fs::read_dir((*FOOD_DIR).as_path()).await?);
                let foods = foods
                    .filter_map(|entry| async {
                        match entry {
                            Ok(entry) => {
                                let file_name = entry.file_name();
                                let file_name = file_name.to_string_lossy();
                                if [".jpg", ".png"].iter().any(|ext| file_name.ends_with(ext)) {
                                    Some((file_name.to_string(), entry.path()))
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    })
                    .collect::<Vec<_>>()
                    .await;
                let food = foods
                    .into_iter()
                    .choose(&mut rand::thread_rng())
                    .ok_or_else(|| anyhow::anyhow!("没有食物"))?;
                Ok::<_, anyhow::Error>((
                    // 前面确保了以 .jpg 或 .png 结尾，这里 unwrap 是安全的
                    food.0.rsplit_once('.').map(|(name, _)| name).unwrap().to_owned(),
                    fs::read(food.1).await?,
                ))
            }
            .await;
            let msg = match res {
                Err(_) => MessageContent::Segment(vec![
                    MessageSegment::Reply {
                        id: ctx.event.message_id().to_string(),
                    },
                    MessageSegment::Text {
                        text: "出错啦，请稍后再试".to_string(),
                    },
                ]),
                Ok((food_name, image_content)) => MessageContent::Segment(vec![
                    MessageSegment::Reply {
                        id: ctx.event.message_id().to_string(),
                    },
                    MessageSegment::Text {
                        text: format!("今天吃{food_name}！"),
                    },
                    MessageSegment::Image {
                        file: format!("base64://{}", base64_simd::STANDARD.encode_to_string(image_content)),
                        r#type: None,
                        url: None,
                        cache: Some(true),
                        proxy: None,
                        timeout: None,
                    },
                ]),
            };
            ctx.caller
                .send_msg(SendMsgParams {
                    user_id: ctx.event.try_user_id().ok(),
                    group_id: ctx.event.try_group_id().ok(),
                    message: msg,
                    auto_escape: true,
                    message_type: None,
                })
                .await?;
            Ok(true)
        },
    );

    plugin
}
