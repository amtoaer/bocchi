use anyhow::Result;

use crate::{
    connector::{self, Connector},
    matcher::{Handler, MatchUnion, Matcher},
};

pub struct Bot {
    connector: Box<dyn Connector>,
    match_unions: Vec<MatchUnion>,
}

impl Bot {
    pub async fn connect(address: &str) -> Result<Self> {
        Ok(Bot {
            connector: connector::WsConnector::connect(address).await?,
            match_unions: Vec::new(),
        })
    }

    pub fn on(&mut self, matcher: Matcher, handler: Handler) {
        self.match_unions.push(MatchUnion::new(matcher, handler));
    }

    pub async fn start(&mut self) -> Result<()> {
        self.connector
            .spawn(std::mem::replace(&mut self.match_unions, Vec::new()))
            .await;
        tokio::signal::ctrl_c().await?;
        Ok(())
    }
}
