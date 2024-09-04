use crate::chain::{Handler, MatchUnion, Matcher};

#[derive(Default)]
pub struct Plugin(Vec<MatchUnion>);

impl Plugin {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on(&mut self, matcher: impl Into<Matcher>, handler: Handler) {
        self.0.push(MatchUnion::new(matcher.into(), handler));
    }

    pub fn into_inner(self) -> Vec<MatchUnion> {
        self.0
    }
}
