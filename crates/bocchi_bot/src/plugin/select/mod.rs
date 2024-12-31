use bocchi::{
    chain::Rule,
    plugin::Plugin,
    schema::{MessageContent, MessageSegment, SendMsgParams},
};
use rand::seq::SliceRandom;

pub fn select_plugin() -> Plugin {
    let mut plugin = Plugin::new("随机选择插件", "解决选择困难症");

    plugin.on(
        "随机选择",
        i32::default(),
        Rule::on_message() & Rule::on_prefix("#select"),
        |ctx| async move {
            let plain_text = ctx.event.plain_text();
            let choices = plain_text
                .trim()
                .trim_start_matches("#select")
                .split("/")
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            if !choices.is_empty() {
                let choice = choices.choose(&mut rand::thread_rng());
                if let Some(choice) = choice {
                    ctx.caller
                        .send_msg(SendMsgParams {
                            user_id: ctx.event.try_user_id().ok(),
                            group_id: ctx.event.try_group_id().ok(),
                            message: MessageContent::Segment(vec![
                                MessageSegment::Reply {
                                    id: ctx.event.message_id().to_string(),
                                },
                                MessageSegment::Text {
                                    text: choice.to_string(),
                                },
                            ]),
                            auto_escape: true,
                            message_type: None,
                        })
                        .await?;
                }
            }
            Ok(true)
        },
    );

    plugin
}
