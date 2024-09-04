use crate::{
    adapter::Caller,
    chain::{MatchUnion, Matcher},
    schema::Event,
};
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

#[derive(Default)]
pub struct Plugin(Vec<MatchUnion>);

impl Plugin {
    pub fn new() -> Self {
        Self::default()
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
        self.0
            .push(MatchUnion::new(matcher.into(), Box::new(handler)));
    }

    pub fn into_inner(self) -> Vec<MatchUnion> {
        self.0
    }
}
