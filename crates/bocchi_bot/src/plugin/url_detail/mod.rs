mod bilibili;
mod spotify;
mod youtube;

use std::{future::Future, pin::Pin};

use bocchi::{chain::Rule, plugin::Plugin, schema::MessageSegment};
use futures::{stream::FuturesUnordered, StreamExt};

pub fn url_detail_plugin() -> Plugin {
    let mut plugin = Plugin::new("链接解析插件", "解析消息中的链接，展示详情");
    plugin.on(
        "识别消息中是否包含可解析详情的链接",
        1, // 优先级比默认的高，以便在其他插件之前处理，此插件仅返回 false，确保不会阻止其他插件的执行
        Rule::on_group_message(),
        |ctx| async move {
            let plain_text = ctx.event.plain_text();
            let futures: [Pin<Box<dyn Future<Output = Option<Vec<MessageSegment>>> + Send>>; 3] = [
                Box::pin(bilibili::recognizer(&plain_text)),
                Box::pin(youtube::recognizer(&plain_text)),
                Box::pin(spotify::recognizer(&plain_text)),
            ];
            let mut futures_unordered = futures.into_iter().collect::<FuturesUnordered<_>>();
            while let Some(res) = futures_unordered.next().await {
                let Some(message) = res else {
                    continue;
                };
                if let Err(e) = ctx.reply_content(message).await {
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
