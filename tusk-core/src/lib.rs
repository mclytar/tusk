//! This crate contains the core items needed to run the server.

#![warn(missing_docs)]

pub mod config;
pub mod error;
pub mod os;
pub mod resources;
#[allow(missing_docs)]
pub mod schema;

pub use diesel::PgConnection;
pub use diesel::result::Error as DieselError;