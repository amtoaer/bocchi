use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::Result;

use crate::{
    adapter::Caller,
    plugin::Plugin,
    schema::{Event, MessageContent, MessageSegment, SendForwardMsgParams, SendMsgParams, SendMsgResult},
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

    /// rust-analyzer 认为我开启了所有的 feature，导致报 unreachable_code，忽略掉
    #[allow(unreachable_code)]
    pub async fn set_reaction(&self, emoji: impl Into<i32>) -> Result<serde_json::Value> {
        #[cfg(feature = "go-cqhttp")]
        return self
            .caller
            .set_group_reaction(crate::schema::SetGroupReactionParams {
                message_id: self.event.message_id(),
                emoji_id: emoji.into(),
            })
            .await;
        #[cfg(feature = "napcat")]
        return self
            .caller
            .set_msg_emoji_like(crate::schema::SetMsgEmojiLikeParams {
                message_id: self.event.message_id(),
                emoji_id: emoji.into(),
            })
            .await;
        #[cfg(not(any(feature = "go-cqhttp", feature = "napcat")))]
        unreachable!("Unsupported")
    }

    pub async fn send_forward(&self, messages: Vec<String>) -> Result<SendMsgResult> {
        self.send_forward_content(messages.into_iter().map(|m| MessageContent::Text(m)).collect())
            .await
    }

    pub async fn send_forward_content(&self, messages: Vec<MessageContent>) -> Result<SendMsgResult> {
        let user_id = self.event.try_user_id().ok().map(|id| id.to_string());
        let nickname = self.event.sender().nickname.clone();
        self.caller
            .send_forward_msg(SendForwardMsgParams {
                user_id: self.event.try_private_user_id().ok(),
                group_id: self.event.try_group_id().ok(),
                messages: MessageContent::Segment(
                    messages
                        .into_iter()
                        .map(|m| MessageSegment::Node {
                            id: None,
                            user_id: user_id.clone(),
                            nickname: nickname.clone(),
                            content: Some(m),
                        })
                        .collect(),
                ),
                message_type: None,
            })
            .await
    }
}

pub type Handler = Box<dyn Fn(Context) -> Pin<Box<dyn Future<Output = Result<bool>> + Send>> + Send + Sync>;
