use std::{borrow::Cow, future::Future};

use anyhow::Result;

use crate::{
    adapter::{self, Adapter},
    chain::{Context, Matcher, Rule},
    plugin::Plugin,
    schema::{MessageContent, MessageSegment, SendForwardMsgParams},
};

pub struct Bot {
    adapter: Box<dyn Adapter>,
    plugins: Vec<Plugin>,
}

impl Bot {
    pub async fn connect(address: &str) -> Result<Self> {
        Ok(Bot {
            adapter: adapter::WsAdapter::connect(address).await?,
            plugins: vec![Plugin::new("内建插件", "直接注册在 Bot 上的插件")],
        })
    }

    pub fn on<D, M, H, Fut>(&mut self, description: D, priority: i32, matcher: M, handler: H)
    where
        D: Into<Cow<'static, str>>,
        M: Into<Matcher>,
        H: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<bool>> + Send + 'static,
    {
        self.plugins[0].on(description, priority, matcher, handler);
    }

    pub fn register_plugin(&mut self, plugin: Plugin) {
        self.plugins.push(plugin);
    }

    pub async fn start(self) -> Result<()> {
        self.adapter.spawn(self.plugins).await
    }

    pub fn use_builtin_handler(&mut self) {
        self.on(
            "显示帮助信息",
            i32::MAX,
            Rule::on_message() & Rule::on_exact_match("#help"),
            |ctx| async move {
                let mut help_message = String::from("由 Rust 与 Tokio 驱动的机器人波奇酱！目前由如下插件提供服务：\n");
                let mut tab_str = 2;
                for plugin in ctx.plugins.as_ref() {
                    help_message.push_str(&format!(
                        "\n{}{} - {}\n",
                        " ".repeat(tab_str),
                        plugin.name,
                        plugin.description
                    ));
                    tab_str += 2;
                    for mu in plugin.match_unions() {
                        help_message.push_str(&format!("{}{} - {}\n", " ".repeat(tab_str), mu.matcher, mu.description));
                    }
                    tab_str -= 2;
                }
                ctx.caller
                    .send_forward_msg(SendForwardMsgParams {
                        user_id: ctx.event.try_user_id().ok(),
                        group_id: ctx.event.try_group_id().ok(),
                        message: MessageContent::Segment(vec![MessageSegment::Node {
                            id: None,
                            user_id: None,
                            nickname: None,
                            content: Some(MessageContent::Text(help_message)),
                        }]),
                        message_type: None,
                    })
                    .await?;
                Ok(true)
            },
        );
    }
}
