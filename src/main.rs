#![allow(dead_code)]
#![feature(async_closure)]

#[macro_use]
extern crate tracing;

use core::time;

use anyhow::Result;
use futures_util::{future, FutureExt};
use std::future::Future;

use crate::matcher::Rule;

mod bot;
mod caller;
mod connector;
mod error;
mod matcher;
mod schema;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut bot_instance = bot::Bot::connect("ws://192.168.1.250:3001").await?;
    bot_instance.on(
        Rule::on_group_message().into(),
        // Box::new(|x| {
        //     return Box::pin(tokio::time::sleep(1 * time::Duration::from_secs(1)));
        // }),
    );
    bot_instance.start().await?;
    Ok(())
}
