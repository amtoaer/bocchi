#![deny(unsafe_code)]

#[macro_use]
extern crate tracing;

mod migrate;
mod model;
mod plugin;
mod utils;

use anyhow::Result;
use bocchi::bot::Bot;

fn init() {
    // 初始化日志
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
        .init();
    // 初始化设置 rustls 使用的全局加密库（https://docs.rs/rustls/latest/rustls/crypto/struct.CryptoProvider.html#using-the-per-process-default-cryptoprovider）
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("设置默认 Crypto Provider 失败");
}

#[tokio::main]
async fn main() -> Result<()> {
    init();
    let mut bot_instance = Bot::connect("ws://localhost:3001").await?;
    bot_instance.use_builtin_handler();
    bot_instance.register_plugin(plugin::bonus_plugin());
    bot_instance.register_plugin(plugin::echo_plugin());
    bot_instance.register_plugin(plugin::gpt_plugin());
    bot_instance.register_plugin(plugin::repeat_plugin());
    bot_instance.register_plugin(plugin::hacker_news_plugin());
    bot_instance.register_plugin(plugin::what_to_eat_plugin());
    bot_instance.register_plugin(plugin::url_detail_plugin());
    bot_instance.register_plugin(plugin::select_plugin());
    bot_instance.start().await
}
