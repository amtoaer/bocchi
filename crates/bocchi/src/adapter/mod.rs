use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
mod error;
mod ws;

pub use ws::WsAdapter;

use crate::{chain::MatchUnion, plugin::Plugin, schema::*};

#[async_trait]
pub trait Connector: Send + Sync {
    async fn spawn(mut self: Box<Self>, plugins: Vec<Plugin>) -> Result<()>;
}

#[async_trait]
pub trait Caller: Send + Sync {
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
    #[cfg(feature = "napcat")]
    async fn send_forward_msg(&self, param: SendForwardMsgParams) -> Result<SendMsgResult>;
}

#[async_trait]
pub trait Adapter: Connector + Caller {}

pub(crate) fn extract_match_unions(plugins: &[Plugin]) -> Vec<Arc<MatchUnion>> {
    // 每个插件都有自己的 MatchUnion，但处理时不按插件分割，而是统一按照优先级排序处理
    // 将排序过程提前，避免在处理任务中重复排序（引入的代价就是 MatchUnion 需要用 Arc 包装）
    let mut match_unions = plugins
        .iter()
        .flat_map(|plugin| plugin.match_unions())
        .cloned()
        .collect::<Vec<_>>();
    // 优先级从大到小排序
    match_unions.sort_by(|a, b| b.priority.cmp(&a.priority));
    match_unions
}
