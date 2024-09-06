use anyhow::{Error, Result};
use async_trait::async_trait;
mod error;
mod ws;

pub use ws::WsAdapter;

use crate::{chain::MatchUnion, schema::*};

#[derive(Debug, Default)]
pub enum Status {
    #[default]
    NotConnected,
    Connected,
    Disconnected(Option<Error>),
}

#[async_trait]
pub trait Connector: Sync {
    async fn spawn(mut self: Box<Self>, match_unions: Vec<MatchUnion>);
}

#[async_trait]
pub trait Caller: Sync {
    async fn call(&self, request: ApiRequest) -> Result<ApiResponse>;
    async fn get_login_info(&self) -> Result<GetLoginInfoResult>;
    async fn send_private_msg(&self, param: SendPrivateMsgParams) -> Result<SendMsgResult>;
    async fn send_group_msg(&self, param: SendGroupMsgParams) -> Result<SendMsgResult>;
    async fn send_msg(&self, param: SendMsgParams) -> Result<SendMsgResult>;
    async fn delete_msg(&self, param: DeleteMsgParams) -> Result<serde_json::Value>;
    async fn get_msg(&self, param: GetMsgParams) -> Result<GetMsgResult>;
    async fn get_forward_msg(&self, param: GetForwardMsgParams) -> Result<GetForwardMsgResult>;
    #[cfg(feature = "napcat")]
    async fn set_msg_emoji_like(&self, param: SetMsgEmojiLikeParams) -> Result<serde_json::Value>;
}

#[async_trait]
pub trait Adapter: Connector + Caller + Sync {}
