use crate::{adapter::Caller, error::ApiError, schema::*};
use anyhow::Result;

pub async fn get_login_info(connector: &dyn Caller) -> Result<GetLoginInfoResult> {
    connector
        .call(ApiRequest::new(RequestParams::GetLoginInfo))
        .await?
        .data
        .into_get_login_info()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

pub async fn send_private_msg(
    connector: &dyn Caller,
    param: SendPrivateMsgParams,
) -> Result<SendMsgResult> {
    connector
        .call(ApiRequest::new(RequestParams::SendPrivateMsg(param)))
        .await?
        .data
        .into_send_msg()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

pub async fn send_group_msg(
    connector: &dyn Caller,
    param: SendGroupMsgParams,
) -> Result<SendMsgResult> {
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

pub async fn delete_msg(connector: &dyn Caller, param: DeleteMsgParams) -> Result<()> {
    connector
        .call(ApiRequest::new(RequestParams::DeleteMsg(param)))
        .await?;
    Ok(())
}

pub async fn get_msg(connector: &dyn Caller, param: GetMsgParams) -> Result<GetMsgResult> {
    connector
        .call(ApiRequest::new(RequestParams::GetMsg(param)))
        .await?
        .data
        .into_get_msg()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

pub async fn get_forward_msg(
    connector: &dyn Caller,
    param: GetForwardMsgParams,
) -> Result<GetForwardMsgResult> {
    connector
        .call(ApiRequest::new(RequestParams::GetForwardMsg(param)))
        .await?
        .data
        .into_get_forward_msg()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}
