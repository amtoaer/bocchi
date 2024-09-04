use anyhow::{bail, Result};
use std::future::Future;
use std::pin::Pin;

use crate::{
    adapter::{self, Adapter, Caller},
    chain::{MatchUnion, Matcher},
    plugin::Plugin,
    schema::Event,
};

pub struct Bot {
    adapter: Option<Box<dyn Adapter>>,
    match_unions: Vec<MatchUnion>,
}

impl Bot {
    pub async fn connect(address: &str) -> Result<Self> {
        Ok(Bot {
            adapter: Some(adapter::WsAdapter::connect(address).await?),
            match_unions: Vec::new(),
        })
    }

    pub fn on<M, H>(&mut self, matcher: M, handler: H)
    where
        M: Into<Matcher>,
        H: for<'a> Fn(
                &'a dyn Caller,
                &'a Event,
            ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
            + Send
            + Sync
            + 'static,
    {
        self.match_unions
            .push(MatchUnion::new(matcher.into(), Box::new(handler)));
    }

    pub fn register_plugin(&mut self, plugin: Plugin) {
        self.match_unions.extend(plugin.into_inner());
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.adapter.is_none() {
            bail!("bot already started!");
        }
        // 已经断言过，这里 unwrap 是安全的
        let connector = self.adapter.take().unwrap();
        let mut match_unions = std::mem::take(&mut self.match_unions);
        match_unions.sort_by_key(|u| u.matcher.priority);
        connector.spawn(match_unions).await;
        info!("bot started!");
        Ok(tokio::signal::ctrl_c().await?)
    }
}
