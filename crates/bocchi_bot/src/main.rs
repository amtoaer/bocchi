#[macro_use]
extern crate tracing;

mod migrate;
mod model;
mod plugin;
mod utils;

use anyhow::Result;

use bocchi::bot::Bot;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut bot_instance = Bot::connect("ws://192.168.1.250:3001").await?;
    bot_instance.register_plugin(plugin::check_in_plugin());
    bot_instance.register_plugin(plugin::echo_plugin());
    bot_instance.register_plugin(plugin::gpt_plugin());
    bot_instance.start().await
}
