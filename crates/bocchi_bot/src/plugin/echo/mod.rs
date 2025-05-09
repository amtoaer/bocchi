use bocchi::{
    chain::Rule,
    plugin::Plugin,
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
                ctx.send(plain_text).await?;
            }
            Ok(true)
        },
    );

    plugin
}
