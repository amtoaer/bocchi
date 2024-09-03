#![allow(dead_code)]
#![feature(async_fn_traits)]
#[macro_use]
extern crate tracing;

use anyhow::Result;

use crate::{
    caller::send_private_msg,
    chain::Rule,
    schema::{MessageContent, SendPrivateMsgParams},
};

mod bot;
mod caller;
mod chain;
mod connector;
mod error;
mod schema;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut bot_instance = bot::Bot::connect("ws://192.168.1.250:3001").await?;
    bot_instance.on(
        Rule::on_group_message() & Rule::on_sender_id(1361974998),
        Box::new(|caller, event| {
            Box::pin(async move {
                println!("{:?}", event);
                send_private_msg(
                    caller,
                    SendPrivateMsgParams {
                        user_id: 1361974998,
                        message: MessageContent::Text("I received it!".to_string()),
                        auto_escape: true,
                    },
                )
                .await?;
                Ok(())
            })
        }),
    );
    bot_instance.start().await
}
