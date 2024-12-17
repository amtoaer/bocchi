use std::{collections::HashSet, sync::Arc};

use anyhow::Ok;
use bocchi::{
    chain::Rule,
    plugin::Plugin,
    schema::{MessageContent, MessageSegment, SendMsgParams},
};
use dashmap::DashMap;

const THRESHOLD: usize = 2; // 复读的重复阈值，必须大于等于 1

struct Repeat {
    text: String,
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
                // 因为 on_group_message 限制，group_id unwrap 是安全的
                let (user_id, group_id) = (ctx.event.user_id(), ctx.event.group_id().unwrap());
                let text = match ctx.event.message() {
                    MessageContent::Text(text) => text.to_string(),
                    MessageContent::Segment(segments) => {
                        if segments.iter().any(|msg| !matches!(msg, MessageSegment::Text { .. })) {
                            // 打个补丁，暂时忽略非纯文本消息
                            "".to_string()
                        } else {
                            ctx.event.plain_text().to_string()
                        }
                    }
                };
                // 注意 text 不再过滤空文本，因为穿插图片等消息应该被视作打断复读
                let map_entry = map_clone.entry(group_id);
                let mut repeat = map_entry.or_insert_with(|| Repeat {
                    text: text.clone(),
                    users: HashSet::from([user_id]),
                    repeated: false,
                });
                if repeat.text != text {
                    // 文本不同，重置复读
                    repeat.text = text;
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
                            ctx.caller
                                .send_msg(SendMsgParams {
                                    user_id: None,
                                    group_id: Some(group_id),
                                    message: MessageContent::Text(text),
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
                Ok(false)
            }
        },
    );

    plugin
}
