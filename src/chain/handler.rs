use crate::{adapter::Caller, schema::Event};

use anyhow::Result;
use std::{future::Future, pin::Pin};

pub type Handler = Box<
    dyn for<'a> Fn(
            &'a dyn Caller,
            &'a Event,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
        + Send
        + Sync,
>;
