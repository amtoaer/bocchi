#![deny(unsafe_code)]

#[macro_use]
extern crate tracing;

mod migrate;
mod model;
mod plugin;
mod runtime;
mod utils;

use crate::runtime::RUNTIME;
use anyhow::Result;
use bocchi::bot::Bot;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    RUNTIME.block_on(async {
        let mut bot_instance = Bot::connect("ws://192.168.1.250:3001").await?;
        bot_instance.register_plugin(plugin::daily_bonus_plugin());
        bot_instance.register_plugin(plugin::echo_plugin());
        bot_instance.register_plugin(plugin::gpt_plugin());
        bot_instance.register_plugin(plugin::repeat_plugin());
        bot_instance.start().await
    })
}
