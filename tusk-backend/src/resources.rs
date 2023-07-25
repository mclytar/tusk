use diesel::prelude::*;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use uuid::Uuid;

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
    pub fn create<U: AsRef<str>>(db_connection: &mut PgConnection, username: U, password: Secret<String>) -> diesel::QueryResult<User> {
        use crate::schema::user;
        let username = username.as_ref();
        let password = password.expose_secret();

        let password = bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .unwrap();

        diesel::insert_into(user::table)
            .values(
                (user::username.eq(username), user::password.eq(&password))
            ).get_result(db_connection)
    }

    pub fn read(db_connection: &mut PgConnection, user_id: Uuid) -> diesel::QueryResult<User> {
        use crate::schema::user;

        user::table
            .filter(user::user_id.eq(user_id))
            .first(db_connection)
    }

    pub fn read_by_username<S: AsRef<str>>(db_connection: &mut PgConnection, username: S) -> diesel::QueryResult<User> {
        use crate::schema::user;
        let username = username.as_ref();

        user::table
            .filter(user::username.eq(username))
            .first(db_connection)
    }

    pub fn read_all(db_connection: &mut PgConnection) -> diesel::QueryResult<Vec<User>> {
        use crate::schema::user;

        user::table
            .load(db_connection)
    }

    pub fn verify_password(&self, password: &Secret<String>) -> bool {
        let password = password.expose_secret();
        bcrypt::verify(password, &self.password)
            .unwrap()
    }
}