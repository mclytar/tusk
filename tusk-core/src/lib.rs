//! This crate contains the core items needed to run the server.

#![warn(missing_docs)]

pub mod config;
pub mod error;
pub mod resources;
#[allow(missing_docs)]
pub mod schema;
pub mod session;

pub use diesel::PgConnection;
pub use diesel::Connection;
use diesel::r2d2::ConnectionManager;
pub use diesel::result::Error as DieselError;
pub use lettre::Message;
use r2d2::PooledConnection;

/// Identifies a pooled `PgConnection`.
pub type PooledPgConnection = PooledConnection<ConnectionManager<PgConnection>>;

/// Contains all the test utilities that can be used to set up a test server.
pub mod test {
    pub use ::diesel_migrations::{self, EmbeddedMigrations, embed_migrations, MigrationHarness};
}