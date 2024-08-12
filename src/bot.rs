use crate::{
    caller,
    connector::{self, Connector},
    schema::{SendPrivateMsgParams, SendPrivateMsgResult},
};
use anyhow::Result;
use derive_more::derive::Display;

#[derive(Display)]
#[display("Bot: id: {:?}, nickname: {:?}", id, nickname)]
pub struct Bot {
    connector: Box<dyn Connector>,
    id: i64,
    nickname: String,
}

impl Bot {
    // TODO: 最好提供一个不直接运行的方法，比如 build
    pub async fn run(address: &str) -> Result<Self> {
        let mut connector = connector::WsConnector::connect(address).await?;
        connector.spawn().await;
        let login_info = caller::get_login_info(&*connector).await?;
        let bot = Self {
            connector,
            id: login_info.user_id,
            nickname: login_info.nickname,
        };
        info!("Bot started: {}", bot);
        Ok(bot)
    }

    pub async fn send_private_msg(
        &self,
        param: SendPrivateMsgParams,
    ) -> Result<SendPrivateMsgResult> {
        caller::send_private_msg(&*self.connector, param).await
    }
}
