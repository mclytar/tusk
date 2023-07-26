use diesel::prelude::*;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::Result;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Queryable, Selectable, Deserialize)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub user_id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String
}
impl User {
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

    pub fn read(db_connection: &mut PgConnection, user_id: Uuid) -> Result<User> {
        use crate::schema::user;

        let user = user::table
            .filter(user::user_id.eq(user_id))
            .first(db_connection)?;

        Ok(user)
    }

    pub fn read_by_username<S: AsRef<str>>(db_connection: &mut PgConnection, username: S) -> Result<User> {
        use crate::schema::user;
        let username = username.as_ref();

        let user = user::table
            .filter(user::username.eq(username))
            .first(db_connection)?;

        Ok(user)
    }

    pub fn read_all(db_connection: &mut PgConnection) -> Result<Vec<User>> {
        use crate::schema::user;

        let users = user::table
            .load(db_connection)?;

        Ok(users)
    }

    pub fn verify_password(&self, password: &Secret<String>) -> bool {
        let password = password.expose_secret();
        bcrypt::verify(password, &self.password)
            .unwrap()
    }

    pub fn delete_by_id(db_connection: &mut PgConnection, user_id: Uuid) -> Result<usize> {
        use crate::schema::user;

        let selected = user::table
            .filter(user::user_id.eq(user_id));

        let num_deleted = diesel::delete(selected)
            .execute(db_connection)?;

        Ok(num_deleted)
    }

    pub fn delete_by_username(db_connection: &mut PgConnection, username: String) -> Result<usize> {
        use crate::schema::user;

        let selected = user::table
            .filter(user::username.eq(username));

        let num_deleted = diesel::delete(selected)
            .execute(db_connection)?;

        Ok(num_deleted)
    }
}