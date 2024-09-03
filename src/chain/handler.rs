use crate::{connector::Connector, schema::Event};

use anyhow::Result;
use std::{future::Future, pin::Pin};

pub type Handler = Box<
    dyn for<'a> Fn(
            &'a dyn Connector,
            &'a Event,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
        + Send
        + Sync,
>;
