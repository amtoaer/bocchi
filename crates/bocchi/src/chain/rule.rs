use std::{borrow::Cow, ops};

use crate::{
    chain::Matcher,
    schema::{Event, Sender},
};

#[allow(clippy::enum_variant_names)]
pub enum InnerRule {
    OnEventStatic(&'static (dyn Fn(&Event) -> bool + Send + Sync)),
    OnText(Box<dyn Fn(&str) -> bool + Send + Sync>),
    OnSender(Box<dyn Fn(&Sender) -> bool + Send + Sync>),
    OnEvent(Box<dyn Fn(&Event) -> bool + Send + Sync>),
}

pub struct Rule {
    pub(crate) name: Cow<'static, str>,
    pub(crate) inner: InnerRule,
}

impl Rule {
    pub fn on_message() -> Rule {
        Self {
            name: "on_message".into(),
            inner: InnerRule::OnEventStatic(&|event: &Event| -> bool {
                matches!(event, Event::GroupMessage(_) | Event::PrivateMessage(_))
            }),
        }
    }

    pub fn on_group_message() -> Rule {
        Self {
            name: "on_group_message".into(),
            inner: InnerRule::OnEventStatic(&|event: &Event| -> bool { matches!(event, Event::GroupMessage(_)) }),
        }
    }

    pub fn on_private_message() -> Rule {
        Self {
            name: "on_private_message".into(),
            inner: InnerRule::OnEventStatic(&|event: &Event| -> bool { matches!(event, Event::PrivateMessage(_)) }),
        }
    }

    pub fn on_sender_id(user_id: i64) -> Rule {
        Self {
            name: format!("on_sender_id({user_id})").into(),
            inner: InnerRule::OnSender(Box::new(move |sender: &Sender| -> bool {
                sender.user_id == Some(user_id)
            })),
        }
    }

    pub fn on_group_id(group_id: u64) -> Rule {
        Self {
            name: format!("on_group_id({group_id})").into(),
            inner: InnerRule::OnEvent(Box::new(move |event: &Event| -> bool {
                matches!(event, Event::GroupMessage(e) if e.group_id == group_id)
            })),
        }
    }

    fn on_text(name: Cow<'static, str>, is_valid: impl Fn(&str) -> bool + Send + Sync + 'static) -> Rule {
        Self {
            name,
            inner: InnerRule::OnText(Box::new(move |text| is_valid(text.trim()))),
        }
    }

    pub fn on_exact_match(str: &'static str) -> Rule {
        Self::on_text(format!("on_exact_match({str})").into(), move |text| {
            error!("on_exact_match: text = \"{text}\", str = \"{str}\"");
            text == str.trim()
        })
    }

    pub fn on_prefix(prefix: &'static str) -> Rule {
        Self::on_text(format!("on_prefix({prefix})").into(), move |text| {
            text.starts_with(prefix.trim())
        })
    }

    pub fn on_suffix(suffix: &'static str) -> Rule {
        Self::on_text(format!("on_suffix({suffix})").into(), move |text| {
            text.ends_with(suffix.trim())
        })
    }
}

impl ops::BitAnd<Rule> for Rule {
    type Output = Matcher;

    fn bitand(self, rhs: Rule) -> Matcher {
        Matcher {
            condition: vec![self, rhs],
        }
    }
}
