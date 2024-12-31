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
        "每日签到",
        i32::default(),
        Rule::on_message() & Rule::on_exact_match("#bonus"),
        |ctx| async move {
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
                point.name = nickname.to_owned();
                point.last_update = chrono::Local::now();
                rw.insert(point.clone())?;
            }
            rw.commit()?;
            let msg = if got_point != 0 {
                format!(
                    "本次签到积分：{}\n当前总积分：{}\n最后签到时间：{}",
                    got_point,
                    point.point,
                    point.last_update.format("%Y-%m-%d %H:%M:%S")
                )
            } else {
                "今天已经签到过了，请明天再来～".to_string()
            };
            ctx.caller
                .send_msg(SendMsgParams {
                    user_id: Some(user_id),
                    group_id: ctx.event.try_group_id().ok(),
                    message: MessageContent::Segment(vec![
                        MessageSegment::Reply {
                            id: ctx.event.message_id().to_string(),
                        },
                        MessageSegment::Text { text: msg },
                    ]),
                    auto_escape: true,
                    message_type: None,
                })
                .await?;
            Ok(true)
        },
    );

    plugin.on(
        "查询个人签到分数",
        i32::default(),
        Rule::on_message() & Rule::on_exact_match("#my_bonus"),
        |ctx| async move {
            let user_id = ctx.event.user_id();
            let r = database().r_transaction()?;
            let point: Option<Point> = r.get().primary(user_id)?;
            drop(r);
            let msg = match point {
                Some(point) => format!(
                    "当前总积分：{}\n最后签到时间：{}",
                    point.point,
                    point.last_update.format("%Y-%m-%d %H:%M:%S")
                ),
                None => "你还没有签到过哦，发送 #bonus 进行第一次签到吧！".to_string(),
            };
            ctx.caller
                .send_msg(SendMsgParams {
                    user_id: Some(user_id),
                    group_id: ctx.event.try_group_id().ok(),
                    message: MessageContent::Segment(vec![
                        MessageSegment::Reply {
                            id: ctx.event.message_id().to_string(),
                        },
                        MessageSegment::Text { text: msg },
                    ]),
                    auto_escape: true,
                    message_type: None,
                })
                .await?;
            Ok(true)
        },
    );

    plugin
}
