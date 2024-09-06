use std::{env, sync::LazyLock};

use anyhow::{Error, Result};
use async_tempfile::TempFile;
use bocchi::{
    adapter::Caller,
    chain::Rule,
    plugin::Plugin,
    schema::{Emoji, Event, MessageContent, MessageSegment, SendMsgParams, SetMsgEmojiLikeParams},
};
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;

use crate::utils::HTTP_CLIENT;
mod markdown;

static DEEPSEEK_API_KEY: LazyLock<String> = LazyLock::new(|| env::var("DEEPSEEK_API_KEY").unwrap_or_default());

pub fn gpt_plugin() -> Plugin {
    let mut plugin = Plugin::new();
    for (command, max_tokens, reply_image) in [
        ("#gpt", Some(512), false), // gpt 使用文本输出，需要文本内容较短
        ("#igpt", None, true),      // igpt 使用图片输出，不需要限制 token
    ] {
        plugin.on(
            Rule::on_group_message() & Rule::on_prefix(command),
            move |caller, event| Box::pin(call_deepseek_api(caller, event, command, max_tokens, reply_image)),
        )
    }
    plugin
}

async fn call_deepseek_api(
    caller: &dyn Caller,
    event: &Event,
    command: &'static str,
    max_tokens: Option<i32>,
    reply_image: bool,
) -> Result<()> {
    let text = event.plain_text().trim().trim_start_matches(command).trim().to_owned();
    if text.is_empty() {
        return Ok(());
    }
    caller
        .set_msg_emoji_like(SetMsgEmojiLikeParams {
            message_id: event.message_id(),
            emoji_id: Emoji::敬礼_1.id(),
        })
        .await?;
    let resp_text = async {
        let mut body = json!({
            "model": "deepseek-chat",
            "messages": [
                {"role": "user", "content": text}
            ],
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
        Ok(resp_text) => (resp_text, Emoji::庆祝_1, Ok(())),
    };
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
    caller
        .set_msg_emoji_like(SetMsgEmojiLikeParams {
            message_id: event.message_id(),
            emoji_id: emoji.id(),
        })
        .await?;
    caller
        .send_msg(SendMsgParams {
            message_type: None,
            user_id: Some(event.user_id()),
            group_id: event.group_id(),
            message: MessageContent::Segment(vec![
                MessageSegment::Reply {
                    id: event.message_id().to_string(),
                },
                message,
            ]),
            auto_escape: true,
        })
        .await?;
    assert!(tempfile.file_path().is_file());
    res
}
