use anyhow::{Error, Result};
use async_trait::async_trait;
mod error;
mod ws;

pub use ws::WsConnector;

use crate::{
    chain::MatchUnion,
    schema::{ApiRequest, ApiResponse},
};

#[derive(Debug, Default)]
pub enum Status {
    #[default]
    NotConnected,
    Connected,
    #[allow(dead_code)]
    Disconnected(Option<Error>),
}

#[async_trait]
pub trait Connector: Sync {
    async fn call(&self, request: ApiRequest) -> Result<ApiResponse>;

    async fn spawn(mut self: Box<Self>, match_unions: Vec<MatchUnion>);
}
