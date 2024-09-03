use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConnectError {
    #[error("Invalid status, must be Connected")]
    StatusError(String),
    #[error("Call api timeout")]
    TimeoutError,
}
