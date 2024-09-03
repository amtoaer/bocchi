use std::{cmp::max, ops};

use crate::{chain::Rule, schema::Event};

pub struct Matcher {
    pub condition: Vec<Rule>,
    pub priority: u8,
}

impl Matcher {
    pub fn new() -> Self {
        Self {
            condition: Vec::new(),
            priority: 0,
        }
    }

    pub fn set_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn add(&mut self, rules: Vec<Rule>) {
        self.condition.extend(rules);
    }

    pub fn is_match(&self, event: &Event) -> bool {
        for rule in &self.condition {
            match rule {
                Rule::OnText(handler) => {
                    if !handler(event.raw_message()) {
                        return false;
                    }
                }
                Rule::OnMessage(handler) => {
                    if !handler(event.message()) {
                        return false;
                    }
                }
                Rule::OnSender(handler) => {
                    if !handler(event.sender()) {
                        return false;
                    }
                }
                Rule::OnEventStatic(handler) => {
                    if !handler(event) {
                        return false;
                    }
                }
                Rule::OnEvent(handler) => {
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
            priority: max(self.priority, rhs.priority),
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
        Self {
            condition: vec![rule],
            priority: 0,
        }
    }
}
