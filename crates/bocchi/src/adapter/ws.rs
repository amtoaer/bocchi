use std::{str::FromStr, sync::Arc};

use crate::{
    adapter::{error::ConnectError, Adapter, Caller, Connector, Status},
    caller::*,
    chain::MatchUnion,
    schema::*,
};
use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use http::Uri;
use tokio::net::TcpStream;
use tokio::sync::oneshot::Sender;
use tokio::time;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

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
        let (ws_sink, ws_stream) = ws_stream.split();
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
    async fn spawn(mut self: Box<Self>, match_unions: Vec<MatchUnion>) {
        if let (Some(mut ws_sink), Some(mut ws_stream)) =
            (self.ws_sink.take(), self.ws_stream.take())
        {
            let _ = self.status_tx.send(Status::Connected);
            let (mut status_rx, status_tx) = (self.status_rx.clone(), self.status_tx.clone());
            let (request_tx, mut request_rx) = tokio::sync::mpsc::channel(32);
            self.request_tx = Some(request_tx);
            tokio::spawn(async move {
                let e = async {
                    loop {
                        tokio::select! {
                            _ = status_rx.changed() => {
                                if matches!(*status_rx.borrow_and_update(), Status::Disconnected(_)) {
                                    break;
                                }
                            }
                            Some(msg) = request_rx.recv() => {
                                ws_sink.send(Message::text(
                                    serde_json::to_string(&msg)?
                                )).await?;
                            }
                        }
                    }
                    // magic from https://rust-lang.github.io/async-book/07_workarounds/02_err_in_async_blocks.html
                    Ok::<_, anyhow::Error>(())
                }.await;
                let _ = status_tx.send(Status::Disconnected(e.err()));
            });
            let request_recorder = self.request_recorder.clone();
            let (mut status_rx, status_tx) = (self.status_rx.clone(), self.status_tx.clone());
            let self = Arc::new(self);
            let match_unions = Arc::new(match_unions);
            tokio::spawn(async move {
                let e = async {
                    loop {
                        tokio::select! {
                            _ = status_rx.changed() => {
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
                                            info!("Receive response: {resp:?}");
                                            if let Some((_, tx)) = request_recorder.remove(&resp.echo()) {
                                                if let Err(e) = tx.send(resp) {
                                                    error!("Failed to send response: {e:?}");
                                                }
                                            } else {
                                                error!("Received response with unknown request ID: {text}");
                                            }
                                        } else if let Ok(event) = serde_json::from_str::<Event>(&text) {
                                            info!("Receive event: {event:?}");
                                            let match_unions_clone = match_unions.clone();
                                            let self_clone = self.clone();
                                            tokio::spawn(async move {
                                                for match_union in &*match_unions_clone {
                                                    if match_union.matcher.is_match(&event) {
                                                        let res = match_union.handler.as_ref()(self_clone.as_ref().as_ref(), &event).await;
                                                        if res.is_err() {
                                                            error!("Failed to handle event: {res:?}");
                                                        }
                                                    }
                                                }
                                            });
                                        } else {
                                            warn!("Receive unknown message: {text}");
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Ok::<_, anyhow::Error>(())
                }.await;
                let _ = status_tx.send(Status::Disconnected(e.err()));
                Ok::<_, anyhow::Error>(())
            });
        }
    }
}

#[async_trait]
impl Caller for WsAdapter {
    async fn call(&self, payload: ApiRequest) -> Result<ApiResponse> {
        {
            let status = &*self.status_rx.borrow();
            if matches!(status, Status::NotConnected | Status::Disconnected(_)) {
                return Err(ConnectError::StatusError(format!(
                    "Invalid status, expect Connected, actual {status:?}"
                ))
                .into());
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

    async fn delete_msg(&self, param: DeleteMsgParams) -> Result<()> {
        delete_msg(self, param).await
    }

    async fn get_msg(&self, param: GetMsgParams) -> Result<GetMsgResult> {
        get_msg(self, param).await
    }

    async fn get_forward_msg(&self, param: GetForwardMsgParams) -> Result<GetForwardMsgResult> {
        get_forward_msg(self, param).await
    }

    #[cfg(feature = "napcat")]
    async fn send_msg_emoji_like(&self, param: SendMsgEmojiLikeParams) -> Result<()> {
        send_msg_emoji_like(self, param).await
    }
}

#[async_trait]
impl Adapter for WsAdapter {}
