use bocchi::{
    chain::Rule,
    plugin::Plugin,
    schema::{Emoji, MessageContent, SendMsgEmojiLikeParams, SendMsgParams},
};

pub fn gpt_plugin() -> Plugin {
    let mut plugin = Plugin::new();
    plugin.on(
        Rule::on_message() & Rule::on_prefix("/gpt"),
        |caller, event| {
            Box::pin(async move {
                let text = event
                    .plain_text()
                    .trim_start_matches("/gpt")
                    .trim()
                    .to_owned();
                if !text.is_empty() {
                    caller
                        .send_msg_emoji_like(SendMsgEmojiLikeParams {
                            message_id: event.message_id(),
                            emoji_id: Emoji::闪光_2.id(),
                        })
                        .await?;
                    caller
                        .send_msg(SendMsgParams {
                            user_id: Some(event.user_id()),
                            group_id: event.group_id(),
                            message: MessageContent::Text("Hello, GPT!".to_string()),
                            message_type: None,
                            auto_escape: true,
                        })
                        .await?;
                    caller
                        .send_msg_emoji_like(SendMsgEmojiLikeParams {
                            message_id: event.message_id(),
                            emoji_id: Emoji::庆祝_2.id(),
                        })
                        .await?;
                }
                Ok(())
            })
        },
    );
    plugin
}
