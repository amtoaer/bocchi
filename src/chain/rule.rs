use std::ops;

use crate::{
    chain::Matcher,
    schema::{Event, MessageContent, MessageSegment, Sender},
};

#[allow(clippy::enum_variant_names)]
pub enum Rule {
    OnType(&'static (dyn Fn(&Event) -> bool + Send + Sync)),
    OnText(Box<dyn Fn(&MessageContent) -> bool + Send + Sync>),
    OnSender(Box<dyn Fn(&Sender) -> bool + Send + Sync>),
}

impl Rule {
    pub fn on_group_message() -> Rule {
        Rule::OnType(&|event: &Event| -> bool { matches!(event, Event::GroupMessage(_)) })
    }

    pub fn on_private_message() -> Rule {
        Rule::OnType(&|event: &Event| -> bool { matches!(event, Event::PrivateMessage(_)) })
    }

    pub fn on_sender_id(user_id: i64) -> Rule {
        Rule::OnSender(Box::new(move |sender: &Sender| -> bool {
            sender.user_id == Some(user_id)
        }))
    }

    fn on_exact_match(str: &'static str) -> Rule {
        Rule::OnText(Box::new(move |message_type: &MessageContent| -> bool {
            match message_type {
                MessageContent::Text(text) => text == str,
                MessageContent::Segment(segments) => {
                    for segment in segments {
                        if let MessageSegment::Text { text } = segment {
                            if text == str {
                                return true;
                            }
                        }
                    }
                    false
                }
            }
        }))
    }

    fn on_prefix(prefix: &'static str) -> Rule {
        Rule::OnText(Box::new(move |message_type: &MessageContent| -> bool {
            match message_type {
                MessageContent::Text(text) => text.starts_with(prefix),
                MessageContent::Segment(segments) => {
                    for segment in segments {
                        if let MessageSegment::Text { text } = segment {
                            if text.starts_with(prefix) {
                                return true;
                            }
                        }
                    }
                    false
                }
            }
        }))
    }
}

impl ops::BitAnd<Rule> for Rule {
    type Output = Matcher;

    fn bitand(self, rhs: Rule) -> Matcher {
        Matcher {
            condition: vec![self, rhs],
            priority: 0,
        }
    }
}
