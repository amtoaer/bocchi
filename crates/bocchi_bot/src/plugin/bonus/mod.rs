use bocchi::{
    chain::Rule,
    plugin::Plugin,
    schema::{MessageContent, MessageSegment, SendMsgParams},
};
use rand::Rng;

use crate::{migrate::database, model::points::v1::Point};

pub fn bonus_plugin() -> Plugin {
    let mut plugin = Plugin::new();

    plugin.on(
        Rule::on_message() & Rule::on_exact_match("#bonus"),
        |caller, event| {
            Box::pin(async move {
                let (user_id, nickname) = (event.user_id(), event.nickname());
                let res = tokio::task::spawn_blocking(move || {
                    let mut got_point = 0;
                    let db = database();
                    let rw = db.rw_transaction()?;
                    let point: Option<Point> = rw.get().primary(user_id)?;
                    match point {
                        Some(mut point) => {
                            if point.last_update.date_naive() != chrono::Local::now().date_naive() {
                                got_point = rand::thread_rng().gen_range(1..=100);
                                point.point += got_point;
                                point.name = nickname.to_owned();
                                point.last_update = chrono::Local::now();
                                rw.insert(point)?;
                            }
                        }
                        None => {
                            got_point = rand::thread_rng().gen_range(1..=100);
                            let point = Point {
                                id: user_id,
                                name: nickname.to_owned(),
                                point: got_point,
                                last_update: chrono::Local::now(),
                            };
                            rw.insert(point)?;
                        }
                    }
                    rw.commit()?;
                    Ok::<_, anyhow::Error>(got_point)
                })
                .await?;
                let (msg, ret) = match res {
                    Err(e) => (" 签到失败，请重试".to_string(), Err(e)),
                    Ok(got_point) => {
                        let msg = if got_point != 0 {
                            format!(" 签到成功，获得 {} 点数", got_point)
                        } else {
                            " 今天已经签到过了，请明天再来".to_string()
                        };
                        (msg, Ok(()))
                    }
                };
                caller
                    .send_msg(SendMsgParams {
                        user_id: Some(user_id),
                        group_id: event.group_id(),
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
                ret
            })
        },
    );

    plugin
}
