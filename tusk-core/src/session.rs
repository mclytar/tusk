//! This module contains the [`AuthenticatedSession`] extractor, which either automatically extracts
//! the authenticated user from the session, or returns an HTTP error code 401 `UNAUTHORIZED` if
//! this is not possible because no session exists for the selected user.

use std::path::PathBuf;
use actix_session::Session;
use diesel::{Connection, PgConnection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::config::TuskConfiguration;
use crate::error::{HttpOkOr, TuskError, TuskResult};
use crate::resources::{Role, User};

/// Represents an authenticated session, that is, a session belonging of a logged user.
#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct AuthenticatedSession {
    id: Uuid,
    email: String,
    display: String
}
impl AuthenticatedSession {
    /// Returns the id of the user this session references to.
    pub fn user_id(&self) -> Uuid {
        self.id
    }
    /// Returns the email of the user this session references to.
    pub fn email(&self) -> &str {
        &self.email
    }
    /// Returns the display name of the user this session references to.
    pub fn display(&self) -> &str {
        &self.email
    }
    /// Returns the user structure of the user this session references to.
    pub fn user(&self, db: &mut PgConnection) -> Result<User, TuskError> {
        let user = User::from_id(db, self.id)?;
        Ok(user)
    }
    /// Returns the absolute path to the directory in which the user files would be stored
    /// if the user has role `directory`.
    pub fn directory(&self, tusk: &TuskConfiguration) -> TuskResult<PathBuf> {
        let mut path = tusk.user_directories()
            .canonicalize()?;
        path.push(self.id.to_string());
        Ok(path)
    }
    /// Returns the list of roles the user belongs to.
    pub fn roles(&self, db: &mut PgConnection) -> Result<Vec<Role>, TuskError> {
        let roles = db.transaction(|db| {
            User::from_id(db, self.id)?
                .roles(db)
        })?;

        Ok(roles)
    }
}
impl From<&User> for AuthenticatedSession {
    fn from(value: &User) -> Self {
        AuthenticatedSession {
            id: value.id(),
            email: value.email().to_owned(),
            display: value.display().to_owned()
        }
    }
}
impl TryFrom<&Session> for AuthenticatedSession {
    type Error = TuskError;

    fn try_from(value: &Session) -> Result<Self, Self::Error> {
        let data = value.get("auth_session")
            .or_unauthorized()?
            .or_unauthorized()?;
        Ok(data)
    }
}