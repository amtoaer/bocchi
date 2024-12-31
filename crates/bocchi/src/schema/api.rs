use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};

use crate::schema::message::MessageContent;
/// 发送私聊消息的参数
#[derive(Debug, Serialize)]
pub struct SendPrivateMsgParams {
    /// 对方 QQ 号
    pub user_id: u64,
    /// 要发送的内容
    pub message: MessageContent,
    /// 消息内容是否作为纯文本发送（即不解析 CQ 码），只在 message 字段是字符串时有效
    pub auto_escape: bool,
}

/// 发送群消息的参数
#[derive(Debug, Serialize)]
pub struct SendGroupMsgParams {
    /// 群号
    group_id: u64,
    /// 要发送的内容
    message: String,
    /// 消息内容是否作为纯文本发送（即不解析 CQ 码），只在 message 字段是字符串时有效
    auto_escape: bool,
}

/// 发送消息的参数
#[derive(Debug, Serialize)]
pub struct SendMsgParams {
    /// 消息类型，支持 private、group，分别对应私聊、群组，如不传入，则根据传入的 *_id 参数判断
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    /// 对方 QQ 号（消息类型为 private 时需要）
    pub user_id: Option<u64>,
    /// 群号（消息类型为 group 时需要）
    pub group_id: Option<u64>,
    /// 要发送的内容
    pub message: MessageContent,
    /// 消息内容是否作为纯文本发送（即不解析 CQ 码），只在 message 字段是字符串时有效
    pub auto_escape: bool,
}

/// 发送消息的公共响应数据
#[derive(Debug, Deserialize)]
pub struct SendMsgResult {
    /// 消息 ID
    pub message_id: i32,
}
/// 撤回消息的参数
#[derive(Debug, Serialize)]
pub struct DeleteMsgParams {
    /// 消息 ID
    message_id: i32,
}

/// 获取消息的参数
#[derive(Debug, Serialize)]
pub struct GetMsgParams {
    /// 消息 ID
    message_id: i32,
}

/// 获取消息的响应数据
#[derive(Debug, Deserialize)]
pub struct GetMsgResult {
    /// 发送时间
    pub time: i32,
    /// 消息类型，同 消息事件
    pub message_type: String,
    /// 消息 ID
    pub message_id: i32,
    /// 消息真实 ID
    pub real_id: i32,
    /// 发送人信息，同 消息事件
    pub sender: serde_json::Value, // 使用 serde_json::Value 作为占位符
    /// 消息内容
    pub message: String,
}

/// 获取合并转发消息的参数
#[derive(Debug, Serialize)]
pub struct GetForwardMsgParams {
    /// 合并转发 ID
    id: String,
}

/// 获取合并转发消息的响应数据
#[derive(Debug, Deserialize)]
pub struct GetForwardMsgResult {
    /// 消息内容，使用消息的数组格式表示，数组中的消息段全部为 node 消息段
    pub message: MessageContent,
}

/// 发送表情回应的参数
#[derive(Debug, Serialize)]
pub struct SetMsgEmojiLikeParams {
    pub message_id: i32,
    pub emoji_id: i32,
}

/// 获取登录信息的响应数据
#[derive(Debug, Deserialize)]
pub struct GetLoginInfoResult {
    pub user_id: i64,
    pub nickname: String,
}

#[cfg(feature = "napcat")]
#[derive(Debug, Serialize)]
pub struct SendForwardMsgParams {
    /// 消息类型，支持 private、group，分别对应私聊、群组，如不传入，则根据传入的 *_id 参数判断
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    /// 对方 QQ 号（消息类型为 private 时需要）
    pub user_id: Option<u64>,
    /// 群号（消息类型为 group 时需要）
    pub group_id: Option<u64>,
    /// 要发送的内容
    pub message: MessageContent,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "action", content = "params")]
pub enum RequestParams {
    GetLoginInfo,
    SendPrivateMsg(SendPrivateMsgParams),
    SendGroupMsg(SendGroupMsgParams),
    SendMsg(SendMsgParams),
    DeleteMsg(DeleteMsgParams),
    GetMsg(GetMsgParams),
    GetForwardMsg(GetForwardMsgParams),

    #[cfg(feature = "napcat")]
    SetMsgEmojiLike(SetMsgEmojiLikeParams),
    #[cfg(feature = "napcat")]
    SendForwardMsg(SendForwardMsgParams),
}
#[derive(Debug, Serialize)]
pub struct ApiRequest {
    echo: u64,
    #[serde(flatten)]
    params: RequestParams,
}

impl ApiRequest {
    pub fn new(params: RequestParams) -> Self {
        Self {
            // 发现这里如果生成的 u64 过长，接口返回的 echo 可能丢失精度，因此减小一些
            echo: rand::random::<u64>() >> 16,
            params,
        }
    }

    pub fn echo(&self) -> u64 {
        self.echo
    }
}

#[derive(Debug, Deserialize, EnumAsInner)]
#[serde(untagged)]
pub enum ResponseBody {
    GetLoginInfo(GetLoginInfoResult),
    SendMsg(SendMsgResult),
    GetMsg(GetMsgResult),
    GetForwardMsg(GetForwardMsgResult),
    Fallback(serde_json::Value),
}

#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    echo: u64,
    pub data: ResponseBody,
}

impl ApiResponse {
    pub fn echo(&self) -> u64 {
        self.echo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_serialize() {
        let send_private_msg = ApiRequest::new(RequestParams::SendPrivateMsg(SendPrivateMsgParams {
            user_id: 10000,
            message: MessageContent::Text("Hello, world!".to_string()),
            auto_escape: true,
        }));
        assert_eq!(
            serde_json::to_string(&send_private_msg).unwrap(),
            r#"{"action":"send_private_msg","params":{"user_id":10000,"message":"Hello, world!","auto_escape":false}}"#
        );
    }
}
