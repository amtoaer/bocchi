#[macro_use]
extern crate tracing;

mod migrate;
mod model;
mod plugin;
mod utils;

use std::borrow::Cow;

use anyhow::Result;

use bocchi::{
    bot::Bot,
    chain::Rule,
    schema::{MessageContent, MessageSegment, SendMsgParams},
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut bot_instance = Bot::connect("ws://192.168.1.250:3001").await?;
    bot_instance.on(
        Rule::on_message() & Rule::on_prefix("/echo"),
        Box::new(|caller, event| {
            Box::pin(async move {
                let raw = raw
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<&str>>()
                    .join("");
                let raw = raw.as_str();
                let msg = raw.strip_prefix("/echo").unwrap_or(raw).trim().to_owned();
                if msg.is_empty() {
                    return Ok(());
                }
                let msg = MessageContent::Text(msg);
                let params = match event.group_id() {
                    Some(group_id) => SendMsgParams {
                        user_id: None,
                        group_id: Some(group_id),
                        message: msg,
                        auto_escape: true,
                        message_type: None,
                    },
                    None => SendMsgParams {
                        user_id: Some(event.user_id()),
                        group_id: None,
                        message: msg,
                        auto_escape: true,
                        message_type: None,
                    },
                };
                caller.send_msg(params).await?;
                Ok(())
            })
        }),
    );

    bot_instance.register_plugin(plugin::signin_plugin());
    bot_instance.start().await
}
