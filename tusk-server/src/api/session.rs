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

use actix_web::{HttpResponse};
use actix_web::web::{Json};
use secrecy::{Secret};
use serde::{Deserialize};
use tusk_core::config::Tusk;
use tusk_core::error::{TuskError, TuskErrorResult, TuskHttpResult};
use tusk_core::resources::{User};
use tusk_derive::rest_resource;

#[derive(Clone, Debug, Deserialize)]
struct SessionPostData {
    email: String,
    password: Secret<String>
}

/// Represents the `/session` REST resource.
///
/// The `/session` resource is responsible for authenticating users and keeping user sessions.
pub struct SessionResource;
#[rest_resource("/session")]
impl SessionResource {
    async fn get(tusk: Tusk) -> TuskHttpResult {
        let auth_session = tusk.authenticate()?;
        Ok(HttpResponse::Ok().json(auth_session))
    }


    async fn post(tusk: Tusk, Json(session_create): Json<SessionPostData>) -> TuskHttpResult {
        let mut db_connection = tusk.db()?;

        let user = User::from_email(&mut db_connection, &session_create.email)
            .mask_authentication_failure(&session_create.email)?
            .mask_authentication_failure(&session_create.email)?;

        if !user.verify_password(&session_create.password) {
            log::warn!("Failed login attempt for user `{}`", &session_create.email);
            return TuskError::unauthorized().bail();
        }

        tusk.log_in(&user)?;
        log::info!("User {} logged in", &session_create.email);

        Ok(HttpResponse::Created().finish())
    }

    async fn delete(tusk: Tusk) -> HttpResponse {
        tusk.log_out();

        HttpResponse::NoContent()
            .finish()
    }
}