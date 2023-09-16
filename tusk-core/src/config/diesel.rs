use std::sync::Arc;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use crate::error::TuskResult;

/// Represents the `diesel` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct Diesel {
    url: Secret<String>
}
impl Diesel {
    pub fn pool(&self) -> TuskResult<Arc<Pool<ConnectionManager<PgConnection>>>> {
        log::debug!("Loading Diesel section");

        let connection_manager = ConnectionManager::new(self.url.expose_secret());
        let database_pool = Pool::new(connection_manager)?;
        Ok(Arc::new(database_pool))
    }
}