use std::{str::FromStr, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use http::Uri;
use tokio::{net::TcpStream, sync::oneshot::Sender, time};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::{
    adapter::{error::ConnectError, Adapter, Caller, Connector, Status},
    caller::*,
    chain::Context,
    plugin::Plugin,
    schema::*,
};

#[derive(Debug)]
pub struct WsAdapter {
    ws_sink: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    ws_stream: Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    request_tx: Option<tokio::sync::mpsc::Sender<ApiRequest>>,
    request_recorder: Arc<DashMap<u64, Sender<ApiResponse>>>,
    status_tx: tokio::sync::watch::Sender<Status>,
    status_rx: tokio::sync::watch::Receiver<Status>,
}

impl WsAdapter {
    pub async fn connect(address: &str) -> Result<Box<Self>> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(Uri::from_str(address)?).await?;
        // 分割 websocket 流为发送和接收两个部分
        let (ws_sink, ws_stream) = ws_stream.split();
        // 创建一个状态通道，用于通知连接状态
        let (status_tx, status_rx) = tokio::sync::watch::channel(Status::NotConnected);
        Ok(Box::new(WsAdapter {
            ws_sink: Some(ws_sink),
            ws_stream: Some(ws_stream),
            request_tx: None,
            request_recorder: Arc::new(DashMap::new()),
            status_tx,
            status_rx,
        }))
    }
}

#[async_trait]
impl Connector for WsAdapter {
    async fn spawn(mut self: Box<Self>, plugins: Vec<Plugin>) {
        let (mut ws_sink, mut ws_stream) = match (self.ws_sink.take(), self.ws_stream.take()) {
            (Some(ws_sink), Some(ws_stream)) => (ws_sink, ws_stream),
            _ => return,
        };
        // 设置开始状态
        let _ = self.status_tx.send(Status::Connected);
        // 用来推送状态变更的通道
        let (mut status_rx, status_tx) = (self.status_rx.clone(), self.status_tx.clone());
        // 用来传递请求的通道
        let (request_tx, mut request_rx) = tokio::sync::mpsc::channel(32);
        self.request_tx = Some(request_tx);
        // 启动一个用于发送请求的任务
        tokio::spawn(async move {
            let e = async {
                loop {
                    tokio::select! {
                        _ = status_rx.changed() => {
                            // 如果状态变为 Disconnected，则退出循环
                            if matches!(*status_rx.borrow_and_update(), Status::Disconnected(_)) {
                                break;
                            }
                        }
                        Some(msg) = request_rx.recv() => {
                            // 从请求通道中接收请求，发送到 websocket 服务器
                            ws_sink.send(Message::text(
                                serde_json::to_string(&msg)?
                            )).await?;
                        }
                    }
                }
                // magic from https://rust-lang.github.io/async-book/07_workarounds/02_err_in_async_blocks.html
                Ok::<_, anyhow::Error>(())
            }
            .await;
            // 如果发送请求的任务由于某种原因退出，则通知状态变更
            let _ = status_tx.send(Status::Disconnected(e.err()));
        });
        let request_recorder = self.request_recorder.clone();
        let (mut status_rx, status_tx) = (self.status_rx.clone(), self.status_tx.clone());
        let self = Arc::new(*self);
        // 每个插件都有自己的 MatchUnion，但处理时不按插件分割，而是统一按照优先级排序处理
        // 将排序过程提前，避免在处理任务中重复排序（引入的代价就是 MatchUnion 需要用 Arc 包装）
        let mut match_unions = plugins
            .iter()
            .flat_map(|plugin| plugin.match_unions())
            .cloned()
            .collect::<Vec<_>>();
        // 优先级从大到小排序
        match_unions.sort_by(|a, b| b.priority.cmp(&a.priority));
        // Arc 封装后传递给处理任务
        let match_unions = Arc::new(match_unions);
        // 插件本身保持不变，用 Arc 封装后传递给处理任务
        let plugins = Arc::new(plugins);
        // 启动一个用于接收消息的任务
        tokio::spawn(async move {
            let e = async {
                loop {
                    tokio::select! {
                        _ = status_rx.changed() => {
                            // 如果状态变为 Disconnected，则退出循环
                            if matches!(*status_rx.borrow_and_update(), Status::Disconnected(_)) {
                                break;
                            }
                        }
                        Some(msg) = ws_stream.next() => {
                            let msg = msg?;
                            match msg {
                                Message::Close(_) => {
                                    error!("Connection closed");
                                    break;
                                }
                                Message::Text(text) => {
                                    if let Ok(resp) = serde_json::from_str::<ApiResponse>(&text){
                                        if let Some((_, tx)) = request_recorder.remove(&resp.echo()) {
                                            if let Err(e) = tx.send(resp) {
                                                error!("Failed to send response: {e:?}");
                                            }
                                        } else {
                                            error!("Received response with unknown request ID: {text}");
                                        }
                                    } else if let Ok(event) = serde_json::from_str::<Event>(&text) {
                                        info!("Receive event: {event:?}");
                                        let self_clone = self.clone();
                                        let match_unions_clone = match_unions.clone();
                                        let plugins_clone = plugins.clone();
                                        tokio::spawn(async move {
                                            // 按照优先级顺序匹配并处理事件
                                            for match_union in match_unions_clone.iter() {
                                                if match_union.matcher.is_match(&event) {
                                                    match (*match_union.handler)(Context{
                                                        caller: self_clone.as_ref(),
                                                        event: &event,
                                                        plugins: &plugins_clone,
                                                    }).await {
                                                        // 事件的返回值被视为中断标志，如果返回 true
                                                        Err(e) => error!("Failed to handle event with : {e:?}"),
                                                        Ok(true) => break,
                                                        _ => ()
                                                    }
                                                }
                                            }
                                        });
                                    } else {
                                        warn!("Receive unknown message: {text}");
                                    }
                                }
                                _ => ()
                            }
                        }
                    }
                }
                Ok::<_, anyhow::Error>(())
            }
            .await;
            let _ = status_tx.send(Status::Disconnected(e.err()));
            Ok::<_, anyhow::Error>(())
        });
    }
}

#[async_trait]
impl Caller for WsAdapter {
    async fn call(&self, payload: ApiRequest) -> Result<ApiResponse> {
        {
            let status = &*self.status_rx.borrow();
            if matches!(status, Status::NotConnected | Status::Disconnected(_)) {
                return Err(
                    ConnectError::StatusError(format!("Invalid status, expect Connected, actual {status:?}")).into(),
                );
            }
        }
        let (tx, rx) = tokio::sync::oneshot::channel::<ApiResponse>();
        let echo = payload.echo();
        self.request_recorder.insert(echo, tx);
        let res = async {
            self.request_tx.as_ref().unwrap().send(payload).await?;
            tokio::select! {
                response = rx => {
                    Ok(response?)
                }
                _ = time::sleep(time::Duration::from_secs(5)) => {
                    Err(ConnectError::TimeoutError.into())
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
}

#[async_trait]
impl Adapter for WsAdapter {}
