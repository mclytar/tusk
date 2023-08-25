//! This module contains the necessary structures and methods for error handling.

use std::fmt::{Display, Formatter};

/// A `Result` type with a preconfigured error of type [`tusk_core::error::Error`](crate::error::Error).
pub type Result<T> = std::result::Result<T, Error>;
pub use diesel::result::Error as DieselQueryError;

/// Defines the possible errors when starting the Tusk server.
#[derive(Debug)]
pub enum Error {
    /// An error originated while reading from the configuration file.
    ConfigurationFileError(toml::de::Error),
    /// An error originated while trying to connect to the database.
    DatabaseConnectionError(diesel::prelude::ConnectionError),
    /// An error originated while querying the database.
    DatabaseQueryError(diesel::result::Error),
    /// An error originated while performing IO operations.
    IOError(std::io::Error),
    /// An error originated while performing a database migration.
    MigrationError(Box<dyn std::error::Error + Send + Sync>),
    /// An error originated while attempting to create a connection pool.
    R2D2Error(r2d2::Error),
    /// An error originated while attempting to create a secure channel.
    RustlsError(rustls::Error),
    /// An error originated while parsing the Tera templates.
    TeraParseError(tera::Error),
    /// An error originated while starting the server as a Unix daemon.
    #[cfg(unix)]
    UnixError(nix::errno::Errno),
    /// An error originated while starting the server as a Windows service.
    #[cfg(windows)]
    WindowsServiceError(windows_service::Error),
}
impl Error {
    /// Creates an [`Error`] from a boxed migration error.
    pub fn from_migration_error(e: Box<dyn std::error::Error + Send + Sync>) -> Error {
        Error::MigrationError(e)
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ConfigurationFileError(e) => Display::fmt(e, f),
            Error::DatabaseConnectionError(e) => Display::fmt(e, f),
            Error::DatabaseQueryError(e) => Display::fmt(e, f),
            Error::IOError(e) => Display::fmt(e, f),
            Error::MigrationError(e) => Display::fmt(e, f),
            Error::R2D2Error(e) => Display::fmt(e, f),
            Error::RustlsError(e) => Display::fmt(e, f),
            Error::TeraParseError(e) => Display::fmt(e, f),
            #[cfg(unix)]
            Error::UnixError(e) => Display::fmt(e, f),
            #[cfg(windows)]
            Error::WindowsServiceError(e) => Display::fmt(e, f),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ConfigurationFileError(e) => Some(e),
            Error::DatabaseConnectionError(e) => Some(e),
            Error::DatabaseQueryError(e) => Some(e),
            Error::IOError(e) => Some(e),
            Error::MigrationError(e) => Some(e.as_ref()),
            Error::R2D2Error(e) => Some(e),
            Error::RustlsError(e) => Some(e),
            Error::TeraParseError(e) => Some(e),
            #[cfg(unix)]
            Error::UnixError(e) => Some(e),
            #[cfg(windows)]
            Error::WindowsServiceError(e) => Some(e),
        }
    }
}

impl From<toml::de::Error> for Error {
    fn from(value: toml::de::Error) -> Self {
        Error::ConfigurationFileError(value)
    }
}

impl From<diesel::prelude::ConnectionError> for Error {
    fn from(value: diesel::prelude::ConnectionError) -> Self {
        Error::DatabaseConnectionError(value)
    }
}

impl From<diesel::result::Error> for Error {
    fn from(value: diesel::result::Error) -> Self {
        Error::DatabaseQueryError(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IOError(value)
    }
}

impl From<r2d2::Error> for Error {
    fn from(value: r2d2::Error) -> Self {
        Error::R2D2Error(value)
    }
}

impl From<rustls::Error> for Error {
    fn from(value: rustls::Error) -> Self {
        Error::RustlsError(value)
    }
}

impl From<tera::Error> for Error {
    fn from(value: tera::Error) -> Self {
        Error::TeraParseError(value)
    }
}

#[cfg(unix)]
impl From<nix::errno::Errno> for Error {
    fn from(value: nix::errno::Errno) -> Self {
        Error::UnixError(value)
    }
}

#[cfg(windows)]
impl From<windows_service::Error> for Error {
    fn from(value: windows_service::Error) -> Self {
        Error::WindowsServiceError(value)
    }
}

