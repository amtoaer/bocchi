use std::{collections::HashSet, sync::Arc};

use anyhow::Ok;
use bocchi::{
    chain::Rule,
    plugin::Plugin,
    schema::{MessageContent, SendMsgParams},
};
use dashmap::DashMap;

const THRESHOLD: usize = 2; // 复读的重复阈值，必须大于等于 1

struct Repeat {
    text: String,
    count: HashSet<u64>,
    repeated: bool,
}

pub fn repeat_plugin() -> Plugin {
    let mut plugin = Plugin::new();
    let map = Arc::new(DashMap::new());

    plugin.on(Rule::on_group_message(), move |caller, event| {
        let map_clone = map.clone();
        Box::pin(async move {
            // 因为 on_group_message 限制，group_id unwrap 是安全的
            let (user_id, group_id) = (event.user_id(), event.group_id().unwrap());
            let text = event.plain_text();
            if text.is_empty() {
                return Ok(());
            }
            let map_entry = map_clone.entry(group_id);
            let mut repeat = map_entry.or_insert_with(|| Repeat {
                text: text.to_string(),
                count: HashSet::from([user_id]),
                repeated: false,
            });
            if repeat.text != text {
                // 文本不同，重置复读
                repeat.text = text.to_string();
                repeat.count = HashSet::from([user_id]);
                repeat.repeated = false;
            } else if !repeat.repeated {
                // 文本相同且未复读的情况下，插入用户 ID 并检查是否达到阈值
                repeat.count.insert(user_id);
                if repeat.count.len() >= THRESHOLD {
                    caller
                        .send_msg(SendMsgParams {
                            user_id: None,
                            group_id: Some(group_id),
                            message: MessageContent::Text(text.to_string()),
                            auto_escape: true,
                            message_type: None,
                        })
                        .await?;
                    repeat.repeated = true;
                }
            };
            Ok(())
        })
    });

    plugin
}
