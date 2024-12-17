use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::Result;

use crate::{adapter::Caller, plugin::Plugin, schema::Event};

#[derive(Clone)]
pub struct Context {
    pub caller: Arc<dyn Caller>,
    pub event: Arc<Event>,
    pub plugins: Arc<Vec<Plugin>>,
}

pub type Handler = Box<dyn Fn(Context) -> Pin<Box<dyn Future<Output = Result<bool>> + Send>> + Send + Sync>;
