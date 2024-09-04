use crate::utils::HTTP_CLIENT;
use anyhow::{Error, Result};
use bocchi::{
    adapter::Caller,
    chain::Rule,
    plugin::Plugin,
    schema::{Emoji, Event, MessageContent, MessageSegment, SendMsgParams, SetMsgEmojiLikeParams},
};
use serde_json::{json, Value};
use std::{env, sync::LazyLock};

const MAX_TOKENS: i32 = 1024;

const CHAT_PROMPT: &str = "Chat with me!";
const CODE_PROMPT: &str = "Code with me!";

static DEEPSEEK_API_KEY: LazyLock<String> =
    LazyLock::new(|| env::var("DEEPSEEK_API_KEY").unwrap_or_default());

pub fn gpt_plugin() -> Plugin {
    let mut plugin = Plugin::new();
    for (command, prompt, model_name) in &[
        ("chat", CHAT_PROMPT, "deepseek-chat"),
        ("code", CODE_PROMPT, "deepseek-coder"),
    ] {
        plugin.on(
            Rule::on_group_message() & Rule::on_prefix(command),
            |caller, event| {
                Box::pin(call_deepseek_api(
                    caller, event, command, prompt, model_name,
                ))
            },
        )
    }
    plugin
}

#[allow(unused)]
async fn call_deepseek_api(
    caller: &dyn Caller,
    event: &Event,
    command: &'static str,
    prompt: &'static str,
    model_name: &'static str,
) -> Result<()> {
    let text = event
        .plain_text()
        .trim_start_matches(command)
        .trim()
        .to_owned();
    if text.is_empty() {
        return Ok(());
    }
    caller
        .set_msg_emoji_like(SetMsgEmojiLikeParams {
            message_id: event.message_id(),
            emoji_id: Emoji::闪光_2.id(),
        })
        .await?;
    let resp_text = async {
        let resp = HTTP_CLIENT
            .post("https://api.deepseek.com/chat/completions")
            .bearer_auth(DEEPSEEK_API_KEY.as_str())
            .json(&json!({
                "model": model_name,
                "messages": [
                    // {"role": "system", "content":prompt},
                    {"role": "user", "content": text}
                ],
                "max_tokens": MAX_TOKENS,
                "stream": false
            }))
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
        Err(e) => (
            "获取大模型回复失败，请稍后重试".to_string(),
            Emoji::大哭_2,
            Err(e),
        ),
        Ok(resp_text) => (resp_text, Emoji::庆祝_2, Ok(())),
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
                MessageSegment::Text { text },
            ]),
            auto_escape: true,
        })
        .await?;
    res
}
