mod youtube;

use bocchi::{chain::Rule, plugin::Plugin, schema::SendMsgParams};

pub fn video_detail_plugin() -> Plugin {
    let mut plugin = Plugin::new("视频详情插件", "识别消息中的视频链接，展示详情");
    plugin.on(
        "注册消息文本监听",
        1, // 优先级比默认的高，以便在其他插件之前处理，此插件仅返回 false，确保不会阻止其他插件的执行
        Rule::on_message(),
        |ctx| {
            Box::pin(async move {
                let (plain_text, message_id) = (ctx.event.plain_text(), ctx.event.message_id());
                for recognizer in [youtube::recognizer] {
                    if let Some(message) = recognizer(&plain_text, message_id).await {
                        if let Err(e) = ctx
                            .caller
                            .send_msg(SendMsgParams {
                                message_type: None,
                                user_id: Some(ctx.event.user_id()),
                                group_id: ctx.event.group_id(),
                                message,
                                auto_escape: true,
                            })
                            .await
                        {
                            error!("发送 Youtube 详情消息失败: {:?}", e);
                        }
                        // 暂时认为消息中只会包含一种链接
                        break;
                    }
                }
                Ok(false)
            })
        },
    );

    plugin
}
