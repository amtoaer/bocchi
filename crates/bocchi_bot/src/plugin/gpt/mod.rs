use std::{env, sync::LazyLock};

use anyhow::{Error, Result};
use async_tempfile::TempFile;
use bocchi::{
    chain::{Context, Rule},
    plugin::Plugin,
    schema::{Emoji, MessageContent, MessageSegment},
};
use dashmap::DashMap;
use serde_json::{Value, json};
use tokio::{io::AsyncWriteExt, sync::Mutex};

use crate::{
    migrate::database,
    model::memory::{CachedMessage, v1::Memory},
    utils::HTTP_CLIENT,
};

mod markdown;

static DEEPSEEK_API_KEY: LazyLock<String> = LazyLock::new(|| env::var("DEEPSEEK_API_KEY").unwrap_or_default());
static LOCKS: LazyLock<DashMap<String, Mutex<()>>> = LazyLock::new(DashMap::new);

pub fn gpt_plugin() -> Plugin {
    let mut plugin = Plugin::new("GPT 插件", "使用 DeepSeek API 进行对话");

    for (description, command, max_tokens, reply_image) in [
        ("提问并获得文本答复", "#gpt", Some(512), false), // gpt 使用文本输出，需要文本内容较短
        ("提问并获得图片答复", "#igpt", None, true),      // igpt 使用图片输出，不需要限制 token
    ] {
        plugin.on(
            description,
            i32::default(),
            Rule::on_group_message() & Rule::on_prefix(command),
            move |ctx| async move { call_deepseek_api(ctx, command, max_tokens, reply_image).await },
        )
    }

    for (description, command, lookup_command, reply_image) in [
        ("查询 gpt 历史记录", "#gpt_history", "#gpt", false),
        ("查询 igpt 历史记录", "#igpt_history", "#igpt", true),
    ] {
        plugin.on(
            description,
            i32::default() + 1, // 确保查询历史记录的优先级高于提问
            Rule::on_group_message() & Rule::on_exact_match(command),
            move |ctx| async move { query_gpt_history(ctx, lookup_command, reply_image).await },
        );
    }

    plugin.on(
        "清除 GPT 的消息历史",
        i32::default(),
        Rule::on_group_message() & Rule::on_exact_match("#clear_gpt"),
        move |ctx| async move {
            // 上面 matcher 条件写了 group_message，理论上可以直接拿到 group_id
            // 但为了保证 cache_key 的兼容性，还是使用 try_group_id().ok() 拿到 Option<u64> 使用
            let (user_id, optional_group_id) = (ctx.event.user_id(), ctx.event.try_group_id().ok());
            let rw = database().rw_transaction()?;
            for command in ["#gpt", "#igpt"] {
                let cache_key = format!("{}_{:?}_{}", command, optional_group_id, user_id);
                if let Some(memory) = rw.get().primary::<Memory>(cache_key)? {
                    rw.remove(memory)?;
                }
            }
            rw.commit()?;
            ctx.reply("GPT 消息历史已清除").await?;
            Ok(true)
        },
    );
    plugin
}

async fn call_deepseek_api(
    ctx: Context,
    command: &'static str,
    max_tokens: Option<i32>,
    reply_image: bool,
) -> Result<bool> {
    let text = ctx
        .event
        .plain_text()
        .trim()
        .trim_start_matches(command)
        .trim()
        .to_owned();
    if text.is_empty() {
        return Ok(false);
    }
    ctx.set_reaction(Emoji::敬礼_1).await?;
    let (user_id, optional_group_id) = (ctx.event.user_id(), ctx.event.try_group_id().ok());
    let cache_key = format!("{}_{:?}_{}", command, optional_group_id, user_id);
    let lock = LOCKS.entry(cache_key.clone()).or_default();
    let _guard = lock.lock().await;
    let r = database().r_transaction()?;
    let mut memory = r
        .get()
        .primary::<Memory>(cache_key.as_str())?
        .unwrap_or_else(|| Memory::new(cache_key.clone()));
    drop(r);
    memory.history.push_back(CachedMessage {
        sender: Some(ctx.event.sender().clone()),
        content: text,
    });
    let resp_text = async {
        let mut body = json!({
            "model": "deepseek-chat",
            "messages": memory.history.iter().map(CachedMessage::to_gpt_message).collect::<Vec<_>>(),
            "stream": false
        });
        if let Some(max_tokens) = max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        let resp = HTTP_CLIENT
            .post("https://api.deepseek.com/chat/completions")
            .bearer_auth(DEEPSEEK_API_KEY.as_str())
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        resp["choices"][0]["message"]["content"]
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| Error::msg("Invalid response"))
    }
    .await;
    let (text, emoji, res) = match resp_text {
        Err(e) => ("获取大模型回复失败，请稍后重试".to_string(), Emoji::泪奔_1, Err(e)),
        Ok(resp_text) => (resp_text, Emoji::庆祝_1, Ok(true)),
    };
    memory.history.push_back(CachedMessage {
        sender: None,
        content: text.clone(),
    });
    while memory.history.len() > 10 {
        memory.history.pop_front();
    }
    let rw = database().rw_transaction()?;
    rw.upsert(memory)?;
    rw.commit()?;
    drop(_guard);
    let mut tempfile = TempFile::new().await?;
    let message = if reply_image {
        tempfile.write_all(&markdown::markdown_to_image(text).await?).await?;
        tempfile.flush().await?;
        MessageSegment::Image {
            /*
            当前使用 /tmp 中转来进行 bot 与 onebot server 的文件传输，这要求 bot 与 onebot server 在同一台机器上，
            且如果有容器等隔离环境，需要保证 /tmp 是共享的。更通用的做法是引入 base64 crates 通过 base64 传递图片。
            */
            file: format!("file://{}", tempfile.file_path().to_string_lossy()),
            r#type: None,
            url: None,
            cache: Some(true),
            proxy: None,
            timeout: None,
        }
    } else {
        MessageSegment::Text { text }
    };
    ctx.set_reaction(emoji).await?;
    ctx.reply_content(vec![message]).await?;
    res
}

async fn query_gpt_history(ctx: Context, command: &'static str, reply_image: bool) -> Result<bool> {
    let (user_id, optional_group_id) = (ctx.event.user_id(), ctx.event.try_group_id().ok());
    let cache_key = format!("{}_{:?}_{}", command, optional_group_id, user_id);
    let r = database().r_transaction()?;
    let memory = r.get().primary::<Memory>(cache_key.as_str())?;
    drop(r);
    let command_name = command.trim_start_matches('#').to_uppercase();
    if let Some(memory) = memory {
        if memory.history.is_empty() {
            ctx.reply(format!("没有找到 {} 历史记录", command_name)).await?;
            return Ok(true);
        }
        let mut messages = Vec::new();
        for message in memory.history.iter() {
            match &message.sender {
                Some(sender) => {
                    messages.push(MessageSegment::Node {
                        id: None,
                        user_id: sender.user_id.map(|id| id.to_string()),
                        nickname: sender.nickname.clone(),
                        content: Some(MessageContent::Text(message.content.clone())),
                    });
                }
                None => {
                    let message_user_id = Some(ctx.event.user_id().to_string());
                    if reply_image {
                        let mut tempfile = TempFile::new().await?;
                        tempfile
                            .write_all(&markdown::markdown_to_image(message.content.clone()).await?)
                            .await?;
                        tempfile.flush().await?;
                        messages.push(MessageSegment::Node {
                            id: None,
                            user_id: message_user_id,
                            nickname: Some(command_name.clone()),
                            content: Some(MessageContent::Segment(vec![MessageSegment::Image {
                                file: format!("file://{}", tempfile.file_path().to_string_lossy()),
                                r#type: None,
                                url: None,
                                cache: Some(true),
                                proxy: None,
                                timeout: None,
                            }])),
                        });
                    } else {
                        messages.push(MessageSegment::Node {
                            id: None,
                            user_id: message_user_id,
                            nickname: Some(command_name.clone()),
                            content: Some(MessageContent::Text(message.content.clone())),
                        });
                    }
                }
            }
        }
        ctx.send_forward_segment(messages).await?;
    } else {
        ctx.reply(format!("没有找到 {} 历史记录", command_name)).await?;
    }
    Ok(true)
}
