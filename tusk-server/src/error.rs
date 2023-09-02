//! Module for error handling and utilities.

use std::error::Error;
use std::fmt::{Display, Formatter};
use actix_web::{HttpResponse, ResponseError};
use actix_web::body::BoxBody;
use actix_web::http::header::{HeaderMap, TryIntoHeaderPair};
use actix_web::http::StatusCode;
use tusk_core::DieselError;
use tusk_core::error::Error as TuskError;

/// Wraps the structure in one of the `Result` variants.
pub trait WrapResult: Sized {
    /// Wraps the structure into the `Ok` variant of the `Result` enum.
    fn wrap_ok<E>(self) -> Result<Self, E> {
        Ok(self)
    }
    /// Wraps the structure into the `Err` variant of the `Result` enum.
    fn wrap_err<T>(self) -> Result<T, Self> { Err(self) }
}
impl<T: Sized> WrapResult for T {}

/// Result which resolves either into an `Ok(HttpResponse)` item or into an `Err(HttpError)` item.
pub type HttpResult = Result<HttpResponse, HttpError>;

/// Converts the `Err` variant of the `Result` into one of the possible HTTP error responses.
pub trait HttpOkOr<T> {
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 400 -- BAD REQUEST
    ///
    /// The server cannot or will not process the request due to something that is perceived to be
    /// a client error (e.g., malformed request syntax, invalid request message framing,
    /// or deceptive request routing).
    fn or_bad_request(self) -> Result<T, HttpError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 401 -- UNAUTHORIZED
    ///
    /// Although the HTTP standard specifies "unauthorized", semantically this response means
    /// "unauthenticated".
    /// That is, the client must authenticate itself to get the requested response.
    fn or_unauthorized(self) -> Result<T, HttpError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 403 -- FORBIDDEN
    ///
    /// The client does not have access rights to the content; that is, it is unauthorized,
    /// so the server is refusing to give the requested resource.
    /// Unlike 401 Unauthorized, the client's identity is known to the server.
    fn or_forbidden(self) -> Result<T, HttpError>;
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
    fn or_not_found(self) -> Result<T, HttpError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 405 -- METHOD NOT ALLOWED
    ///
    /// The request method is known by the server but is not supported by the target resource.
    //     /// For example, an API may not allow calling DELETE to remove a resource.
    fn or_method_not_allowed(self) -> Result<T, HttpError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 406 -- NOT ACCEPTABLE
    ///
    /// This response is sent when the web server, after performing server-driven
    /// content negotiation, doesn't find any content that conforms to the criteria given
    /// by the user agent.
    fn or_not_acceptable(self) -> Result<T, HttpError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 409 -- CONFLICT
    ///
    /// This response is sent when a request conflicts with the current state of the server.
    fn or_conflict(self) -> Result<T, HttpError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 410 -- GONE
    ///
    /// This response is sent when the requested content has been permanently deleted from server,
    /// with no forwarding address. Clients are expected to remove their caches and link
    /// to the resource. The HTTP specification intends this status code to be used for
    /// "limited-time, promotional services". APIs should not feel compelled to indicate resources
    /// that have been deleted with this status code.
    fn or_gone(self) -> Result<T, HttpError>;
    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 418 -- I'M A TEAPOT
    ///
    /// The server refuses the attempt to brew coffee with a teapot.
    fn or_i_am_a_teapot(self) -> Result<T, HttpError>;

    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 500 -- INTERNAL SERVER ERROR
    ///
    /// The server has encountered a situation it does not know how to handle.
    fn or_internal_server_error(self) -> Result<T, HttpError>;

    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 501 -- NOT IMPLEMENTED
    ///
    /// The request method is not supported by the server and cannot be handled. The only methods
    /// that servers are required to support (and therefore that must not return this code) are
    /// `GET` and `HEAD`.
    fn or_not_implemented(self) -> Result<T, HttpError>;

    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 502 -- BAD GATEWAY
    ///
    /// This error response means that the server, while working as a gateway to get a response
    /// needed to handle the request, got an invalid response.
    fn or_bad_gateway(self) -> Result<T, HttpError>;

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
    fn or_service_unavailable(self) -> Result<T, HttpError>;

    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 507 -- INSUFFICIENT STORAGE
    ///
    /// The method could not be performed on the resource because the server is unable to store
    /// the representation needed to successfully complete the request.
    fn or_insufficient_storage(self) -> Result<T, HttpError>;

    /// Converts the `Err` variant of the `Result` into an HTTP response with the following status code.
    ///
    /// ## 508 -- LOOP DETECTED
    ///
    /// The server detected an infinite loop while processing the request.
    fn or_loop_detected(self) -> Result<T, HttpError>;
}
impl<T, E: Error + 'static> HttpOkOr<T> for Result<T, E> {
    fn or_bad_request(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::bad_request().error(e))
    }
    fn or_unauthorized(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::unauthorized().error(e))
    }
    fn or_forbidden(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::forbidden().error(e))
    }
    fn or_not_found(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::not_found().error(e))
    }
    fn or_method_not_allowed(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::method_not_allowed().error(e))
    }
    fn or_not_acceptable(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::not_acceptable().error(e))
    }
    fn or_conflict(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::conflict().error(e))
    }
    fn or_gone(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::gone().error(e))
    }
    fn or_i_am_a_teapot(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::i_am_a_teapot().error(e))
    }

    fn or_internal_server_error(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::internal_server_error().error(e))
    }
    fn or_not_implemented(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::not_implemented().error(e))
    }
    fn or_bad_gateway(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::bad_gateway().error(e))
    }
    fn or_service_unavailable(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::service_unavailable().error(e))
    }
    fn or_insufficient_storage(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::insufficient_storage().error(e))
    }
    fn or_loop_detected(self) -> Result<T, HttpError> {
        self.map_err(|e| HttpError::loop_detected().error(e))
    }
}
impl<T> HttpOkOr<T> for Option<T> {
    fn or_bad_request(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::bad_request())
    }
    fn or_unauthorized(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::unauthorized())
    }
    fn or_forbidden(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::forbidden())
    }
    fn or_not_found(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::not_found())
    }
    fn or_method_not_allowed(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::method_not_allowed())
    }
    fn or_not_acceptable(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::not_acceptable())
    }
    fn or_conflict(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::conflict())
    }
    fn or_gone(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::gone())
    }
    fn or_i_am_a_teapot(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::i_am_a_teapot())
    }

    fn or_internal_server_error(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::internal_server_error())
    }
    fn or_not_implemented(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::not_implemented())
    }
    fn or_bad_gateway(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::bad_gateway())
    }
    fn or_service_unavailable(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::service_unavailable())
    }
    fn or_insufficient_storage(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::insufficient_storage())
    }
    fn or_loop_detected(self) -> Result<T, HttpError> {
        self.ok_or_else(|| HttpError::loop_detected())
    }
}

/// Implements specific functions for type `Result<T, HttpError>`, so that it's easier to add
/// further information to the `HttpError` error variant.
pub trait HttpIfError<T> {
    /// Executes the specified `handler` in case the item is the error variant.
    fn with<F: FnOnce(&mut HttpError)>(self, handler: F) -> Result<T, HttpError>;
    /// Logs the error as `info` in case the item is the error variant.
    fn with_log_info(self) -> Result<T, HttpError>;
    /// Logs the error as `warn` in case the item is the error variant.
    fn with_log_warn(self) -> Result<T, HttpError>;
    /// Logs the error as `error` in case the item is the error variant.
    fn with_log_error(self) -> Result<T, HttpError>;
    /// Adds an HTTP response header in case the item is the error variant.
    fn with_header<P: TryIntoHeaderPair>(self, header: P) -> Result<T, HttpError>;
    /// Adds an HTTP response body in case the item is the error variant.
    fn with_body<S: AsRef<str>>(self, body: S) -> Result<T, HttpError>;
    /// Converts a possible `NOT FOUND` error into a generic `UNAUTHORIZED` error and computes a
    /// fake password hash, so that no information is leaked about the user existence.
    fn with_authentication_failure<U: AsRef<str>, P: AsRef<str>>(self, username: U, password: P) -> Result<T, HttpError>;
}
impl<T> HttpIfError<T> for Result<T, HttpError> {
    fn with<F: FnOnce(&mut HttpError)>(self, handler: F) -> Result<T, HttpError> {
        match self {
            Ok(value) => Ok(value),
            Err(mut e) => {
                handler(&mut e);
                Err(e)
            }
        }
    }

    fn with_log_info(self) -> Result<T, HttpError> {
        match self {
            Ok(value) => Ok(value),
            Err(e) => {
                log::info!("{e}");
                Err(e)
            }
        }
    }

    fn with_log_warn(self) -> Result<T, HttpError> {
        match self {
            Ok(value) => Ok(value),
            Err(e) => {
                log::warn!("{e}");
                Err(e)
            }
        }
    }

    fn with_log_error(self) -> Result<T, HttpError> {
        match self {
            Ok(value) => Ok(value),
            Err(e) => {
                log::error!("{e}");
                Err(e)
            }
        }
    }

    fn with_header<P: TryIntoHeaderPair>(self, header: P) -> Result<T, HttpError> {
        match self {
            Ok(value) => Ok(value),
            Err(mut e) => match header.try_into_pair() {
                Ok((name, value)) => {
                    e.headers.append(name, value);
                    Err(e)
                },
                Err(dbl_e) => {
                    log::error!("Header error while processing error: {}", dbl_e.into());
                    Err(e)
                }
            }
        }
    }

    fn with_body<S: AsRef<str>>(self, body: S) -> Result<T, HttpError> {
        match self {
            Ok(value) => Ok(value),
            Err(mut e) => {
                e.body = Some(body.as_ref().to_owned());
                Err(e)
            }
        }
    }

    fn with_authentication_failure<U: AsRef<str>, P: AsRef<str>>(self, username: U, password: P) -> Result<T, HttpError> {
        let username = username.as_ref();

        match self {
            Ok(value) => Ok(value),
            Err(mut e) if e.status_code == StatusCode::NOT_FOUND => {
                log::warn!("Failed login attempt for user `{}`", username);
                e.status_code = StatusCode::UNAUTHORIZED;
                let _check = tusk_core::resources::User::fake_password_check(password);
                Err(e)
            },
            Err(e) => Err(e)
        }
    }
}

/// Defines an HTTP response in case of error.
#[derive(Debug)]
pub struct HttpError {
    status_code: StatusCode,
    headers: HeaderMap,
    body: Option<String>,
    inner: Option<Box<dyn Error>>
}
impl HttpError {
    /// Creates a new instance of `HttpError` with status code `BAD REQUEST`.
    ///
    /// ## 400 -- BAD REQUEST
    ///
    /// The server cannot or will not process the request due to something that is perceived to be
    /// a client error (e.g., malformed request syntax, invalid request message framing,
    /// or deceptive request routing).
    pub fn bad_request() -> Self {
        HttpError::from(StatusCode::BAD_REQUEST)
    }
    /// Creates a new instance of `HttpError` with status code `UNAUTHORIZED`.
    ///
    /// ## 401 -- UNAUTHORIZED
    ///
    /// Although the HTTP standard specifies "unauthorized", semantically this response means
    /// "unauthenticated".
    /// That is, the client must authenticate itself to get the requested response.
    pub fn unauthorized() -> Self {
        HttpError::from(StatusCode::UNAUTHORIZED)
    }
    /// Creates a new instance of `HttpError` with status code `FORBIDDEN`.
    ///
    /// ## 403 -- FORBIDDEN
    ///
    /// The client does not have access rights to the content; that is, it is unauthorized,
    /// so the server is refusing to give the requested resource.
    /// Unlike 401 Unauthorized, the client's identity is known to the server.
    pub fn forbidden() -> Self {
        HttpError::from(StatusCode::FORBIDDEN)
    }
    /// Creates a new instance of `HttpError` with status code `NOT FOUND`.
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
        HttpError::from(StatusCode::NOT_FOUND)
    }
    /// Creates a new instance of `HttpError` with status code `METHOD NOT ALLOWED`.
    ///
    /// ## 405 -- METHOD NOT ALLOWED
    ///
    /// The request method is known by the server but is not supported by the target resource.
    /// For example, an API may not allow calling DELETE to remove a resource.
    pub fn method_not_allowed() -> Self {
        HttpError::from(StatusCode::METHOD_NOT_ALLOWED)
    }
    /// Creates a new instance of `HttpError` with status code `NOT ACCEPTABLE`.
    ///
    /// ## 406 -- NOT ACCEPTABLE
    ///
    /// This response is sent when the web server, after performing server-driven
    /// content negotiation, doesn't find any content that conforms to the criteria given
    /// by the user agent.
    pub fn not_acceptable() -> Self {
        HttpError::from(StatusCode::NOT_ACCEPTABLE)
    }
    /// Creates a new instance of `HttpError` with status code `CONFLICT`.
    ///
    /// ## 409 -- CONFLICT
    ///
    /// This response is sent when a request conflicts with the current state of the server.
    pub fn conflict() -> Self {
        HttpError::from(StatusCode::CONFLICT)
    }
    /// Creates a new instance of `HttpError` with status code `GONE`.
    ///
    /// ## 410 -- GONE
    ///
    /// This response is sent when the requested content has been permanently deleted from server,
    /// with no forwarding address. Clients are expected to remove their caches and link
    /// to the resource. The HTTP specification intends this status code to be used for
    /// "limited-time, promotional services". APIs should not feel compelled to indicate resources
    /// that have been deleted with this status code.
    pub fn gone() -> Self {
        HttpError::from(StatusCode::GONE)
    }
    /// Creates a new instance of `HttpError` with status code `I'M A TEAPOT`.
    ///
    /// ## 418 -- I'M A TEAPOT
    ///
    /// The server refuses the attempt to brew coffee with a teapot.
    pub fn i_am_a_teapot() -> Self {
        HttpError::from(StatusCode::IM_A_TEAPOT)
    }

    /// Creates a new instance of `HttpError` with status code `INTERNAL SERVER ERROR`.
    ///
    /// ## 500 -- INTERNAL SERVER ERROR
    ///
    /// The server has encountered a situation it does not know how to handle.
    pub fn internal_server_error() -> Self {
        HttpError::from(StatusCode::INTERNAL_SERVER_ERROR)
    }
    /// Creates a new instance of `HttpError` with status code `NOT IMPLEMENTED`.
    ///
    /// ## 501 -- NOT IMPLEMENTED
    ///
    /// The request method is not supported by the server and cannot be handled. The only methods
    /// that servers are required to support (and therefore that must not return this code) are
    /// `GET` and `HEAD`.
    pub fn not_implemented() -> Self {
        HttpError::from(StatusCode::NOT_IMPLEMENTED)
    }
    /// Creates a new instance of `HttpError` with status code `BAD GATEWAY`.
    ///
    /// ## 502 -- BAD GATEWAY
    ///
    /// This error response means that the server, while working as a gateway to get a response
    /// needed to handle the request, got an invalid response.
    pub fn bad_gateway() -> Self {
        HttpError::from(StatusCode::BAD_GATEWAY)
    }
    /// Creates a new instance of `HttpError` with status code `SERVICE UNAVAILABLE`.
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
        HttpError::from(StatusCode::SERVICE_UNAVAILABLE)
    }
    /// Creates a new instance of `HttpError` with status code `INSUFFICIENT STORAGE`.
    ///
    /// ## 507 -- INSUFFICIENT STORAGE
    ///
    /// The method could not be performed on the resource because the server is unable to store
    /// the representation needed to successfully complete the request.
    pub fn insufficient_storage() -> Self {
        HttpError::from(StatusCode::INSUFFICIENT_STORAGE)
    }
    /// Creates a new instance of `HttpError` with status code `LOOP DETECTED`.
    ///
    /// ## 508 -- LOOP DETECTED
    ///
    /// The server detected an infinite loop while processing the request.
    pub fn loop_detected() -> Self {
        HttpError::from(StatusCode::LOOP_DETECTED)
    }

    /// Attaches the specified `error` to the current `HttpError` instance.
    pub fn error<E: Error + 'static>(mut self, error: E) -> Self {
        self.inner = Some(Box::new(error));
        self
    }
}
impl Display for HttpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(error) = &self.inner {
            Display::fmt(error, f)?;
        } else if let Some(body) = &self.body {
            write!(f, "{}: {}", self.status_code, body)?;
        } else {
            write!(f, "{}", self.status_code)?;
        }
        Ok(())
    }
}
impl Error for HttpError {
    fn cause(&self) -> Option<&dyn Error> {
        match &self.inner {
            Some(inner) => Some(inner.as_ref()),
            None => None
        }
    }
}
impl ResponseError for HttpError {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let mut response = HttpResponse::build(self.status_code);
        for (name, value) in &self.headers {
            response.append_header((name, value));
        }
        if let Some(body) = &self.body {
            response.body(body.to_owned())
        } else {
            response.finish()
        }
    }
}
impl From<StatusCode> for HttpError {
    fn from(value: StatusCode) -> Self {
        HttpError {
            status_code: value,
            headers: HeaderMap::new(),
            body: None,
            inner: None
        }
    }
}
impl From<tusk_core::error::Error> for HttpError {
    fn from(value: tusk_core::error::Error) -> Self {
        let status_code = match value {
            TuskError::DatabaseQueryError(DieselError::NotFound) => StatusCode::NOT_FOUND,
            TuskError::IOError(e) if e.kind() == std::io::ErrorKind::AlreadyExists => StatusCode::CONFLICT,
            TuskError::IOError(e) if e.kind() == std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR
        };
        Self::from(status_code)
    }
}