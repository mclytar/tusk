//! This module contains the necessary structures and methods for error handling.

use std::error::Error;
use std::fmt::{Display, Formatter};
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use actix_web::body::BoxBody;
pub use diesel::result::Error as DieselError;
use uuid::Uuid;

/// A `Result` type with a preconfigured error of type [`tusk_core::error::Error`](crate::error::TuskError).
pub type TuskResult<T> = Result<T, TuskError>;
/// Represents an HTTP error on the REST API.
pub type TuskHttpResult = Result<HttpResponse, TuskError>;


/// Defines the possible errors when starting the Tusk server.
#[derive(Debug)]
pub enum TuskError {
    /// Error produced by `RedisSessionStore` (or any other thing that outputs [`anyhow::Error`]).
    Anyhow(anyhow::Error),
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
    /// HTTP Error defined via status code and inner error.
    HTTP {
        /// Status code of the HTTP error.
        status: StatusCode,
        /// Inner error that caused the error.
        inner: Option<Box<dyn Error + Send + Sync>>,
        /// Text to display to the client.
        text: Option<String>
    },
    /// An error originated while performing IO operations.
    IOError(std::io::Error),
    /// An error originated from a bad email address.
    MailError(lettre::address::AddressError),
    /// An error originated while performing a database migration.
    MigrationError(Box<dyn Error + Send + Sync>),
    /// An error originated while attempting to create a connection pool.
    R2D2Error(r2d2::Error),
    /// An error originated while attempting to create a secure channel.
    RustlsError(rustls::Error),
    /// An error originated while constructing a transport for sending emails.
    SmtpTransportError(lettre::transport::smtp::Error),
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
    /// Internally converts this error into an `HTTP` variant.
    pub fn into_http(self) -> Self {
        let status = self.status_code();
        match self {
            TuskError::Anyhow(_) => TuskError::HTTP { status, inner: None, text: None },
            TuskError::ConfigurationNotFound => TuskError::HTTP { status, inner: None, text: None },
            TuskError::CertificatesNotFound => TuskError::HTTP { status, inner: None, text: None },
            TuskError::ConfigurationFileError(e) => TuskError::HTTP { status, inner: Some(Box::new(e)), text: None },
            TuskError::DatabaseConnectionError(e) => TuskError::HTTP { status, inner: Some(Box::new(e)), text: None },
            TuskError::DatabaseQueryError(e) => TuskError::HTTP { status, inner: Some(Box::new(e)), text: None },
            TuskError::HTTP { status, inner, text } => return TuskError::HTTP { status, inner, text },
            TuskError::IOError(e) => TuskError::HTTP { status, inner: Some(Box::new(e)), text: None },
            TuskError::MailError(e) => TuskError::HTTP { status, inner: Some(Box::new(e)), text: None },
            TuskError::MigrationError(e) => TuskError::HTTP { status, inner: Some(e), text: None },
            TuskError::R2D2Error(e) => TuskError::HTTP { status, inner: Some(Box::new(e)), text: None },
            TuskError::RustlsError(e) => TuskError::HTTP { status, inner: Some(Box::new(e)), text: None },
            TuskError::SmtpTransportError(e) => TuskError::HTTP { status, inner: Some(Box::new(e)), text: None },
            TuskError::TeraParseError(e) => TuskError::HTTP { status, inner: Some(Box::new(e)), text: None },
            #[cfg(unix)]
            TuskError::UnixError(e) => TuskError::HTTP { status, inner: Some(Box::new(e)), text: None },
            #[cfg(windows)]
            TuskError::WindowsServiceError(e) => TuskError::HTTP { status, inner: Some(Box::new(e)), text: None },
        }
    }
    /// Logs this error instance with `info` log level.
    pub fn log_info(self) -> Self {
        log::info!("{self}");
        self
    }
    /// Logs this error instance with `warning` log level.
    pub fn log_warn(self) -> Self {
        log::warn!("{self}");
        self
    }
    /// Logs this error instance with `error` log level.
    pub fn log_error(self) -> Self {
        log::error!("{self}");
        self
    }
    /// Internally converts this error into an `HTTP` variant and attaches the specified `error`
    /// to the current `TuskError` instance.
    pub fn with_error<E: Error + Send + Sync + 'static>(self, error: E) -> Self {
        if let TuskError::HTTP { status, text, .. } = self {
            TuskError::HTTP { status, inner: Some(Box::new(error)), text }
        } else {
            let status = self.status_code();
            TuskError::HTTP { status, inner: Some(Box::new(error)), text: None }
        }
    }
    /// Internally converts this error into an `HTTP` variant and attaches the specified `text`
    /// to the current `TuskError` instance.
    pub fn with_text<S: Into<String>>(self, text: S) -> Self {
        let TuskError::HTTP { status, inner, .. } = self.into_http() else { unreachable!() };
        TuskError::HTTP { status, inner, text: Some(text.into()) }
    }
    /// Creates a [`TuskError`] from a boxed migration error.
    pub fn from_migration_error(e: Box<dyn std::error::Error + Send + Sync>) -> TuskError {
        TuskError::MigrationError(e)
    }
    /// Wraps the error into a [`Result::Err`] variant.
    pub fn bail<T>(self) -> Result<T, Self> {
        Err(self)
    }

    /// Creates a new instance of `TuskError` with status code `BAD REQUEST`.
    ///
    /// ## 400 -- BAD REQUEST
    ///
    /// The server cannot or will not process the request due to something that is perceived to be
    /// a client error (e.g., malformed request syntax, invalid request message framing,
    /// or deceptive request routing).
    pub fn bad_request() -> Self {
        TuskError::from(StatusCode::BAD_REQUEST)
    }
    /// Creates a new instance of `TuskError` with status code `UNAUTHORIZED`.
    ///
    /// ## 401 -- UNAUTHORIZED
    ///
    /// Although the HTTP standard specifies "unauthorized", semantically this response means
    /// "unauthenticated".
    /// That is, the client must authenticate itself to get the requested response.
    pub fn unauthorized() -> Self {
        TuskError::from(StatusCode::UNAUTHORIZED)
    }
    /// Creates a new instance of `TuskError` with status code `FORBIDDEN`.
    ///
    /// ## 403 -- FORBIDDEN
    ///
    /// The client does not have access rights to the content; that is, it is unauthorized,
    /// so the server is refusing to give the requested resource.
    /// Unlike 401 Unauthorized, the client's identity is known to the server.
    pub fn forbidden() -> Self {
        TuskError::from(StatusCode::FORBIDDEN)
    }
    /// Creates a new instance of `TuskError` with status code `NOT FOUND`.
    ///
    /// ## 404 -- NOT FOUND
    ///
    /// The server cannot find the requested resource.
    /// In the browser, this means the URL is not recognized. In an API, this can also mean that
    /// the endpoint is valid but the resource itself does not exist. Servers may also send
    /// this response instead of 403 Forbidden to hide the existence of a resource from
    /// an unauthorized client. This response code is probably the most well known due to
    /// its frequent occurrence on the web.
    pub fn not_found() -> Self {
        TuskError::from(StatusCode::NOT_FOUND)
    }
    /// Creates a new instance of `TuskError` with status code `METHOD NOT ALLOWED`.
    ///
    /// ## 405 -- METHOD NOT ALLOWED
    ///
    /// The request method is known by the server but is not supported by the target resource.
    /// For example, an API may not allow calling DELETE to remove a resource.
    pub fn method_not_allowed() -> Self {
        TuskError::from(StatusCode::METHOD_NOT_ALLOWED)
    }
    /// Creates a new instance of `TuskError` with status code `NOT ACCEPTABLE`.
    ///
    /// ## 406 -- NOT ACCEPTABLE
    ///
    /// This response is sent when the web server, after performing server-driven
    /// content negotiation, doesn't find any content that conforms to the criteria given
    /// by the user agent.
    pub fn not_acceptable() -> Self {
        TuskError::from(StatusCode::NOT_ACCEPTABLE)
    }
    /// Creates a new instance of `TuskError` with status code `CONFLICT`.
    ///
    /// ## 409 -- CONFLICT
    ///
    /// This response is sent when a request conflicts with the current state of the server.
    pub fn conflict() -> Self {
        TuskError::from(StatusCode::CONFLICT)
    }
    /// Creates a new instance of `TuskError` with status code `GONE`.
    ///
    /// ## 410 -- GONE
    ///
    /// This response is sent when the requested content has been permanently deleted from server,
    /// with no forwarding address. Clients are expected to remove their caches and link
    /// to the resource. The HTTP specification intends this status code to be used for
    /// "limited-time, promotional services". APIs should not feel compelled to indicate resources
    /// that have been deleted with this status code.
    pub fn gone() -> Self {
        TuskError::from(StatusCode::GONE)
    }
    /// Creates a new instance of `TuskError` with status code `I'M A TEAPOT`.
    ///
    /// ## 418 -- I'M A TEAPOT
    ///
    /// The server refuses the attempt to brew coffee with a teapot.
    pub fn i_am_a_teapot() -> Self {
        TuskError::from(StatusCode::IM_A_TEAPOT)
    }
    /// Creates a new instance of `TuskError` with status code `UNPROCESSABLE ENTITY`.
    ///
    /// ## 422 -- UNPROCESSABLE ENTITY
    ///
    /// The request was well-formed but was unable to be followed due to semantic errors.
    pub fn unprocessable_entity() -> Self {
        TuskError::from(StatusCode::UNPROCESSABLE_ENTITY)
    }

    /// Creates a new instance of `TuskError` with status code `INTERNAL SERVER ERROR`.
    ///
    /// ## 500 -- INTERNAL SERVER ERROR
    ///
    /// The server has encountered a situation it does not know how to handle.
    pub fn internal_server_error() -> Self {
        TuskError::from(StatusCode::INTERNAL_SERVER_ERROR)
    }
    /// Creates a new instance of `TuskError` with status code `NOT IMPLEMENTED`.
    ///
    /// ## 501 -- NOT IMPLEMENTED
    ///
    /// The request method is not supported by the server and cannot be handled. The only methods
    /// that servers are required to support (and therefore that must not return this code) are
    /// `GET` and `HEAD`.
    pub fn not_implemented() -> Self {
        TuskError::from(StatusCode::NOT_IMPLEMENTED)
    }
    /// Creates a new instance of `TuskError` with status code `BAD GATEWAY`.
    ///
    /// ## 502 -- BAD GATEWAY
    ///
    /// This error response means that the server, while working as a gateway to get a response
    /// needed to handle the request, got an invalid response.
    pub fn bad_gateway() -> Self {
        TuskError::from(StatusCode::BAD_GATEWAY)
    }
    /// Creates a new instance of `TuskError` with status code `SERVICE UNAVAILABLE`.
    ///
    /// ## 503 -- SERVICE UNAVAILABLE
    ///
    /// The server is not ready to handle the request. Common causes are a server that
    /// is down for maintenance or that is overloaded. Note that together with this response,
    /// a user-friendly page explaining the problem should be sent. This response should be used
    /// for temporary conditions and the `Retry-After` HTTP header should, if possible, contain
    /// the estimated time before the recovery of the service. The webmaster must also take care
    /// about the caching-related headers that are sent along with this response, as
    /// these temporary condition responses should usually not be cached.
    pub fn service_unavailable() -> Self {
        TuskError::from(StatusCode::SERVICE_UNAVAILABLE)
    }
    /// Creates a new instance of `TuskError` with status code `INSUFFICIENT STORAGE`.
    ///
    /// ## 507 -- INSUFFICIENT STORAGE
    ///
    /// The method could not be performed on the resource because the server is unable to store
    /// the representation needed to successfully complete the request.
    pub fn insufficient_storage() -> Self {
        TuskError::from(StatusCode::INSUFFICIENT_STORAGE)
    }
    /// Creates a new instance of `TuskError` with status code `LOOP DETECTED`.
    ///
    /// ## 508 -- LOOP DETECTED
    ///
    /// The server detected an infinite loop while processing the request.
    pub fn loop_detected() -> Self {
        TuskError::from(StatusCode::LOOP_DETECTED)
    }
}
impl Display for TuskError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TuskError::Anyhow(e) => Display::fmt(e, f),
            TuskError::ConfigurationNotFound => write!(f, "None of the given configuration files has been found."),
            TuskError::CertificatesNotFound => write!(f, "The certificate file does not contain any certificate."),
            TuskError::ConfigurationFileError(e) => Display::fmt(e, f),
            TuskError::DatabaseConnectionError(e) => Display::fmt(e, f),
            TuskError::DatabaseQueryError(e) => Display::fmt(e, f),
            TuskError::HTTP { inner: Some(e), .. } => Display::fmt(e, f),
            TuskError::HTTP { status, inner: None, .. } => Display::fmt(status, f),
            TuskError::IOError(e) => Display::fmt(e, f),
            TuskError::MailError(e) => Display::fmt(e, f),
            TuskError::MigrationError(e) => Display::fmt(e, f),
            TuskError::R2D2Error(e) => Display::fmt(e, f),
            TuskError::RustlsError(e) => Display::fmt(e, f),
            TuskError::SmtpTransportError(e) => Display::fmt(e, f),
            TuskError::TeraParseError(e) => Display::fmt(e, f),
            #[cfg(unix)]
            TuskError::UnixError(e) => Display::fmt(e, f),
            #[cfg(windows)]
            TuskError::WindowsServiceError(e) => Display::fmt(e, f),
        }
    }
}
impl Error for TuskError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TuskError::Anyhow(e) => Some(e.root_cause()),
            TuskError::ConfigurationNotFound => None,
            TuskError::CertificatesNotFound => None,
            TuskError::ConfigurationFileError(e) => Some(e),
            TuskError::DatabaseConnectionError(e) => Some(e),
            TuskError::DatabaseQueryError(e) => Some(e),
            TuskError::HTTP { inner: Some(e), .. } => Some(e.as_ref()),
            TuskError::HTTP { inner: None, .. } => None,
            TuskError::IOError(e) => Some(e),
            TuskError::MailError(e) => Some(e),
            TuskError::MigrationError(e) => Some(e.as_ref()),
            TuskError::R2D2Error(e) => Some(e),
            TuskError::RustlsError(e) => Some(e),
            TuskError::SmtpTransportError(e) => Some(e),
            TuskError::TeraParseError(e) => Some(e),
            #[cfg(unix)]
            TuskError::UnixError(e) => Some(e),
            #[cfg(windows)]
            TuskError::WindowsServiceError(e) => Some(e),
        }
    }
}

impl From<StatusCode> for TuskError {
    fn from(status: StatusCode) -> Self {
        TuskError::HTTP { status, inner: None, text: None }
    }
}
impl From<anyhow::Error> for TuskError {
    fn from(value: anyhow::Error) -> Self {
        TuskError::Anyhow(value)
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

impl From<lettre::transport::smtp::Error> for TuskError {
    fn from(value: lettre::transport::smtp::Error) -> Self {
        TuskError::SmtpTransportError(value)
    }
}

impl From<lettre::address::AddressError> for TuskError {
    fn from(value: lettre::address::AddressError) -> Self {
        TuskError::MailError(value)
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
impl ResponseError for TuskError {
    fn status_code(&self) -> StatusCode {
        match self {
            TuskError::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TuskError::ConfigurationNotFound => StatusCode::INTERNAL_SERVER_ERROR,
            TuskError::CertificatesNotFound => StatusCode::INTERNAL_SERVER_ERROR,
            TuskError::ConfigurationFileError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TuskError::DatabaseConnectionError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TuskError::DatabaseQueryError(DieselError::NotFound) => StatusCode::NOT_FOUND,
            TuskError::DatabaseQueryError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TuskError::HTTP { status, .. } => *status,
            TuskError::IOError(e) => e.status_code(),
            TuskError::MailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TuskError::MigrationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TuskError::R2D2Error(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TuskError::RustlsError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TuskError::SmtpTransportError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TuskError::TeraParseError(e) => match e.kind {
                tera::ErrorKind::TemplateNotFound(_) => StatusCode::INTERNAL_SERVER_ERROR,
                tera::ErrorKind::Io(std::io::ErrorKind::NotFound) => StatusCode::NOT_FOUND,
                tera::ErrorKind::Io(std::io::ErrorKind::PermissionDenied) => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR
            },
            #[cfg(unix)]
            TuskError::UnixError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            #[cfg(windows)]
            TuskError::WindowsServiceError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        if let TuskError::HTTP { status, text: Some(text), .. } = self {
            HttpResponse::build(*status)
                .body(text.to_owned())
        } else {
            HttpResponse::build(self.status_code())
                .finish()
        }
    }
}

/// Converts the `Err` variant of the `Result` into one of the possible HTTP error responses.
pub trait HttpOkOr<T>: Sized {
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 400 -- BAD REQUEST
    ///
    /// The server cannot or will not process the request due to something that is perceived to be
    /// a client error (e.g., malformed request syntax, invalid request message framing,
    /// or deceptive request routing).
    fn or_bad_request(self) -> Result<T, TuskError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 401 -- UNAUTHORIZED
    ///
    /// Although the HTTP standard specifies "unauthorized", semantically this response means
    /// "unauthenticated".
    /// That is, the client must authenticate itself to get the requested response.
    fn or_unauthorized(self) -> Result<T, TuskError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 403 -- FORBIDDEN
    ///
    /// The client does not have access rights to the content; that is, it is unauthorized,
    /// so the server is refusing to give the requested resource.
    /// Unlike 401 Unauthorized, the client's identity is known to the server.
    fn or_forbidden(self) -> Result<T, TuskError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 404 -- NOT FOUND
    ///
    /// The server cannot find the requested resource.
    /// In the browser, this means the URL is not recognized. In an API, this can also mean that
    /// the endpoint is valid but the resource itself does not exist. Servers may also send
    /// this response instead of 403 Forbidden to hide the existence of a resource from
    /// an unauthorized client. This response code is probably the most well known due to
    /// its frequent occurrence on the web.
    fn or_not_found(self) -> Result<T, TuskError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 405 -- METHOD NOT ALLOWED
    ///
    /// The request method is known by the server but is not supported by the target resource.
    //     /// For example, an API may not allow calling DELETE to remove a resource.
    fn or_method_not_allowed(self) -> Result<T, TuskError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 406 -- NOT ACCEPTABLE
    ///
    /// This response is sent when the web server, after performing server-driven
    /// content negotiation, doesn't find any content that conforms to the criteria given
    /// by the user agent.
    fn or_not_acceptable(self) -> Result<T, TuskError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 409 -- CONFLICT
    ///
    /// This response is sent when a request conflicts with the current state of the server.
    fn or_conflict(self) -> Result<T, TuskError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 410 -- GONE
    ///
    /// This response is sent when the requested content has been permanently deleted from server,
    /// with no forwarding address. Clients are expected to remove their caches and link
    /// to the resource. The HTTP specification intends this status code to be used for
    /// "limited-time, promotional services". APIs should not feel compelled to indicate resources
    /// that have been deleted with this status code.
    fn or_gone(self) -> Result<T, TuskError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 418 -- I'M A TEAPOT
    ///
    /// The server refuses the attempt to brew coffee with a teapot.
    fn or_i_am_a_teapot(self) -> Result<T, TuskError>;

    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 500 -- INTERNAL SERVER ERROR
    ///
    /// The server has encountered a situation it does not know how to handle.
    fn or_internal_server_error(self) -> Result<T, TuskError>;

    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 501 -- NOT IMPLEMENTED
    ///
    /// The request method is not supported by the server and cannot be handled. The only methods
    /// that servers are required to support (and therefore that must not return this code) are
    /// `GET` and `HEAD`.
    fn or_not_implemented(self) -> Result<T, TuskError>;

    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 502 -- BAD GATEWAY
    ///
    /// This error response means that the server, while working as a gateway to get a response
    /// needed to handle the request, got an invalid response.
    fn or_bad_gateway(self) -> Result<T, TuskError>;

    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 503 -- SERVICE UNAVAILABLE
    ///
    /// The server is not ready to handle the request. Common causes are a server that
    /// is down for maintenance or that is overloaded. Note that together with this response,
    /// a user-friendly page explaining the problem should be sent. This response should be used
    /// for temporary conditions and the `Retry-After` HTTP header should, if possible, contain
    /// the estimated time before the recovery of the service. The webmaster must also take care
    /// about the caching-related headers that are sent along with this response, as
    /// these temporary condition responses should usually not be cached.
    fn or_service_unavailable(self) -> Result<T, TuskError>;

    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 507 -- INSUFFICIENT STORAGE
    ///
    /// The method could not be performed on the resource because the server is unable to store
    /// the representation needed to successfully complete the request.
    fn or_insufficient_storage(self) -> Result<T, TuskError>;

    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 508 -- LOOP DETECTED
    ///
    /// The server detected an infinite loop while processing the request.
    fn or_loop_detected(self) -> Result<T, TuskError>;
    /// Logs this error instance with `info` log level.
    fn log_info(self) -> Self { self }
    /// Logs this error instance with `warning` log level.
    fn log_warn(self) -> Self { self }
    /// Logs this error instance with `error` log level.
    fn log_error(self) -> Self { self }
}
impl<T, E: Error + Send + Sync + 'static> HttpOkOr<T> for Result<T, E> {
    fn or_bad_request(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::bad_request().with_error(e))
    }
    fn or_unauthorized(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::unauthorized().with_error(e))
    }
    fn or_forbidden(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::forbidden().with_error(e))
    }
    fn or_not_found(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::not_found().with_error(e))
    }
    fn or_method_not_allowed(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::method_not_allowed().with_error(e))
    }
    fn or_not_acceptable(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::not_acceptable().with_error(e))
    }
    fn or_conflict(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::conflict().with_error(e))
    }
    fn or_gone(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::gone().with_error(e))
    }
    fn or_i_am_a_teapot(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::i_am_a_teapot().with_error(e))
    }

    fn or_internal_server_error(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::internal_server_error().with_error(e))
    }
    fn or_not_implemented(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::not_implemented().with_error(e))
    }
    fn or_bad_gateway(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::bad_gateway().with_error(e))
    }
    fn or_service_unavailable(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::service_unavailable().with_error(e))
    }
    fn or_insufficient_storage(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::insufficient_storage().with_error(e))
    }
    fn or_loop_detected(self) -> Result<T, TuskError> {
        self.map_err(|e| TuskError::loop_detected().with_error(e))
    }
    /// Logs this error instance with `info` log level.
    fn log_info(self) -> Self {
        if let Err(e) = self {
            log::info!("{e}");
            Err(e)
        } else { self }
    }
    /// Logs this error instance with `warning` log level.
    fn log_warn(self) -> Self {
        if let Err(e) = self {
            log::warn!("{e}");
            Err(e)
        } else { self }
    }
    /// Logs this error instance with `error` log level.
    fn log_error(self) -> Self {
        if let Err(e) = self {
            log::error!("{e}");
            Err(e)
        } else { self }
    }
}
impl<T> HttpOkOr<T> for Option<T> {
    fn or_bad_request(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::bad_request())
    }
    fn or_unauthorized(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::unauthorized())
    }
    fn or_forbidden(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::forbidden())
    }
    fn or_not_found(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::not_found())
    }
    fn or_method_not_allowed(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::method_not_allowed())
    }
    fn or_not_acceptable(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::not_acceptable())
    }
    fn or_conflict(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::conflict())
    }
    fn or_gone(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::gone())
    }
    fn or_i_am_a_teapot(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::i_am_a_teapot())
    }

    fn or_internal_server_error(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::internal_server_error())
    }
    fn or_not_implemented(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::not_implemented())
    }
    fn or_bad_gateway(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::bad_gateway())
    }
    fn or_service_unavailable(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::service_unavailable())
    }
    fn or_insufficient_storage(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::insufficient_storage())
    }
    fn or_loop_detected(self) -> Result<T, TuskError> {
        self.ok_or_else(|| TuskError::loop_detected())
    }
}
/// Trait that implements utility methods on `Result<T, TuskError>`.
pub trait TuskErrorResult<T> {
    /// Masks the authentication attempt as a generic `UNAUTHORIZED` error,
    /// even if the original error is of type `NOT FOUND`.
    fn mask_authentication_failure<S: AsRef<str>>(self, user: S) -> Result<T, TuskError>;
}
impl<T> TuskErrorResult<T> for Result<T, TuskError> {
    fn mask_authentication_failure<S: AsRef<str>>(self, user: S) -> Result<T, TuskError> {
        if let Err(e) = self {
            let user = user.as_ref();
            let status_code = e.status_code();
            if status_code == StatusCode::NOT_FOUND {
                log::warn!("Failed login attempt for user `{user}`");
                let password = Uuid::new_v4().to_string();
                crate::resources::User::fake_password_check(password);
                TuskError::unauthorized()
            } else if status_code == StatusCode::UNAUTHORIZED {
                log::warn!("Failed login attempt for user `{user}`");
                TuskError::unauthorized()
            } else {
                e
            }.bail()
        } else { self }
    }
}
impl<T> TuskErrorResult<T> for Option<T> {
    fn mask_authentication_failure<S: AsRef<str>>(self, user: S) -> Result<T, TuskError> {
        if let Some(value) = self {
            Ok(value)
        } else {
            let user = user.as_ref();
            log::warn!("Failed login attempt for user `{user}`");
            let password = Uuid::new_v4().to_string();
            crate::resources::User::fake_password_check(password);
            TuskError::unauthorized().bail()
        }
    }
}

// TODO: test, at some point