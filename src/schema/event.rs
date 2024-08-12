use serde::Deserialize;

use crate::schema::MessageSegment;

#[derive(Deserialize, Debug)]
pub struct Sender {
    user_id: Option<i64>,
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
    user_id: i64,
    message: Vec<MessageSegment>,
    raw_message: String,
    font: i32,
    sender: Sender,
}

#[derive(Deserialize, Debug)]
pub struct GroupMessage {
    time: i64,
    self_id: i64,
    post_type: String,
    message_type: String,
    sub_type: String,
    message_id: i32,
    group_id: i64,
    user_id: i64,
    anonymous: Option<Anonymous>,
    message: Vec<MessageSegment>,
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
