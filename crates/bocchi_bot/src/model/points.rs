use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};

pub mod v1 {
    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
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
}
