use std::{sync::LazyLock, time::Duration};

use reqwest::{Client, ClientBuilder};

pub static HTTP_CLIENT: LazyLock<Client> =
    LazyLock::new(|| ClientBuilder::new().timeout(Duration::from_secs(20)).build().unwrap());
