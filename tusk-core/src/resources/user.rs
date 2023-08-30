//! Data structures for the `user` table.

use diesel::deserialize::FromSql;
use diesel::prelude::*;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::Result;

/// Wraps a `Secret` so that it is possible to query it from an SQL table.
#[derive(Clone, Debug, Deserialize)]
pub struct Password(Secret<String>);
impl<DB: diesel::backend::Backend> Queryable<diesel::sql_types::Text, DB> for Password
where String: FromSql<diesel::sql_types::Text, DB>{
    type Row = String;

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(Password(Secret::new(row)))
    }
}

#[derive(Clone, Debug, Queryable, Selectable, Deserialize)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::pg::Pg))]
/// Defines a user.
pub struct User {
    user_id: Uuid,
    username: String,
    #[serde(skip_serializing)]
    password: Password
}
impl User {
    /// Inserts a new user in the table and returns the corresponding result.
    pub fn create<U: AsRef<str>>(db_connection: &mut PgConnection, username: U, password: Secret<String>) -> Result<User> {
        use crate::schema::user;
        let username = username.as_ref();
        let password = password.expose_secret();

        let password = bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .unwrap();

        let user = diesel::insert_into(user::table)
            .values(
                (user::username.eq(username), user::password.eq(&password))
            ).get_result(db_connection)?;

        Ok(user)
    }
    /// Reads an user from the table, given the user ID.
    pub fn read(db_connection: &mut PgConnection, user_id: Uuid) -> Result<User> {
        use crate::schema::user;

        let user = user::table
            .filter(user::user_id.eq(user_id))
            .first(db_connection)?;

        Ok(user)
    }
    /// Reads an user from the table, given the username.
    pub fn read_by_username<S: AsRef<str>>(db_connection: &mut PgConnection, username: S) -> Result<User> {
        use crate::schema::user;
        let username = username.as_ref();

        let user = user::table
            .filter(user::username.eq(username))
            .first(db_connection)?;

        Ok(user)
    }
    /// Reads all users from the table.
    pub fn read_all(db_connection: &mut PgConnection) -> Result<Vec<User>> {
        use crate::schema::user;

        let users = user::table
            .load(db_connection)?;

        Ok(users)
    }
    /// Verifies correctness of the user's password by comparing its hash with the hash stored
    /// in the database.
    pub fn verify_password(&self, password: &Secret<String>) -> bool {
        let password = password.expose_secret();
        bcrypt::verify(password, self.password.0.expose_secret())
            .unwrap()
    }
    /// Returns the ID of the current user.
    pub fn id(&self) -> Uuid {
        self.user_id
    }
    /// Returns the username of the current user.
    pub fn username(&self) -> &str {
        &self.username
    }
    /// Deletes an user given the user ID.
    pub fn delete_by_id(db_connection: &mut PgConnection, user_id: Uuid) -> Result<usize> {
        use crate::schema::user;

        let selected = user::table
            .filter(user::user_id.eq(user_id));

        let num_deleted = diesel::delete(selected)
            .execute(db_connection)?;

        Ok(num_deleted)
    }
    /// Deletes an user given the username.
    pub fn delete_by_username<S: AsRef<str>>(db_connection: &mut PgConnection, username: S) -> Result<usize> {
        let username = username.as_ref();

        use crate::schema::user;

        let selected = user::table
            .filter(user::username.eq(username));

        let num_deleted = diesel::delete(selected)
            .execute(db_connection)?;

        Ok(num_deleted)
    }
}