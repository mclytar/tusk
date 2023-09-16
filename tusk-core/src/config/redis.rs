use actix_session::storage::RedisSessionStore;
use actix_web::rt::System;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use crate::error::TuskResult;

/// Represents the `redis` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct Redis {
    url: Secret<String>
}
impl Redis {
    pub fn session_storage(&self) -> TuskResult<RedisSessionStore> {
        let url = self.url.clone();
        let storage = std::thread::spawn(move || System::new().block_on(RedisSessionStore::new(url.expose_secret()))
        ).join().unwrap()?;

        Ok(storage)
    }
}