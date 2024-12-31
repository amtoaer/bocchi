use std::{str::FromStr, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use http::Uri;
use tokio::{net::TcpStream, sync::oneshot::Sender, time};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::{
    adapter::{error::ConnectError, extract_match_unions, Adapter, Caller, Connector},
    caller::*,
    chain::Context,
    plugin::Plugin,
    schema::*,
};

#[derive(Debug)]
pub struct WsAdapter {
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    request_recorder: Arc<DashMap<u64, Sender<ApiResponse>>>,
    request_tx: Option<tokio::sync::mpsc::Sender<ApiRequest>>,
}

impl WsAdapter {
    pub async fn connect(address: &str) -> Result<Box<Self>> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(Uri::from_str(address)?).await?;
        Ok(Box::new(WsAdapter {
            ws_stream: Some(ws_stream),
            request_recorder: Arc::new(DashMap::new()),
            request_tx: None,
        }))
    }
}

#[async_trait]
impl Connector for WsAdapter {
    async fn spawn(mut self: Box<Self>, plugins: Vec<Plugin>) -> Result<()> {
        let ws_stream = self.ws_stream.take().ok_or(ConnectError::WebSocket)?;
        info!("Bot started");
        // 进行一些全局初始化工作
        let (request_tx, mut request_rx) = tokio::sync::mpsc::channel(32);
        self.request_tx = Some(request_tx);
        let (mut ws_sink, mut ws_stream) = ws_stream.split();
        // 初始化发送请求任务
        let (sender_res_tx, sender_res_rx) = tokio::sync::oneshot::channel::<Result<()>>();
        // 启动发送请求任务
        tokio::spawn(async move {
            let e = async {
                while let Some(msg) = request_rx.recv().await {
                    // 从请求通道中接收请求，发送到 websocket 服务器
                    ws_sink.send(Message::text(serde_json::to_string(&msg)?)).await?;
                }
                // magic from https://rust-lang.github.io/async-book/07_workarounds/02_err_in_async_blocks.html
                Ok::<_, anyhow::Error>(())
            }
            .await;
            let _ = sender_res_tx.send(e);
        });
        // 初始化接收消息任务
        let request_recorder = self.request_recorder.clone();
        let self = Arc::new(*self);
        let match_unions = Arc::new(extract_match_unions(&plugins));
        let plugins = Arc::new(plugins);
        let (receiver_res_tx, receiver_res_rx) = tokio::sync::oneshot::channel::<Result<()>>();
        // 启动接收消息任务
        tokio::spawn(async move {
            let e = async {
                while let Some(msg) = ws_stream.next().await {
                    let msg = msg?;
                    match msg {
                        Message::Close(_) => {
                            error!("Connection closed");
                            break;
                        }
                        Message::Text(text) => {
                            if let Ok(resp) = serde_json::from_str::<ApiResponse>(&text) {
                                if let Some((_, tx)) = request_recorder.remove(&resp.echo()) {
                                    if let Err(e) = tx.send(resp) {
                                        error!("Failed to send response: {e:?}");
                                    }
                                } else {
                                    error!("Received response with unknown request ID: {text}");
                                }
                            } else if let Ok(event) = serde_json::from_str::<Event>(&text) {
                                debug!("Receive event: {event:?}");
                                let context = Context {
                                    caller: self.clone(),
                                    event: Arc::new(event),
                                    plugins: plugins.clone(),
                                };
                                let match_unions_clone = match_unions.clone();
                                tokio::spawn(async move {
                                    // 按照优先级顺序匹配并处理事件
                                    for match_union in match_unions_clone.iter() {
                                        if match_union.matcher.is_match(&context.event) {
                                            match (*match_union.handler)(context.clone()).await {
                                                // 事件的返回值被视为中断标志，如果返回 true
                                                Err(e) => error!("Failed to handle event with : {e:?}"),
                                                Ok(true) => break,
                                                _ => (),
                                            }
                                        }
                                    }
                                });
                            } else {
                                warn!("Receive unknown message: {text}");
                            }
                        }
                        _ => (),
                    }
                }
                Ok::<_, anyhow::Error>(())
            }
            .await;
            let _ = receiver_res_tx.send(e);
        });
        let res = tokio::select! {
            res = sender_res_rx => {
                let res = res?;
                error!("Send request task exited: {res:?}");
                res
            },
            res = receiver_res_rx => {
                let res = res?;
                error!("Receive message task exited: {res:?}");
                res
            },
        };
        res
    }
}

#[async_trait]
impl Caller for WsAdapter {
    async fn call(&self, payload: ApiRequest) -> Result<ApiResponse> {
        let request_tx = self
            .request_tx
            .as_ref()
            .ok_or(ConnectError::Status("Bot not started"))?;
        let (tx, rx) = tokio::sync::oneshot::channel::<ApiResponse>();
        let echo = payload.echo();
        self.request_recorder.insert(echo, tx);
        let res = async {
            request_tx.send(payload).await?;
            tokio::select! {
                response = rx => {
                    Ok(response?)
                }
                _ = time::sleep(time::Duration::from_secs(5)) => {
                    Err(ConnectError::Timeout.into())
                }
            }
        }
        .await;
        // no matter success or failure, remove the request from the recorder
        self.request_recorder.remove(&echo);
        res
    }

    async fn get_login_info(&self) -> Result<GetLoginInfoResult> {
        get_login_info(self).await
    }

    async fn send_private_msg(&self, param: SendPrivateMsgParams) -> Result<SendMsgResult> {
        send_private_msg(self, param).await
    }

    async fn send_group_msg(&self, param: SendGroupMsgParams) -> Result<SendMsgResult> {
        send_group_msg(self, param).await
    }

    async fn send_msg(&self, param: SendMsgParams) -> Result<SendMsgResult> {
        send_msg(self, param).await
    }

    async fn delete_msg(&self, param: DeleteMsgParams) -> Result<serde_json::Value> {
        delete_msg(self, param).await
    }

    async fn get_msg(&self, param: GetMsgParams) -> Result<GetMsgResult> {
        get_msg(self, param).await
    }

    async fn get_forward_msg(&self, param: GetForwardMsgParams) -> Result<GetForwardMsgResult> {
        get_forward_msg(self, param).await
    }

    #[cfg(feature = "napcat")]
    async fn set_msg_emoji_like(&self, param: SetMsgEmojiLikeParams) -> Result<serde_json::Value> {
        set_msg_emoji_like(self, param).await
    }

    #[cfg(feature = "napcat")]
    async fn send_forward_msg(&self, param: SendForwardMsgParams) -> Result<SendMsgResult> {
        send_forward_msg(self, param).await
    }
}

#[async_trait]
impl Adapter for WsAdapter {}
