//! This module contains all the database resources, parsed as Rust data structures.

pub mod role;
pub mod password_reset;
pub mod user;

pub use role::Role;
pub use password_reset::PasswordResetRequest;
pub use user::User;