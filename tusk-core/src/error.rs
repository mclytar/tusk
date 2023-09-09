//! This module contains the necessary structures and methods for error handling.

use std::fmt::{Display, Formatter};

/// A `Result` type with a preconfigured error of type [`tusk_core::error::Error`](crate::error::TuskError).
pub type TuskResult<T> = std::result::Result<T, TuskError>;
pub use diesel::result::Error as DieselQueryError;

/// Defines the possible errors when starting the Tusk server.
#[derive(Debug)]
pub enum TuskError {
    /// None of the files given in the configuration list has been found.
    ConfigurationNotFound,
    /// The certificate file does not contain any certificate.
    CertificatesNotFound,
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
impl TuskError {
    /// Creates an [`TuskError`] from a boxed migration error.
    pub fn from_migration_error(e: Box<dyn std::error::Error + Send + Sync>) -> TuskError {
        TuskError::MigrationError(e)
    }
}
impl Display for TuskError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TuskError::ConfigurationNotFound => write!(f, "None of the given configuration files has been found."),
            TuskError::CertificatesNotFound => write!(f, "The certificate file does not contain any certificate."),
            TuskError::ConfigurationFileError(e) => Display::fmt(e, f),
            TuskError::DatabaseConnectionError(e) => Display::fmt(e, f),
            TuskError::DatabaseQueryError(e) => Display::fmt(e, f),
            TuskError::IOError(e) => Display::fmt(e, f),
            TuskError::MigrationError(e) => Display::fmt(e, f),
            TuskError::R2D2Error(e) => Display::fmt(e, f),
            TuskError::RustlsError(e) => Display::fmt(e, f),
            TuskError::TeraParseError(e) => Display::fmt(e, f),
            #[cfg(unix)]
            TuskError::UnixError(e) => Display::fmt(e, f),
            #[cfg(windows)]
            TuskError::WindowsServiceError(e) => Display::fmt(e, f),
        }
    }
}
impl std::error::Error for TuskError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TuskError::ConfigurationNotFound => None,
            TuskError::CertificatesNotFound => None,
            TuskError::ConfigurationFileError(e) => Some(e),
            TuskError::DatabaseConnectionError(e) => Some(e),
            TuskError::DatabaseQueryError(e) => Some(e),
            TuskError::IOError(e) => Some(e),
            TuskError::MigrationError(e) => Some(e.as_ref()),
            TuskError::R2D2Error(e) => Some(e),
            TuskError::RustlsError(e) => Some(e),
            TuskError::TeraParseError(e) => Some(e),
            #[cfg(unix)]
            TuskError::UnixError(e) => Some(e),
            #[cfg(windows)]
            TuskError::WindowsServiceError(e) => Some(e),
        }
    }
}

impl From<toml::de::Error> for TuskError {
    fn from(value: toml::de::Error) -> Self {
        TuskError::ConfigurationFileError(value)
    }
}

impl From<diesel::prelude::ConnectionError> for TuskError {
    fn from(value: diesel::prelude::ConnectionError) -> Self {
        TuskError::DatabaseConnectionError(value)
    }
}

impl From<diesel::result::Error> for TuskError {
    fn from(value: diesel::result::Error) -> Self {
        TuskError::DatabaseQueryError(value)
    }
}

impl From<std::io::Error> for TuskError {
    fn from(value: std::io::Error) -> Self {
        TuskError::IOError(value)
    }
}

impl From<r2d2::Error> for TuskError {
    fn from(value: r2d2::Error) -> Self {
        TuskError::R2D2Error(value)
    }
}

impl From<rustls::Error> for TuskError {
    fn from(value: rustls::Error) -> Self {
        TuskError::RustlsError(value)
    }
}

impl From<tera::Error> for TuskError {
    fn from(value: tera::Error) -> Self {
        TuskError::TeraParseError(value)
    }
}

#[cfg(unix)]
impl From<nix::errno::Errno> for TuskError {
    fn from(value: nix::errno::Errno) -> Self {
        TuskError::UnixError(value)
    }
}

#[cfg(windows)]
impl From<windows_service::Error> for TuskError {
    fn from(value: windows_service::Error) -> Self {
        TuskError::WindowsServiceError(value)
    }
}

