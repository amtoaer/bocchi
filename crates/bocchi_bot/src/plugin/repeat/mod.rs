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
    users: HashSet<u64>,
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
            // FIXME: 一个临时 workaround，避免与其它命令产生冲突，后续支持 handler 打断后再移除
            if text.trim().starts_with("#") {
                return Ok(());
            }
            // 注意 text 不再过滤空文本，因为穿插图片等消息应该被视作打断复读
            let map_entry = map_clone.entry(group_id);
            let mut repeat = map_entry.or_insert_with(|| Repeat {
                text: text.to_string(),
                users: HashSet::from([user_id]),
                repeated: false,
            });
            if repeat.text != text {
                // 文本不同，重置复读
                repeat.text = text.to_string();
                repeat.users.clear();
                repeat.users.insert(user_id);
                repeat.repeated = false;
            } else if !repeat.repeated {
                // 文本相同且未复读过，尝试记录用户
                repeat.users.insert(user_id);
                // 复读相同文本的用户数达到阈值
                if repeat.users.len() >= THRESHOLD {
                    // 且不是在一直斗图导致机器人识别为空文本复读的情况
                    if !text.is_empty() {
                        // 进行一次复读
                        caller
                            .send_msg(SendMsgParams {
                                user_id: None,
                                group_id: Some(group_id),
                                message: MessageContent::Text(text.to_string()),
                                auto_escape: true,
                                message_type: None,
                            })
                            .await?;
                    }
                    // repeated 标记其实可以省略，仅根据 users 数量判断是否复读
                    // 引入 repeated 是为了在复读后清空 users 以节省空间
                    repeat.users.clear();
                    repeat.repeated = true;
                }
            };
            Ok(())
        })
    });

    plugin
}
