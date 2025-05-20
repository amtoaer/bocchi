use std::sync::{LazyLock, OnceLock};

use native_db::{Builder, Database, Models};

use crate::model;

static MODELS: LazyLock<Models> = LazyLock::new(|| {
    let mut models = Models::new();
    models.define::<model::points::v1::Point>().unwrap();
    models.define::<model::memory::v1::Memory>().unwrap();
    models
});

pub fn database() -> &'static Database<'static> {
    static DATABASE: OnceLock<Database<'static>> = OnceLock::new();
    DATABASE.get_or_init(|| Builder::new().create(&MODELS, "./db.native_db").unwrap())
}
