use std::{future::Future, pin::Pin};

use anyhow::Result;

use crate::{adapter::Caller, plugin::Plugin, schema::Event};

pub struct Context<'a> {
    pub caller: &'a dyn Caller,
    pub event: &'a Event,
    pub plugins: &'a Vec<Plugin>,
}

pub type Handler =
    Box<dyn for<'a> Fn(Context<'a>) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'a>> + Send + Sync>;
