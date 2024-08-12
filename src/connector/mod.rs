use anyhow::{Error, Result};
use async_trait::async_trait;
mod error;
mod ws;

pub use ws::WsConnector;

use crate::schema::{ApiRequest, ApiResponse};

#[derive(Debug, Default)]
pub enum Status {
    #[default]
    NotConnected,
    Connected,
    #[allow(dead_code)]
    Disconnected(Option<Error>),
}

#[async_trait]
pub trait Connector {
    async fn call(&self, request: ApiRequest) -> Result<ApiResponse>;

    // TODO: 设计一个 handler 结构，用于处理信息
    async fn spawn(&mut self);
}
