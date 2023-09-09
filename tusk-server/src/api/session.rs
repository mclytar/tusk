//! Contains the CRUD structures relative to the `/session` REST resource.
//!
//! # Security
//! ## Creation
//! A session is created upon
//! ```POST /session```
//! with the correct information provided as form data.
//!
//! The information for an user is correct when:
//! - the user exists in the database;
//! - the given password verifies with the hash stored in the database.
//!
//! If any of these conditions fail, the response will be a generic `UNAUTHORIZED` response.
//! Additionally, if the user does not exist, a fake hash of the given password will be computed,
//! so that no information about the existence of the user is revealed.
//!
//! ## Verification
//! A valid session is a session where:
//! - the client holds a session ID cookie;
//! - the REDIS server contains encrypted data for the ID given by the client cookie;
//! - the encrypted data contains the username of the user.
//!
//! If any of these conditions fail, the response will be with status `UNAUTHORIZED`.
//!
//! ## Deletion
//! When a session is deleted, it is removed both from the server and the client.
//! This means that the server cleans the record with the current session ID
//! (possibly generating a new ID with no data) and the current session ID is not sent anymore to
//! the client.
//!
//! Since the client is arbitrary, it does not necessarily delete the session ID;
//! hence, it is important to remove it from the server.

use actix_session::Session;
use actix_web::{HttpResponse};
use actix_web::web::{self};
use secrecy::{Secret, ExposeSecret};
use serde::{Deserialize, Serialize};
use tusk_core::config::{TuskData};
use tusk_core::resources::User;
use tusk_derive::rest_resource;
use crate::error::{HttpError, HttpIfError, HttpOkOr, HttpResult, WrapResult};

/// Represents the CRUD **Create** structure relative to the `/session` REST resource.
#[derive(Clone, Debug, Deserialize)]
pub struct SessionCreate {
    username: String,
    password: Secret<String>
}
impl SessionCreate {
    /// Creates a new `SessionCreate` form data given username and password.
    pub fn new<U: AsRef<str>, P: AsRef<str>>(username: U, password: P) -> Self {
        let username = username.as_ref().to_owned();
        let password = Secret::new(String::from(password.as_ref()));

        SessionCreate {
            username,
            password
        }
    }
    /// Returns the username of the user this session references to.
    pub fn username(&self) -> &str {
        &self.username
    }
    /// Returns the password of the user this session references to.
    pub fn password(&self) -> &Secret<String> {
        &self.password
    }
}

/// Represents the CRUD **Read** structure relative to the `/session` REST resource.
#[derive(Clone, Eq, PartialEq, Debug, Serialize)]
pub struct SessionRead {
    username: String
}
impl SessionRead {
    /// Returns the username of the user this session references to.
    pub fn username(&self) -> &str {
        &self.username
    }
}
impl TryFrom<Session> for SessionRead {
    type Error = HttpError;

    fn try_from(value: Session) -> Result<Self, Self::Error> {
        let username = value.get("username")
            .or_unauthorized()?
            .or_unauthorized()?;
        let data = SessionRead {
            username
        };
        Ok(data)
    }
}

/// Represents the `/session` REST resource.
///
/// The `/session` resource is responsible for authenticating users and keeping user sessions.
pub struct SessionResource;
#[rest_resource("/session")]
impl SessionResource {
    async fn get(session: Session) -> HttpResult {
        let session: SessionRead = session.try_into()
            .or_unauthorized()?;

        HttpResponse::Ok()
            .json(session)
            .wrap_ok()
    }


    async fn post(tusk: TuskData, session: Session, web::Form(session_create): web::Form<SessionCreate>) -> HttpResult {
        let mut db_connection = tusk.database_connect()
            .or_internal_server_error()?;

        let user = User::read_by_username(&mut db_connection, session_create.username())
            .map_err(|e| HttpError::from(e))
            .with_authentication_failure(session_create.username(), session_create.password().expose_secret())?;

        if !user.verify_password(session_create.password()) {
            log::warn!("Failed login attempt for user `{}`", session_create.username());
            return Err(HttpError::unauthorized());
        }

        session.renew();
        session.insert("username", session_create.username())
            .or_internal_server_error()
            .with_log_error()?;
        log::info!("User {} logged in", session_create.username());

        HttpResponse::Created()
            .finish()
            .wrap_ok()
    }

    async fn delete(session: Session) -> HttpResult {
        session.clear();
        session.purge();

        HttpResponse::Ok()
            .finish()
            .wrap_ok()
    }
}

#[cfg(test)]
pub mod test {
    use actix_web::{FromRequest, ResponseError, web};
    use actix_web::http::{StatusCode};
    use actix_web::test::TestRequest;
    use tusk_core::config::TEST_CONFIGURATION;
    use crate::api::session::SessionCreate;
    use crate::api::{SessionResource};

    pub async fn create_empty_session() -> actix_session::Session {
        let req = TestRequest::default()
            .to_http_request();

        actix_session::Session::extract(&req).await
            .expect("empty session")
    }

    pub async fn create_session_for_user<S: AsRef<str>>(username: S) -> actix_session::Session {
        let username = username.as_ref();

        let session = create_empty_session().await;
        session.renew();
        session.insert("username", username)
            .expect("updated session");

        session
    }

    #[actix_web::test]
    async fn successful_login_attempt() {
        let tusk = TEST_CONFIGURATION.to_data();
        let session = create_empty_session().await;
        let form = web::Form(SessionCreate::new("user", "user#vX78"));

        let resp = SessionResource::post(tusk, session, form).await
            .expect("response");
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[actix_web::test]
    async fn failed_login_attempt() {
        let tusk = TEST_CONFIGURATION.to_data();
        let session = create_empty_session().await;

        // Wrong password leads to UNAUTHORIZED.
        let form = web::Form(SessionCreate::new("user", "not user's password"));
        let err = SessionResource::post(tusk.clone(), session.clone(), form).await
            .expect_err("error response");
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);

        // Wrong username leads to UNAUTHORIZED, as per OWASP cheat sheet:
        // login should not leak users existing or not.
        let form = web::Form(SessionCreate::new("not_user", "1234567890"));
        let resp = SessionResource::post(tusk.clone(), session.clone(), form).await
            .expect_err("error response");
        assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn retrieve_session() {
        let session = create_session_for_user("user").await;

        let resp = SessionResource::get(session).await
            .expect("response");
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn delete_session() {
        let session = create_session_for_user("user").await;

        // BEFORE
        let resp = SessionResource::get(session.clone()).await
            .expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        // DELETE
        let resp = SessionResource::delete(session.clone()).await
            .expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        // AFTER
        let resp = SessionResource::get(session).await
            .expect_err("error response");
        assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
    }
}