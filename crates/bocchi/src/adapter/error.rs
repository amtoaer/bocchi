use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConnectError {
    #[error("Invalid status: {0}")]
    Status(&'static str),
    #[error("Call api timeout")]
    Timeout,
    #[error("WebSocket error")]
    WebSocket,
}
