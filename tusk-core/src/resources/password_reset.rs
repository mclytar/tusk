//! Data structures for the `password_reset` table.

use std::time::SystemTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::error::{HttpOkOr, TuskResult};

/// Represents a password reset request from an user.
///
/// Every interaction with the database removes from the table all the expired tokens, so that
/// the table does not grow with time.
#[derive(Clone, Debug, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::password_reset)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PasswordResetRequest {
    request_id: Uuid,
    user_id: Uuid,
    expiration: SystemTime
}
impl PasswordResetRequest {
    fn update_table(db_connection: &mut PgConnection) -> TuskResult<()> {
        use crate::schema::password_reset;

        let selected = password_reset::table
            .filter(password_reset::expiration.lt(SystemTime::now()));

        let _ = diesel::delete(selected)
            .execute(db_connection)?;

        Ok(())
    }

    /// Creates a new request for the given user.
    pub fn create(db_connection: &mut PgConnection, user_id: Uuid) -> TuskResult<PasswordResetRequest> {
        use crate::schema::password_reset;

        db_connection.transaction(|db_connection| {
            Self::update_table(db_connection)?;

            let request = diesel::insert_into(password_reset::table)
                .values(password_reset::user_id.eq(user_id))
                .get_result(db_connection)?;

            Ok(request)
        })
    }

    /// Returns `true` if the token is still valid and `false` if it is expired.
    pub fn valid(&self) -> bool { self.expiration >= SystemTime::now() }
    /// Returns the token of the request.
    pub fn token(&self) -> Uuid { self.request_id }
    /// Returns the user id of the request.
    pub fn user_id(&self) -> Uuid { self.user_id }

    /// Deletes the request.
    ///
    /// **Warning:** this operation is irreversible; use with caution.
    pub fn delete(self, db_connection: &mut PgConnection) -> TuskResult<()> {
        use crate::schema::password_reset;

        let selected = password_reset::table
            .filter(password_reset::request_id.eq(self.request_id));

        let _ = diesel::delete(selected)
            .execute(db_connection)?;

        Self::update_table(db_connection)?;

        Ok(())
    }
    /// Retrieves a valid request for the given token.
    pub fn from_token(db_connection: &mut PgConnection, token: Uuid) -> TuskResult<PasswordResetRequest> {
        use crate::schema::password_reset;

        db_connection.transaction(|db_connection| {
            Self::update_table(db_connection)?;

            let request = password_reset::table
                .filter(password_reset::expiration.ge(SystemTime::now()))
                .filter(password_reset::request_id.eq(token))
                .first(db_connection)
                .optional()?
                .or_unauthorized()?;

            Ok(request)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::ops::{Add, Sub};
    use std::time::{Duration, SystemTime};
    use uuid::Uuid;
    use crate::resources::PasswordResetRequest;

    #[test]
    fn token_validity() {
        let mut req = PasswordResetRequest {
            request_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            expiration: SystemTime::now().add(Duration::from_secs(60))
        };
        // Token expires in the future, so it is valid.
        assert!(req.valid());

        req.expiration = SystemTime::now().sub(Duration::from_secs(60));
        // Token expires in the past, so it is not valid anymore.
        assert!(!req.valid());
    }
}