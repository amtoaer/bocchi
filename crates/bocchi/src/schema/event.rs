use std::borrow::Cow;

use serde::Deserialize;

use crate::schema::{MessageContent, MessageSegment};

#[derive(Deserialize, Debug)]
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
    pub fn sender(&self) -> &Sender {
        match self {
            Self::GroupMessage(GroupMessage { sender, .. })
            | Self::PrivateMessage(PrivateMessage { sender, .. }) => sender,
            _ => panic!("Event::sender() called on non-message event"),
        }
    }

    pub fn message(&self) -> &MessageContent {
        match self {
            Self::GroupMessage(GroupMessage { message, .. })
            | Self::PrivateMessage(PrivateMessage { message, .. }) => message,
            _ => panic!("Event::message() called on non-message event"),
        }
    }

    pub fn user_id(&self) -> u64 {
        match self {
            Self::GroupMessage(GroupMessage { user_id, .. })
            | Self::PrivateMessage(PrivateMessage { user_id, .. }) => *user_id,
            _ => panic!("Event::user_id() called on non-message event"),
        }
    }

    pub fn nickname(&self) -> String {
        match self {
            Self::GroupMessage(GroupMessage { sender, .. })
            | Self::PrivateMessage(PrivateMessage { sender, .. }) => {
                sender.nickname.clone().unwrap_or_default()
            }
            _ => panic!("Event::nickname() called on non-message event"),
        }
    }

    pub fn group_id(&self) -> Option<u64> {
        match self {
            Self::GroupMessage(GroupMessage { group_id, .. }) => Some(*group_id),
            Self::PrivateMessage(_) => None,
            _ => panic!("Event::group_id() called on non-message event"),
        }
    }

    pub fn message_id(&self) -> i32 {
        match self {
            Self::GroupMessage(GroupMessage { message_id, .. })
            | Self::PrivateMessage(PrivateMessage { message_id, .. }) => *message_id,
            _ => panic!("Event::message_id() called on non-message event"),
        }
    }

    /// 带有 CQ 码的原始消息
    pub fn raw_message(&self) -> &str {
        match self {
            Self::GroupMessage(GroupMessage { raw_message, .. })
            | Self::PrivateMessage(PrivateMessage { raw_message, .. }) => raw_message.trim(),
            _ => panic!("Event::raw_message() called on non-message event"),
        }
    }

    /// 不带有 CQ 码的纯文本消息
    pub fn plain_text(&'a self) -> Cow<'a, str> {
        let msg = self.message();
        match msg {
            MessageContent::Text(text) => Cow::Borrowed(text),
            MessageContent::Segment(segment) => segment
                .iter()
                .filter_map(|seg| match seg {
                    MessageSegment::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<String>()
                .into(),
        }
    }
}
