use std::ops;

use crate::{
    matcher::Matcher,
    schema::{Event, MessageSegment, MessageType, Sender},
};

pub enum Rule {
    OnType(&'static (dyn Fn(&Event) -> bool + Send + Sync)),
    OnText(Box<dyn Fn(&MessageType) -> bool + Send + Sync>),
    OnSender(Box<dyn Fn(&Sender) -> bool + Send + Sync>),
}

impl Rule {
    pub fn on_group_message() -> Rule {
        Rule::OnType(&|event: &Event| -> bool {
            return matches!(event, Event::GroupMessage(_));
        })
    }

    pub fn on_private_message() -> Rule {
        Rule::OnType(&|event: &Event| -> bool {
            return matches!(event, Event::PrivateMessage(_));
        })
    }

    pub fn on_sender_id(user_id: i64) -> Rule {
        Rule::OnSender(Box::new(move |sender: &Sender| -> bool {
            return sender.user_id == Some(user_id);
        }))
    }

    fn on_exact_match(str: &'static str) -> Rule {
        Rule::OnText(Box::new(move |message_type: &MessageType| -> bool {
            match message_type {
                MessageType::Text(text) => return text == str,
                MessageType::Segment(segments) => {
                    for segment in segments {
                        if let MessageSegment::Text { text } = segment {
                            if text == str {
                                return true;
                            }
                        }
                    }
                    return false;
                }
            }
        }))
    }

    fn on_prefix(prefix: &'static str) -> Rule {
        Rule::OnText(Box::new(move |message_type: &MessageType| -> bool {
            match message_type {
                MessageType::Text(text) => return text.starts_with(&prefix),
                MessageType::Segment(segments) => {
                    for segment in segments {
                        if let MessageSegment::Text { text } = segment {
                            if text.starts_with(prefix) {
                                return true;
                            }
                        }
                    }
                    return false;
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
