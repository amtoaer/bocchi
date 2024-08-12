use thiserror::Error;

use crate::schema::ResponseBody;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Invalid Response Type: {0:?}")]
    ResponseTypeError(ResponseBody),
}
