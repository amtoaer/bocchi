use std::sync::OnceLock;

use bocchi::{chain::Rule, plugin::Plugin};
use bollard::{container::StartContainerOptions, secret::ContainerStateStatusEnum, Docker};

pub fn docker_plugin() -> Plugin {
    let mut plugin = Plugin::new("饥荒插件", "控制饥荒服务器");

    plugin.on(
        "启动饥荒服务器",
        i32::default(),
        Rule::on_group_id(954985908) & Rule::on_exact_match("#dst_start"),
        |ctx| async move {
            ctx.reply("收到命令，正在启动饥荒服务器..").await?;
            let resp_content = match docker_start("dst-server").await {
                Ok(_) => "服务器已启动".to_string(),
                Err(e) => format!("服务器启动失败: {e}"),
            };
            ctx.reply(resp_content).await?;
            Ok(true)
        },
    );

    plugin.on(
        "停止饥荒服务器",
        i32::default(),
        Rule::on_group_id(954985908) & Rule::on_exact_match("#dst_stop"),
        |ctx| async move {
            ctx.reply("收到命令，正在停止饥荒服务器..").await?;
            let resp_content = match docker_stop("dst-server").await {
                Ok(_) => "服务器已停止".to_string(),
                Err(e) => format!("服务器停止失败: {e}"),
            };
            ctx.reply(resp_content).await?;
            Ok(true)
        },
    );

    plugin.on(
        "重启饥荒服务器",
        i32::default(),
        Rule::on_group_id(954985908) & Rule::on_exact_match("#dst_restart"),
        |ctx| async move {
            ctx.reply("收到命令，正在重启饥荒服务器..").await?;
            let resp_content = match docker_restart("dst-server").await {
                Ok(_) => "服务器已重启".to_string(),
                Err(e) => format!("服务器重启失败: {e}"),
            };
            ctx.reply(resp_content).await?;
            Ok(true)
        },
    );

    plugin
}

fn docker_connect() -> &'static Docker {
    static DOCKER: OnceLock<Docker> = OnceLock::new();
    DOCKER.get_or_init(|| Docker::connect_with_defaults().unwrap())
}

async fn docker_status(container_name: &str) -> Result<ContainerStateStatusEnum, anyhow::Error> {
    let stats = docker_connect().inspect_container(container_name, None).await?;
    Ok(stats
        .state
        .and_then(|state| state.status)
        .ok_or_else(|| anyhow::Error::msg("未获取到容器状态"))?)
}

async fn docker_start(container_name: &str) -> Result<(), anyhow::Error> {
    let status = docker_status(container_name).await?;
    match status {
        ContainerStateStatusEnum::PAUSED | ContainerStateStatusEnum::EXITED => {
            docker_connect()
                .start_container(container_name, None::<StartContainerOptions<String>>)
                .await?;
            Ok(())
        }
        ContainerStateStatusEnum::RUNNING
        | ContainerStateStatusEnum::CREATED
        | ContainerStateStatusEnum::RESTARTING => Err(anyhow::Error::msg("容器正在重启或已启动")),
        _ => Err(anyhow::anyhow!("不支持的容器状态: {}", status)),
    }
}

async fn docker_stop(container_name: &str) -> Result<(), anyhow::Error> {
    let status = docker_status(container_name).await?;
    match status {
        ContainerStateStatusEnum::RUNNING
        | ContainerStateStatusEnum::CREATED
        | ContainerStateStatusEnum::RESTARTING => {
            docker_connect().stop_container(container_name, None).await?;
            Ok(())
        }
        ContainerStateStatusEnum::PAUSED | ContainerStateStatusEnum::EXITED => {
            Err(anyhow::Error::msg("容器已停止或已暂停"))
        }
        _ => Err(anyhow::anyhow!("不支持的容器状态: {}", status)),
    }
}

async fn docker_restart(container_name: &str) -> Result<(), anyhow::Error> {
    let status = docker_status(container_name).await?;
    match status {
        ContainerStateStatusEnum::RUNNING
        | ContainerStateStatusEnum::CREATED
        | ContainerStateStatusEnum::PAUSED
        | ContainerStateStatusEnum::EXITED => {
            docker_connect().restart_container(container_name, None).await?;
            Ok(())
        }
        ContainerStateStatusEnum::RESTARTING => Err(anyhow::Error::msg("容器已在重启")),
        _ => Err(anyhow::anyhow!("不支持的容器状态: {}", status)),
    }
}
