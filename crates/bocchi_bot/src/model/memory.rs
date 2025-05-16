use bocchi::schema::Sender;
use native_db::*;
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct CachedMessage {
    pub sender: Option<Sender>,
    pub content: String,
}

impl CachedMessage {
    pub fn to_gpt_message(&self) -> serde_json::Value {
        let role = match self.sender {
            Some(_) => "user",
            None => "assistant",
        };
        json!(
            {"role": role, "content": self.content}
        )
    }
}

pub mod v1 {
    use std::collections::VecDeque;

    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    #[native_model(id = 1, version = 1)]
    #[native_db]
    pub struct Memory {
        #[primary_key]
        pub id: String,
        pub history: VecDeque<CachedMessage>,
    }

    impl Memory {
        pub fn new(id: String) -> Self {
            Self {
                id,
                history: VecDeque::new(),
            }
        }
    }
}
