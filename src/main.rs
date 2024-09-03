#![allow(dead_code)]
#[macro_use]
extern crate tracing;

use anyhow::Result;

use crate::{
    chain::Rule,
    schema::{MessageContent, SendMsgParams},
};

mod adapter;
mod bot;
mod caller;
mod chain;
mod error;
mod schema;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut bot_instance = bot::Bot::connect("ws://192.168.1.250:3001").await?;
    bot_instance.on(
        Rule::on_prefix("/echo").into(),
        Box::new(|caller, event| {
            Box::pin(async move {
                let raw = event.message().raw();
                let msg =
                    MessageContent::Text(raw.strip_prefix("/echo").unwrap_or(&raw).to_owned());
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
    bot_instance.start().await
}
