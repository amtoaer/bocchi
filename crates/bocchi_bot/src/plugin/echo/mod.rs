use bocchi::{
    chain::Rule,
    plugin::Plugin,
    schema::{MessageContent, SendMsgParams},
};

pub fn echo_plugin() -> Plugin {
    let mut plugin = Plugin::new("回显插件", "回显用户输入的文本");

    plugin.on(
        "原样输出 echo 后的内容",
        i32::default(),
        Rule::on_message() & Rule::on_prefix("#echo"),
        |ctx| async move {
            let plain_text = ctx
                .event
                .plain_text()
                .trim()
                .trim_start_matches("#echo")
                .trim()
                .to_owned();
            if !plain_text.is_empty() {
                let msg = MessageContent::Text(plain_text);
                ctx.caller
                    .send_msg(SendMsgParams {
                        user_id: Some(ctx.event.user_id()),
                        group_id: ctx.event.try_group_id().ok(),
                        message: msg,
                        auto_escape: true,
                        message_type: None,
                    })
                    .await?;
            }
            Ok(true)
        },
    );

    plugin
}
