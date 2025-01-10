mod bilibili;
mod spotify;
mod youtube;

use std::{future::Future, pin::Pin};

use bocchi::{
    chain::Rule,
    plugin::Plugin,
    schema::{MessageContent, SendMsgParams},
};
use futures::{stream::FuturesUnordered, StreamExt};

pub fn url_detail_plugin() -> Plugin {
    let mut plugin = Plugin::new("链接解析插件", "解析消息中的链接，展示详情");
    plugin.on(
        "识别消息中是否包含可解析详情的链接",
        1, // 优先级比默认的高，以便在其他插件之前处理，此插件仅返回 false，确保不会阻止其他插件的执行
        Rule::on_group_message(),
        |ctx| async move {
            let (plain_text, message_id) = (ctx.event.plain_text(), ctx.event.message_id());
            let futures: [Pin<Box<dyn Future<Output = Option<MessageContent>> + Send>>; 3] = [
                Box::pin(bilibili::recognizer(&plain_text, message_id)),
                Box::pin(youtube::recognizer(&plain_text, message_id)),
                Box::pin(spotify::recognizer(&plain_text, message_id)),
            ];
            let mut futures_unordered = futures.into_iter().collect::<FuturesUnordered<_>>();
            while let Some(res) = futures_unordered.next().await {
                let Some(message) = res else {
                    continue;
                };
                if let Err(e) = ctx
                    .caller
                    .send_msg(SendMsgParams {
                        message_type: None,
                        user_id: ctx.event.try_user_id().ok(),
                        group_id: ctx.event.try_group_id().ok(),
                        message,
                        auto_escape: true,
                    })
                    .await
                {
                    error!("获取消息成功但发送失败: {:?}", e);
                }
                // 暂时认为消息中只会包含一种链接
                break;
            }
            Ok(false)
        },
    );

    plugin
}
