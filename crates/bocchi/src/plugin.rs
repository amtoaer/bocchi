use std::{borrow::Cow, future::Future, pin::Pin, sync::Arc};

use anyhow::Result;

use crate::chain::{Context, MatchUnion, Matcher};

pub struct Plugin {
    pub name: Cow<'static, str>,
    pub description: Cow<'static, str>,
    match_unions: Vec<Arc<MatchUnion>>,
}

impl Plugin {
    pub fn new(name: impl Into<Cow<'static, str>>, description: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            match_unions: Vec::new(),
        }
    }

    pub fn on<D, M, H>(&mut self, description: D, priority: i32, matcher: M, handler: H)
    where
        D: Into<Cow<'static, str>>,
        M: Into<Matcher>,
        H: for<'a> Fn(Context<'a>) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'a>> + Send + Sync + 'static,
    {
        self.match_unions.push(Arc::new(MatchUnion::new(
            description.into(),
            priority,
            matcher.into(),
            Box::new(handler),
        )));
    }

    pub(crate) fn match_unions(&self) -> &[Arc<MatchUnion>] {
        &self.match_unions
    }
}
