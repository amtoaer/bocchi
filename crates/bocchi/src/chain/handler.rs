use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::Result;

use crate::{
    adapter::Caller,
    plugin::Plugin,
    schema::{Event, MessageContent, MessageSegment, SendMsgParams, SendMsgResult},
};

#[derive(Clone)]
pub struct Context {
    pub caller: Arc<dyn Caller>,
    pub event: Arc<Event>,
    pub plugins: Arc<Vec<Plugin>>,
}

impl Context {
    pub async fn send(&self, message: impl Into<String>) -> Result<SendMsgResult> {
        self.caller
            .send_msg(SendMsgParams {
                user_id: self.event.try_private_user_id().ok(),
                group_id: self.event.try_group_id().ok(),
                message: MessageContent::Text(message.into()),
                auto_escape: true,
                message_type: None,
            })
            .await
    }

    pub async fn send_content(&self, message: Vec<MessageSegment>) -> Result<SendMsgResult> {
        self.caller
            .send_msg(SendMsgParams {
                user_id: self.event.try_private_user_id().ok(),
                group_id: self.event.try_group_id().ok(),
                message: MessageContent::Segment(message),
                auto_escape: true,
                message_type: None,
            })
            .await
    }

    pub async fn reply(&self, message: impl Into<String>) -> Result<SendMsgResult> {
        self.send_content(vec![
            MessageSegment::Reply {
                id: self.event.message_id().to_string(),
            },
            MessageSegment::Text { text: message.into() },
        ])
        .await
    }

    pub async fn reply_content(&self, message: Vec<MessageSegment>) -> Result<SendMsgResult> {
        self.send_content(
            std::iter::once(MessageSegment::Reply {
                id: self.event.message_id().to_string(),
            })
            .chain(message.into_iter())
            .collect(),
        )
        .await
    }
}

pub type Handler = Box<dyn Fn(Context) -> Pin<Box<dyn Future<Output = Result<bool>> + Send>> + Send + Sync>;
