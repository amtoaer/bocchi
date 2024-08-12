use crate::{
    connector::Connector,
    error::ApiError,
    schema::{
        ApiRequest, GetLoginInfoResult, RequestParams, SendPrivateMsgParams, SendPrivateMsgResult,
    },
};
use anyhow::Result;

pub async fn send_private_msg(
    connector: &dyn Connector,
    param: SendPrivateMsgParams,
) -> Result<SendPrivateMsgResult> {
    connector
        .call(ApiRequest::new(RequestParams::SendPrivateMsg(param)))
        .await?
        .data
        .into_send_private_msg()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}

pub async fn get_login_info(connector: &dyn Connector) -> Result<GetLoginInfoResult> {
    connector
        .call(ApiRequest::new(RequestParams::GetLoginInfo))
        .await?
        .data
        .into_get_login_info()
        .map_err(|e| ApiError::ResponseTypeError(e).into())
}
