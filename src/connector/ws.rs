use std::{str::FromStr, sync::Arc};

use crate::{
    connector::{error::ConnectError, Connector, Status},
    schema::{ApiRequest, ApiResponse, Event},
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
pub struct WsConnector {
    ws_sink: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    ws_stream: Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    request_tx: Option<tokio::sync::mpsc::Sender<ApiRequest>>,
    request_recorder: Arc<DashMap<u64, Sender<ApiResponse>>>,
    status_tx: tokio::sync::watch::Sender<Status>,
    status_rx: tokio::sync::watch::Receiver<Status>,
}

impl WsConnector {
    pub async fn connect(address: &str) -> Result<Box<Self>> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(Uri::from_str(address)?).await?;
        let (ws_sink, ws_stream) = ws_stream.split();
        let (status_tx, status_rx) = tokio::sync::watch::channel(Status::NotConnected);
        Ok(Box::new(WsConnector {
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
impl Connector for WsConnector {
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

    async fn spawn(&mut self) {
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
                    Ok::<(), anyhow::Error>(())
                }.await;
                let _ = status_tx.send(Status::Disconnected(e.err()));
            });

            let request_recorder = self.request_recorder.clone();
            let (mut status_rx, status_tx) = (self.status_rx.clone(), self.status_tx.clone());
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
                                        if let Ok(msg) = serde_json::from_str::<ApiResponse>(&text){
                                            if let Some((_, tx)) = request_recorder.remove(&msg.echo()) {
                                                if let Err(e) = tx.send(msg) {
                                                    error!("Failed to send response: {e:?}");
                                                }
                                            } else {
                                                error!("Received response with unknown request ID: {text}");
                                            }
                                        } else if let Ok(msg) = serde_json::from_str::<Event>(&text) {
                                            info!("Receive event: {msg:?}");
                                        } else {
                                            warn!("Receive unknown message: {text}");
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Ok::<(), anyhow::Error>(())
                }.await;
                let _ = status_tx.send(Status::Disconnected(e.err()));
                Ok::<(), anyhow::Error>(())
            });
        }
    }
}
