use anyhow::{bail, Result};

use crate::{
    adapter::{self, Adapter},
    chain::{Handler, MatchUnion, Matcher},
    plugin::Plugin,
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

    pub fn on(&mut self, matcher: impl Into<Matcher>, handler: Handler) {
        self.match_unions
            .push(MatchUnion::new(matcher.into(), handler));
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
