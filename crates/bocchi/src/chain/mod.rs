mod handler;
mod matcher;
mod rule;
use std::borrow::Cow;

pub use handler::{Context, Handler};
pub use matcher::Matcher;
pub use rule::Rule;

pub struct MatchUnion {
    pub matcher: Matcher,
    pub handler: Handler,
    pub description: Cow<'static, str>,
    pub priority: i32,
}

impl MatchUnion {
    pub fn new(description: Cow<'static, str>, matcher: Matcher, handler: Handler) -> Self {
        Self {
            description,
            matcher,
            handler,
            priority: 0,
        }
    }
}
