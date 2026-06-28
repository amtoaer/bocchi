use std::{sync::LazyLock, time::Duration};

use reqwest::{Client, ClientBuilder};

pub static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    ClientBuilder::new()
        .timeout(Duration::from_mins(3))
        .zstd(true)
        .gzip(true)
        .deflate(true)
        .brotli(true)
        .build()
        .unwrap()
});
