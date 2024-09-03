use serde::Deserialize;

use crate::schema::MessageContent;

#[derive(Deserialize, Debug)]
pub struct Sender {
    pub user_id: Option<i64>,
    nickname: Option<String>,
    card: Option<String>,
    sex: Option<String>,
    age: Option<i32>,
    area: Option<String>,
    level: Option<String>,
    role: Option<String>,
    title: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Anonymous {
    id: i64,
    name: String,
    flag: String,
}

#[derive(Deserialize, Debug)]
pub struct PrivateMessage {
    time: i64,
    self_id: i64,
    post_type: String,
    message_type: String,
    sub_type: String,
    message_id: i32,
    user_id: u64,
    message: MessageContent,
    raw_message: String,
    font: i32,
    sender: Sender,
}

#[derive(Deserialize, Debug)]
pub struct GroupMessage {
    time: i64,
    self_id: u64,
    post_type: String,
    message_type: String,
    sub_type: String,
    message_id: i32,
    pub group_id: u64,
    user_id: u64,
    anonymous: Option<Anonymous>,
    message: MessageContent,
    raw_message: String,
    font: i32,
    sender: Sender,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Event {
    GroupMessage(GroupMessage),
    PrivateMessage(PrivateMessage),
}

impl Event {
    pub fn sender(&self) -> &Sender {
        match self {
            Self::GroupMessage(GroupMessage { sender, .. })
            | Self::PrivateMessage(PrivateMessage { sender, .. }) => sender,
        }
    }

    pub fn message(&self) -> &MessageContent {
        match self {
            Self::GroupMessage(GroupMessage { message, .. })
            | Self::PrivateMessage(PrivateMessage { message, .. }) => message,
        }
    }

    pub fn user_id(&self) -> u64 {
        match self {
            Self::GroupMessage(GroupMessage { user_id, .. })
            | Self::PrivateMessage(PrivateMessage { user_id, .. }) => *user_id,
        }
    }

    pub fn group_id(&self) -> Option<u64> {
        match self {
            Self::GroupMessage(GroupMessage { group_id, .. }) => Some(*group_id),
            _ => None,
        }
    }

    pub fn message_id(&self) -> i32 {
        match self {
            Self::GroupMessage(GroupMessage { message_id, .. })
            | Self::PrivateMessage(PrivateMessage { message_id, .. }) => *message_id,
        }
    }

    pub fn raw_message(&self) -> &str {
        match self {
            Self::GroupMessage(GroupMessage { raw_message, .. })
            | Self::PrivateMessage(PrivateMessage { raw_message, .. }) => raw_message.trim(),
        }
    }
}
