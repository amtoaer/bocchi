use std::{net::IpAddr, time::Duration};

use anyhow::{Context, Result};
use bocchi::{chain::Rule, plugin::Plugin};

use crate::utils::HTTP_CLIENT;

const IP_API_URL: &str = "https://myip.ipip.net/s";

pub fn ip_plugin() -> Plugin {
    let mut plugin = Plugin::new("IP 插件", "获取服务器公网 IP");

    plugin.on(
        "获取服务器公网 IP",
        i32::default(),
        Rule::on_group_id(954985908) & Rule::on_exact_match("#ip"),
        |ctx| async move {
            let response = match public_ip().await {
                Ok(ip) => ip.to_string(),
                Err(error) => format!("获取公网 IP 失败: {error:#}"),
            };
            ctx.reply(response).await?;
            Ok(true)
        },
    );

    plugin
}

async fn public_ip() -> Result<IpAddr> {
    HTTP_CLIENT
        .get(IP_API_URL)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .context("请求 IP 查询服务失败")?
        .error_for_status()
        .context("IP 查询服务返回错误状态")?
        .text()
        .await
        .context("读取 IP 查询结果失败")?
        .trim()
        .parse()
        .context("IP 查询服务返回了无效地址")
}
