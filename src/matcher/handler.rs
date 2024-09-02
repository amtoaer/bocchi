use crate::schema::Event;

use anyhow::Result;
use std::{future::Future, pin::Pin};

pub type Handler = Box<dyn Fn(&Event) -> Pin<Box<dyn Future<Output = Result<()>>>> + Send + Sync>;
