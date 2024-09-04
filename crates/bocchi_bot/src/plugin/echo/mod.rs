use bocchi::{
    chain::Rule,
    plugin::Plugin,
    schema::{MessageContent, SendMsgParams},
};

pub fn echo_plugin() -> Plugin {
    let mut plugin = Plugin::new();

    plugin.on(
        Rule::on_message() & Rule::on_prefix("echo"),
        |caller, event| {
            Box::pin(async move {
                let plain_text = event
                    .plain_text()
                    .trim_start_matches("echo")
                    .trim()
                    .to_owned();
                if !plain_text.is_empty() {
                    let msg = MessageContent::Text(plain_text);
                    caller
                        .send_msg(SendMsgParams {
                            user_id: Some(event.user_id()),
                            group_id: event.group_id(),
                            message: msg,
                            auto_escape: true,
                            message_type: None,
                        })
                        .await?;
                }
                Ok(())
            })
        },
    );

    plugin
}
