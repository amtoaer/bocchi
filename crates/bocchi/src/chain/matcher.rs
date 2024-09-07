use std::{fmt::Display, ops};

use crate::{
    chain::{rule::InnerRule, Rule},
    schema::Event,
};

#[derive(Default)]
pub struct Matcher {
    pub condition: Vec<Rule>,
}

impl Display for Matcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for rule in self.condition.iter().take(self.condition.len() - 1) {
            write!(f, "{} & ", rule.name)?;
        }
        if let Some(rule) = self.condition.last() {
            write!(f, "{}", rule.name)?;
        }
        Ok(())
    }
}

impl Matcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, rules: Vec<Rule>) {
        self.condition.extend(rules);
    }

    pub fn is_match(&self, event: &Event) -> bool {
        for rule in &self.condition {
            match &rule.inner {
                InnerRule::OnText(handler) => {
                    if !handler(&event.plain_text()) {
                        return false;
                    }
                }
                InnerRule::OnSender(handler) => {
                    if !handler(event.sender()) {
                        return false;
                    }
                }
                InnerRule::OnEventStatic(handler) => {
                    if !handler(event) {
                        return false;
                    }
                }
                InnerRule::OnEvent(handler) => {
                    if !handler(event) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

impl ops::BitAnd<Matcher> for Matcher {
    type Output = Self;

    fn bitand(self, rhs: Matcher) -> Self::Output {
        Self {
            condition: self.condition.into_iter().chain(rhs.condition).collect(),
        }
    }
}

impl ops::BitAnd<Rule> for Matcher {
    type Output = Self;

    fn bitand(mut self, rhs: Rule) -> Self::Output {
        self.add(vec![rhs]);
        self
    }
}

impl From<Rule> for Matcher {
    fn from(rule: Rule) -> Self {
        Self { condition: vec![rule] }
    }
}
