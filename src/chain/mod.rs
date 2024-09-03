mod handler;
mod matcher;
mod rule;
pub use handler::Handler;
pub use matcher::Matcher;
pub use rule::Rule;

pub struct MatchUnion {
    pub matcher: Matcher,
    pub handler: Handler,
}

impl MatchUnion {
    pub fn new(matcher: Matcher, handler: Handler) -> Self {
        Self { matcher, handler }
    }
}
