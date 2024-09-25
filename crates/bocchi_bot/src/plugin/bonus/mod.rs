use bocchi::{
    chain::Rule,
    plugin::Plugin,
    schema::{MessageContent, MessageSegment, SendMsgParams},
};
use rand::Rng;

use crate::{migrate::database, model::points::v1::Point};

pub fn bonus_plugin() -> Plugin {
    let mut plugin = Plugin::new("每日签到插件", "每日签到获取积分");

    plugin.on(
        "签到逻辑的处理",
        i32::default(),
        Rule::on_message() & Rule::on_exact_match("#bonus"),
        |ctx| {
            Box::pin(async move {
                let (user_id, nickname) = (ctx.event.user_id(), ctx.event.nickname());
                let rw = database().rw_transaction()?;
                let mut point: Point = rw
                    .get()
                    .primary(user_id)?
                    .unwrap_or_else(|| Point::new(user_id, nickname.to_owned()));
                let mut got_point = 0;
                if point.last_update.date_naive() != chrono::Local::now().date_naive() {
                    got_point = rand::thread_rng().gen_range(1..=100);
                    point.point += got_point;
                    point.name = nickname;
                    point.last_update = chrono::Local::now();
                    rw.insert(point)?;
                }
                rw.commit()?;
                let msg = if got_point != 0 {
                    format!(" 签到成功，获得 {} 点数", got_point)
                } else {
                    " 今天已经签到过了，请明天再来".to_string()
                };
                ctx.caller
                    .send_msg(SendMsgParams {
                        user_id: Some(user_id),
                        group_id: ctx.event.group_id(),
                        message: MessageContent::Segment(vec![
                            MessageSegment::At {
                                qq: user_id.to_string(),
                            },
                            MessageSegment::Text { text: msg },
                        ]),
                        auto_escape: true,
                        message_type: None,
                    })
                    .await?;
                Ok(true)
            })
        },
    );

    plugin
}
