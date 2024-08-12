#![allow(dead_code)]

#[macro_use]
extern crate tracing;

use anyhow::Result;

use crate::schema::{MessageType, SendPrivateMsgParams};

mod bot;
mod caller;
mod connector;
mod error;
mod matcher;
mod schema;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let _ = bot::Bot::run("ws://192.168.1.250:3001").await?;
    tokio::signal::ctrl_c().await?;
    Ok(())
}
