//! Contains the CRUD structures relative to the `/users` REST resource.

use actix_web::HttpResponse;
use secrecy::{Secret};
use serde::Deserialize;
use tusk_core::resources::User;
use crate::error::{HttpError, HttpIfError, HttpResult, WrapResult};

/// Represents the CRUD (partial) **Update** structure relative to the `/users/{username}` REST resource.
#[derive(Clone, Debug, Deserialize)]
pub struct UserPatchData {
    username: Option<String>,
    password: Option<Secret<String>>,
    proof: Option<Secret<String>>
}
impl UserPatchData {
    /// Returns true if only the user itself is allowed to perform the requested changes.
    pub fn only_owner(&self) -> bool {
        self.username.is_some() || self.password.is_some()
    }
    /// Returns the new username of the user, if any.
    pub fn username(&self) -> Option<&str> {
        self.username.as_ref()
            .map(|s| s.as_str())
    }
    /// Returns the new password of the user, if any.
    pub fn password(&self) -> Option<&Secret<String>> {
        self.password.as_ref()
    }
    /// Returns a proof that the original user is requesting the changes.
    ///
    /// The proof consists of the current password of the user.
    pub fn proof(&self) -> Option<&Secret<String>> {
        self.proof.as_ref()
    }
    /// Applies the changes, or reports the error if any.
    pub fn apply<I: AsRef<str>, U: AsRef<str>>(&self, db_connection: &mut tusk_core::PgConnection, initiator: I, user: U) -> HttpResult {
        let initiator = initiator.as_ref();
        let user = user.as_ref();

        let mut user = User::read_by_username(db_connection, user)
            .map_err(|e| HttpError::from(e))
            .with_authentication_failure(user, "fake password")?;

        if self.only_owner() {
            if initiator != user.username() {
                return HttpError::forbidden()
                    .wrap_err();
            }

            if let Some(proof) = &self.proof {
                if !user.verify_password(proof) {
                    log::warn!("Failed authentication attempt for user `{}`", user.username());
                    return Err(HttpError::unauthorized());
                }
            } else {
                return HttpError::unauthorized()
                    .wrap_err();
            }
        }

        if let Some(username) = &self.username {
            user = user.update_username(db_connection, username)?;
        }

        if let Some(password) = &self.password {
            user.update_password(db_connection, password)?;
        }

        HttpResponse::Ok()
            .finish()
            .wrap_ok()
    }
}