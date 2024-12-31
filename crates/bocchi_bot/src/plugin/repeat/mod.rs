use std::{collections::HashSet, sync::Arc};

use anyhow::Ok;
use bocchi::{
    chain::Rule,
    plugin::Plugin,
    schema::{MessageContent, MessageSegment, SendMsgParams},
};
use dashmap::DashMap;

const THRESHOLD: usize = 2; // 复读的重复阈值，必须大于 1

struct Repeat {
    message: Option<MessageContent>,
    users: HashSet<u64>,
    repeated: bool,
}

pub fn repeat_plugin() -> Plugin {
    let mut plugin = Plugin::new("复读插件", format!("连续文本达到 {} 次时自动复读", THRESHOLD));
    let map = Arc::new(DashMap::new());

    plugin.on(
        format!("检测是否满足连续 {THRESHOLD} 条消息"),
        i32::MIN, // 复读插件的优先级需要很低，避免其它命令被复读响应
        Rule::on_group_message(),
        move |ctx| {
            let map_clone = map.clone();
            async move {
                let (user_id, group_id) = (ctx.event.user_id(), ctx.event.group_id());
                let message = ctx.event.message();
                let message = match message {
                    MessageContent::Text(_) => Some(message),
                    MessageContent::Segment(segments)
                        if segments
                            .iter()
                            .all(|msg| matches!(msg, MessageSegment::Text { .. } | MessageSegment::Face { .. })) =>
                    {
                        Some(message)
                    }
                    _ => None,
                };
                let map_entry = map_clone.entry(group_id);
                let mut repeat = map_entry.or_insert_with(|| Repeat {
                    message: message.cloned(),
                    users: HashSet::from([user_id]),
                    repeated: false,
                });
                if repeat.message.as_ref() != message {
                    *repeat = Repeat {
                        message: message.cloned(),
                        users: HashSet::from([user_id]),
                        repeated: false,
                    };
                } else if !repeat.repeated {
                    repeat.users.insert(user_id);
                    if repeat.users.len() >= THRESHOLD {
                        if let Some(inner_message) = &repeat.message {
                            ctx.caller
                                .send_msg(SendMsgParams {
                                    user_id: None,
                                    group_id: Some(group_id),
                                    message: inner_message.clone(),
                                    auto_escape: true,
                                    message_type: None,
                                })
                                .await?;
                        }
                        repeat.users.clear();
                        repeat.repeated = true;
                    }
                };
                Ok(false)
            }
        },
    );

    plugin
}
