use anyhow::{bail, Result};

use crate::{adapter::Caller, error::ApiError, schema::*};

pub async fn get_login_info(connector: &dyn Caller) -> Result<GetLoginInfoResult> {
    connector
        .call(ApiRequest::new(RequestParams::GetLoginInfo))
        .await?
        .data
        .into_get_login_info()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

pub async fn send_private_msg(connector: &dyn Caller, param: SendPrivateMsgParams) -> Result<SendMsgResult> {
    connector
        .call(ApiRequest::new(RequestParams::SendPrivateMsg(param)))
        .await?
        .data
        .into_send_msg()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

pub async fn send_group_msg(connector: &dyn Caller, param: SendGroupMsgParams) -> Result<SendMsgResult> {
    connector
        .call(ApiRequest::new(RequestParams::SendGroupMsg(param)))
        .await?
        .data
        .into_send_msg()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

pub async fn send_msg(connector: &dyn Caller, param: SendMsgParams) -> Result<SendMsgResult> {
    connector
        .call(ApiRequest::new(RequestParams::SendMsg(param)))
        .await?
        .data
        .into_send_msg()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

pub async fn delete_msg(connector: &dyn Caller, param: DeleteMsgParams) -> Result<serde_json::Value> {
    connector
        .call(ApiRequest::new(RequestParams::DeleteMsg(param)))
        .await?
        .data
        .into_fallback()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

pub async fn get_msg(connector: &dyn Caller, param: GetMsgParams) -> Result<GetMsgResult> {
    connector
        .call(ApiRequest::new(RequestParams::GetMsg(param)))
        .await?
        .data
        .into_get_msg()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

pub async fn get_forward_msg(connector: &dyn Caller, param: GetForwardMsgParams) -> Result<GetForwardMsgResult> {
    connector
        .call(ApiRequest::new(RequestParams::GetForwardMsg(param)))
        .await?
        .data
        .into_get_forward_msg()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

#[cfg(feature = "napcat")]
pub async fn set_msg_emoji_like(connector: &dyn Caller, param: SetMsgEmojiLikeParams) -> Result<serde_json::Value> {
    connector
        .call(ApiRequest::new(RequestParams::SetMsgEmojiLike(param)))
        .await?
        .data
        .into_fallback()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

#[cfg(feature = "go-cqhttp")]
pub async fn set_group_reaction(connector: &dyn Caller, param: SetGroupReactionParams) -> Result<serde_json::Value> {
    connector
        .call(ApiRequest::new(RequestParams::SetGroupReaction(param)))
        .await?
        .data
        .into_fallback()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

#[cfg(feature = "lagrange")]
pub async fn set_group_reaction(connector: &dyn Caller, param: SetGroupReactionParams) -> Result<serde_json::Value> {
    connector
        .call(ApiRequest::new(RequestParams::SetGroupReaction(param)))
        .await?
        .data
        .into_fallback()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

#[cfg(feature = "lagrange")]
pub async fn send_private_forward_msg(
    connector: &dyn Caller,
    param: SendPrivateForwardMsgParams,
) -> Result<SendMsgResult> {
    connector
        .call(ApiRequest::new(RequestParams::SendPrivateForwardMsg(param)))
        .await?
        .data
        .into_send_msg()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

#[cfg(feature = "lagrange")]
pub async fn send_group_forward_msg(connector: &dyn Caller, param: SendGroupForwardMsgParams) -> Result<SendMsgResult> {
    connector
        .call(ApiRequest::new(RequestParams::SendGroupForwardMsg(param)))
        .await?
        .data
        .into_send_msg()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

#[cfg(feature = "lagrange")]
pub async fn send_forward_msg(connector: &dyn Caller, param: SendForwardMsgParams) -> Result<SendMsgResult> {
    match (param.group_id, param.user_id) {
        (Some(group_id), _) => {
            // 群聊转发
            send_group_forward_msg(
                connector,
                SendGroupForwardMsgParams {
                    group_id,
                    messages: param.messages,
                },
            )
            .await
        }
        (_, Some(user_id)) => {
            // 私聊转发
            send_private_forward_msg(
                connector,
                SendPrivateForwardMsgParams {
                    user_id,
                    messages: param.messages,
                },
            )
            .await
        }
        _ => bail!("Neither group_id nor user_id is specified"),
    }
}

#[cfg(any(feature = "napcat", feature = "go-cqhttp"))]
pub async fn send_forward_msg(connector: &dyn Caller, param: SendForwardMsgParams) -> Result<SendMsgResult> {
    connector
        .call(ApiRequest::new(RequestParams::SendForwardMsg(param)))
        .await?
        .data
        .into_send_msg()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}
