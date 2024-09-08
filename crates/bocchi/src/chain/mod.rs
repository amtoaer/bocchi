mod handler;
mod matcher;
mod rule;
use std::borrow::Cow;

pub use handler::{Context, Handler};
pub use matcher::Matcher;
pub use rule::Rule;

pub struct MatchUnion {
    pub description: Cow<'static, str>,
    pub priority: i32,
    pub matcher: Matcher,
    pub handler: Handler,
}

impl MatchUnion {
    pub fn new(description: Cow<'static, str>, priority: i32, matcher: Matcher, handler: Handler) -> Self {
        Self {
            description,
            priority,
            matcher,
            handler,
        }
    }
}
