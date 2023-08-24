//! Contains the CRUD structures relative to the `/session` REST resource.

use actix_session::{Session};
use secrecy::Secret;
use serde::{Deserialize, Serialize};
use crate::error::{HttpError, HttpOkOr};

/// Represents the CRUD **Create** structure relative to the `/session` REST resource.
#[derive(Clone, Debug, Deserialize)]
pub struct SessionCreate {
    username: String,
    password: Secret<String>
}
impl SessionCreate {
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

#[cfg(test)]
mod test {
    // No tests needed.
}