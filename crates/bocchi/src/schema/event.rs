use std::borrow::Cow;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::schema::{MessageContent, MessageSegment};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Sender {
    pub user_id: Option<i64>,
    pub nickname: Option<String>,
    pub card: Option<String>,
    pub sex: Option<String>,
    pub age: Option<i32>,
    pub area: Option<String>,
    pub level: Option<String>,
    pub role: Option<String>,
    pub title: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Anonymous {
    pub id: i64,
    pub name: String,
    pub flag: String,
}

#[derive(Deserialize, Debug)]
pub struct PrivateMessage {
    pub time: i64,
    pub self_id: i64,
    pub post_type: String,
    pub message_type: String,
    pub sub_type: String,
    pub message_id: i32,
    pub user_id: u64,
    pub message: MessageContent,
    pub raw_message: String,
    pub font: i32,
    pub sender: Sender,
}

#[derive(Deserialize, Debug)]
pub struct GroupMessage {
    pub time: i64,
    pub self_id: u64,
    pub post_type: String,
    pub message_type: String,
    pub sub_type: String,
    pub message_id: i32,
    pub group_id: u64,
    pub user_id: u64,
    pub anonymous: Option<Anonymous>,
    pub message: MessageContent,
    pub raw_message: String,
    pub font: i32,
    pub sender: Sender,
}

#[derive(Deserialize, Debug)]
pub struct LifeCycle {
    pub time: i64,
    pub self_id: u64,
    pub post_type: String,
    pub meta_event_type: String,
    pub sub_type: String,
}

#[derive(Deserialize, Debug)]
pub struct HeartBeat {
    pub time: i64,
    pub self_id: u64,
    pub post_type: String,
    pub meta_event_type: String,
    pub status: serde_json::Value,
    pub interval: i64,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Event {
    GroupMessage(GroupMessage),
    PrivateMessage(PrivateMessage),
    LifeCycle(LifeCycle),
    HeartBeat(HeartBeat),
}

impl<'a> Event {
    pub fn try_sender(&self) -> Result<&Sender> {
        match self {
            Self::GroupMessage(GroupMessage { sender, .. }) | Self::PrivateMessage(PrivateMessage { sender, .. }) => {
                Ok(sender)
            }
            _ => bail!("Event::try_sender() called on non-message event"),
        }
    }

    pub fn sender(&self) -> &Sender {
        self.try_sender().unwrap()
    }

    pub fn try_message(&self) -> Result<&MessageContent> {
        match self {
            Self::GroupMessage(GroupMessage { message, .. }) | Self::PrivateMessage(PrivateMessage { message, .. }) => {
                Ok(message)
            }
            _ => bail!("Event::try_message() called on non-message event"),
        }
    }

    pub fn message(&self) -> &MessageContent {
        self.try_message().unwrap()
    }

    pub fn try_user_id(&self) -> Result<u64> {
        match self {
            Self::GroupMessage(GroupMessage { user_id, .. }) | Self::PrivateMessage(PrivateMessage { user_id, .. }) => {
                Ok(*user_id)
            }
            _ => bail!("Event::try_user_id() called on non-message event"),
        }
    }

    pub fn user_id(&self) -> u64 {
        self.try_user_id().unwrap()
    }

    pub fn try_group_id(&self) -> Result<u64> {
        match self {
            Self::GroupMessage(GroupMessage { group_id, .. }) => Ok(*group_id),
            _ => bail!("Event::try_group_id() called on non-group-message event"),
        }
    }

    pub fn group_id(&self) -> u64 {
        self.try_group_id().unwrap()
    }

    pub fn try_nickname(&self) -> Result<&str> {
        match self {
            Self::GroupMessage(GroupMessage { sender, .. }) | Self::PrivateMessage(PrivateMessage { sender, .. }) => {
                Ok(sender.nickname.as_deref().unwrap_or_default())
            }
            _ => bail!("Event::try_nickname() called on non-message event"),
        }
    }

    pub fn nickname(&self) -> &str {
        self.try_nickname().unwrap()
    }

    pub fn try_message_id(&self) -> Result<i32> {
        match self {
            Self::GroupMessage(GroupMessage { message_id, .. })
            | Self::PrivateMessage(PrivateMessage { message_id, .. }) => Ok(*message_id),
            _ => bail!("Event::try_message_id() called on non-message event"),
        }
    }

    pub fn message_id(&self) -> i32 {
        self.try_message_id().unwrap()
    }

    pub fn try_plain_text(&self) -> Result<Cow<'_, str>> {
        let msg = self.try_message()?;
        match msg {
            // 这里作为通用实现可能是错误的，因为框架可能会发送包含 CQ 码的纯文本而非结构化数据，
            // 如 “[CQ:face,id=178]看看我刚拍的照片[CQ:image,file=123.jpg]”
            // 但实现 CQ 码解析会很麻烦，而且使用 napcat 时并未出现这种情况，所以先这样用着
            MessageContent::Text(text) => Ok(Cow::Borrowed(text)),
            MessageContent::Segment(segment) => Ok(segment
                .iter()
                .filter_map(|seg| match seg {
                    MessageSegment::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<String>()
                .into()),
        }
    }

    pub fn plain_text(&'a self) -> Cow<'a, str> {
        self.try_plain_text().unwrap()
    }
}
