use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};

pub mod v1 {
    use chrono::Days;

    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    #[native_model(id = 1, version = 1)]
    #[native_db]
    pub struct Point {
        #[primary_key]
        pub id: u64,
        pub name: String,
        #[secondary_key]
        pub point: u64,
        pub last_update: chrono::DateTime<chrono::Local>,
    }

    impl Point {
        pub fn new(id: u64, name: String) -> Self {
            Self {
                id,
                name,
                point: 0,
                // 此处将最新更新时间设置为昨天，以便第一次签到正常触发
                last_update: chrono::Local::now().checked_sub_days(Days::new(1)).unwrap(),
            }
        }
    }
}
