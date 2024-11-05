mod bilibili;
mod youtube;

use std::{future::Future, pin::Pin};

use bocchi::{
    chain::Rule,
    plugin::Plugin,
    schema::{MessageContent, SendMsgParams},
};
use futures::StreamExt;

type AsyncMaybeMsg = Pin<Box<dyn Future<Output = Option<MessageContent>> + Send>>;
type Recognizer = fn(String, i32) -> AsyncMaybeMsg;

pub fn video_detail_plugin() -> Plugin {
    let mut plugin = Plugin::new("视频详情插件", "识别消息中的视频链接，展示详情");
    plugin.on(
        "识别消息中是否包含视频链接",
        1, // 优先级比默认的高，以便在其他插件之前处理，此插件仅返回 false，确保不会阻止其他插件的执行
        Rule::on_group_message(),
        |ctx| {
            Box::pin(async move {
                let (plain_text, message_id) = (ctx.event.plain_text(), ctx.event.message_id());
                let named_recognizers: [(&str, Recognizer); 2] =
                    [("哔哩哔哩", bilibili::recognizer), ("Youtube", youtube::recognizer)];
                let mut futures_unordered = named_recognizers
                    .into_iter()
                    .map(|(name, recognizer)| {
                        let plain_text = plain_text.to_string();
                        async move { (name, recognizer(plain_text, message_id).await) }
                    })
                    .collect::<futures::stream::FuturesUnordered<_>>();
                while let Some(res) = futures_unordered.next().await {
                    let (name, Some(message)) = res else {
                        continue;
                    };
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
                        error!("{} 平台获取消息成功但发送失败: {:?}", name, e);
                    }
                    // 暂时认为消息中只会包含一种链接
                    break;
                }
                Ok(false)
            })
        },
    );

    plugin
}
