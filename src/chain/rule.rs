use std::ops;

use crate::{
    chain::Matcher,
    schema::{Event, MessageContent, Sender},
};

#[allow(clippy::enum_variant_names)]
pub enum Rule {
    OnType(&'static (dyn Fn(&Event) -> bool + Send + Sync)),
    OnText(Box<dyn Fn(&MessageContent) -> bool + Send + Sync>),
    OnSender(Box<dyn Fn(&Sender) -> bool + Send + Sync>),
    OnField(Box<dyn Fn(&Event) -> bool + Send + Sync>),
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

    pub fn on_group_id(group_id: u64) -> Rule {
        Rule::OnField(Box::new(move |event: &Event| -> bool {
            matches!(event, Event::GroupMessage(e) if e.group_id == group_id)
        }))
    }

    fn on_text(is_valid: impl Fn(&str) -> bool + Send + Sync + 'static) -> Rule {
        Rule::OnText(Box::new(move |message_type: &MessageContent| -> bool {
            let raw = message_type.raw();
            is_valid(raw.trim())
        }))
    }

    pub fn on_exact_match(str: &'static str) -> Rule {
        Self::on_text(move |text| text == str.trim())
    }

    pub fn on_prefix(prefix: &'static str) -> Rule {
        Self::on_text(move |text| text.starts_with(prefix.trim()))
    }

    pub fn on_suffix(suffix: &'static str) -> Rule {
        Self::on_text(move |text| text.ends_with(suffix.trim()))
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
