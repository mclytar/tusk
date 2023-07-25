use std::fmt::{Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ConfigurationFileError(toml::de::Error),
    IOError(std::io::Error),
    #[cfg(unix)]
    UnixError(nix::errno::Errno),
    #[cfg(windows)]
    WindowsServiceError(windows_service::Error),
    TeraParseError(tera::Error),
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ConfigurationFileError(e) => Display::fmt(e, f),
            Error::IOError(e) => Display::fmt(e, f),
            #[cfg(unix)]
            Error::UnixError(e) => Display::fmt(e, f),
            #[cfg(windows)]
            Error::WindowsServiceError(e) => Display::fmt(e, f),
            Error::TeraParseError(e) => Display::fmt(e, f)
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ConfigurationFileError(e) => Some(e),
            Error::IOError(e) => Some(e),
            #[cfg(unix)]
            Error::UnixError(e) => Some(e),
            #[cfg(windows)]
            Error::WindowsServiceError(e) => Some(e),
            Error::TeraParseError(e) => Some(e)
        }
    }
}

impl From<toml::de::Error> for Error {
    fn from(value: toml::de::Error) -> Self {
        Error::ConfigurationFileError(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IOError(value)
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

