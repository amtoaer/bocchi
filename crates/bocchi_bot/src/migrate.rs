use std::sync::{LazyLock, OnceLock};

use crate::model;

use native_db::{Builder, Database, Models};

static MODELS: LazyLock<Models> = LazyLock::new(|| {
    let mut models = Models::new();
    models.define::<model::points::v1::Point>().unwrap();
    models
});

pub fn database() -> &'static Database<'static> {
    static DATABASE: OnceLock<Database<'static>> = OnceLock::new();
    DATABASE.get_or_init(|| {
        let db = Builder::new().create(&MODELS, "./db.native_db").unwrap();
        let rw = db.rw_transaction().unwrap();
        rw.migrate::<model::points::v1::Point>().unwrap();
        rw.commit().unwrap();
        db
    })
}
