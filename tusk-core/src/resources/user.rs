//! Data structures for the `user` table.

use diesel::deserialize::FromSql;
use diesel::prelude::*;
use secrecy::{ExposeSecret, Secret};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::error::TuskResult;

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

#[derive(Clone, Debug, Queryable, Selectable, Serialize, Deserialize)]
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
    pub fn create<U: AsRef<str>>(db_connection: &mut PgConnection, username: U, password: Secret<String>) -> TuskResult<User> {
        use crate::schema::user;
        let username = username.as_ref();

        let password = bcrypt::hash(password.expose_secret(), bcrypt::DEFAULT_COST)
            .unwrap();

        let user = diesel::insert_into(user::table)
            .values(
                (user::username.eq(username), user::password.eq(&password))
            ).get_result(db_connection)?;

        Ok(user)
    }
    /// Reads an user from the table, given the user ID.
    pub fn read(db_connection: &mut PgConnection, user_id: Uuid) -> TuskResult<User> {
        use crate::schema::user;

        let user = user::table
            .filter(user::user_id.eq(user_id))
            .first(db_connection)?;

        Ok(user)
    }
    /// Reads an user from the table, given the username.
    pub fn read_by_username<S: AsRef<str>>(db_connection: &mut PgConnection, username: S) -> TuskResult<User> {
        use crate::schema::user;
        let username = username.as_ref();

        let user = user::table
            .filter(user::username.eq(username))
            .first(db_connection)?;

        Ok(user)
    }
    /// Reads all the users with the given role.
    pub fn read_by_role_name<S: AsRef<str>>(db_connection: &mut PgConnection, name: S) -> TuskResult<Vec<User>> {
        let name = name.as_ref();

        use crate::schema::{user, role, user_role};

        let users = user::table
            .inner_join(user_role::table.inner_join(role::table))
            .filter(role::name.eq(name))
            .select(User::as_select())
            .load::<User>(db_connection)?;

        Ok(users)
    }
    /// Reads all users from the table.
    pub fn read_all(db_connection: &mut PgConnection) -> TuskResult<Vec<User>> {
        use crate::schema::user;

        let users = user::table
            .load(db_connection)?;

        Ok(users)
    }
    /// Updates the username of the selected user.
    pub fn update_username<U: AsRef<str>>(self, db_connection: &mut PgConnection, username: U) -> TuskResult<Self> {
        use crate::schema::user;

        let user = diesel::update(user::table)
            .filter(user::username.eq(self.username))
            .set(user::username.eq(username.as_ref()))
            .get_result(db_connection)?;

        Ok(user)
    }
    /// Updates the password of the selected user.
    pub fn update_password(self, db_connection: &mut PgConnection, password: &Secret<String>) -> TuskResult<Self> {
        use crate::schema::user;

        let password = bcrypt::hash(password.expose_secret(), bcrypt::DEFAULT_COST)
            .unwrap();

        let user = diesel::update(user::table)
            .filter(user::username.eq(self.username))
            .set(user::password.eq(password))
            .get_result(db_connection)?;

        Ok(user)
    }
    /// Deletes an user given the user ID.
    pub fn delete_by_id(db_connection: &mut PgConnection, user_id: Uuid) -> TuskResult<usize> {
        use crate::schema::user;

        let selected = user::table
            .filter(user::user_id.eq(user_id));

        let num_deleted = diesel::delete(selected)
            .execute(db_connection)?;

        Ok(num_deleted)
    }
    /// Deletes an user given the username.
    pub fn delete_by_username<S: AsRef<str>>(db_connection: &mut PgConnection, username: S) -> TuskResult<usize> {
        let username = username.as_ref();

        use crate::schema::user;

        let selected = user::table
            .filter(user::username.eq(username));

        let num_deleted = diesel::delete(selected)
            .execute(db_connection)?;

        Ok(num_deleted)
    }
    /// Fakes the verification of a password to avoid information leakage about the existence of
    /// a user.
    pub fn fake_password_check<P: AsRef<str>>(password: P) -> String {
        bcrypt::hash(password.as_ref(), bcrypt::DEFAULT_COST)
            .unwrap()
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
}

#[cfg(test)]
mod test {
    use diesel::Connection;
    use secrecy::{ExposeSecret, Secret};
    use crate::config::TEST_CONFIGURATION;
    use crate::resources::{User};

    #[test]
    fn create_user() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let user = User::create(db_connection, "user1234", Secret::new(String::from("password1234")))
                .expect("user");

            assert_eq!(user.username(), "user1234");
            assert!(user.verify_password(&Secret::new(String::from("password1234"))));

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn cannot_create_user_twice() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            User::create(db_connection, "user1234", Secret::new(String::from("password1234")))
                .expect("user");

            assert!(User::create(db_connection, "user1234", Secret::new(String::from("password5678"))).is_err());

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn read_user_by_id() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // SETUP
            let user = User::create(db_connection, "user1234", Secret::new(String::from("password1234")))
                .expect("user");
            let user_id = user.id();

            // TEST
            let user = User::read(db_connection, user_id)
                .expect("user");
            assert_eq!(user.id(), user_id);
            assert_eq!(user.username(), "user1234");
            assert!(user.verify_password(&Secret::new(String::from("password1234"))));

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn read_user_by_name() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let user = User::read_by_username(db_connection, "admin")
                .expect("user");
            assert_eq!(user.username(), "admin");
            assert!(user.verify_password(&Secret::new(String::from("admin"))));

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn read_users() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let mut users = User::read_all(db_connection).expect("users");
            users.sort_by(|a, b| a.username().cmp(b.username()));

            assert_eq!(users[0].username(), "admin");
            assert_eq!(users[1].username(), "dummy");
            assert_eq!(users[2].username(), "test");
            assert_eq!(users[3].username(), "user");
            assert_eq!(users.len(), 4);

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn read_users_by_role() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let mut users = User::read_by_role_name(db_connection, "directory").expect("users");
            users.sort_by(|a, b| a.username().cmp(b.username()));

            assert_eq!(users[0].username(), "admin");
            assert_eq!(users[1].username(), "test");
            assert_eq!(users[2].username(), "user");
            assert_eq!(users.len(), 3);

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn update_username() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // SETUP
            let user = User::read_by_username(db_connection, "test")
                .expect("user");

            assert_eq!(user.username(), "test");
            assert!(user.verify_password(&Secret::new(String::from("test"))));

            // TEST
            let user = user.update_username(db_connection, "user5678")
                .expect("user");
            assert_eq!(user.username(), "user5678");
            assert!(user.verify_password(&Secret::new(String::from("test"))));

            let mut users = User::read_all(db_connection).expect("users");
            users.sort_by(|a, b| a.username().cmp(b.username()));

            assert_eq!(users[0].username(), "admin");
            assert_eq!(users[1].username(), "dummy");
            assert_eq!(users[2].username(), "user");
            assert_eq!(users[3].username(), "user5678");
            assert_eq!(users.len(), 4);

            assert_eq!(users[3].id(), user.id());
            assert_eq!(users[3].username(), user.username());
            assert_eq!(users[3].password.0.expose_secret(), user.password.0.expose_secret());

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn update_password() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // SETUP
            let user = User::read_by_username(db_connection, "test")
                .expect("user");

            assert_eq!(user.username(), "test");
            assert!(user.verify_password(&Secret::new(String::from("test"))));

            // TEST
            let user = user.update_password(db_connection, &Secret::new(String::from("user5678")))
                .expect("user");
            assert_eq!(user.username(), "test");
            assert!(user.verify_password(&Secret::new(String::from("user5678"))));

            let mut users = User::read_all(db_connection).expect("users");
            users.sort_by(|a, b| a.username().cmp(b.username()));

            assert_eq!(users[0].username(), "admin");
            assert_eq!(users[1].username(), "dummy");
            assert_eq!(users[2].username(), "test");
            assert_eq!(users[3].username(), "user");
            assert_eq!(users.len(), 4);

            assert_eq!(users[2].id(), user.id());
            assert_eq!(users[2].username(), user.username());
            assert_eq!(users[2].password.0.expose_secret(), user.password.0.expose_secret());

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn delete_user_by_id() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // SETUP
            let user_dummy = User::read_by_username(db_connection, "dummy").expect("user");
            let user_id = user_dummy.id();

            // BEFORE
            let mut users = User::read_all(db_connection).expect("users");
            users.sort_by(|a, b| a.username().cmp(b.username()));

            assert_eq!(users[0].username(), "admin");
            assert_eq!(users[1].username(), "dummy");
            assert_eq!(users[2].username(), "test");
            assert_eq!(users[3].username(), "user");
            assert_eq!(users.len(), 4);

            // TEST
            let deleted = User::delete_by_id(db_connection, user_id).expect("user deletion");
            assert_eq!(deleted, 1);

            // AFTER
            let mut users = User::read_all(db_connection).expect("users");
            users.sort_by(|a, b| a.username().cmp(b.username()));

            assert_eq!(users[0].username(), "admin");
            assert_eq!(users[1].username(), "test");
            assert_eq!(users[2].username(), "user");
            assert_eq!(users.len(), 3);

            // TEST DOUBLE DELETE DOES NOTHING
            let deleted = User::delete_by_id(db_connection, user_id).expect("user deletion");
            assert_eq!(deleted, 0);

            // AFTER DOUBLE DELETE
            let mut users = User::read_all(db_connection).expect("users");
            users.sort_by(|a, b| a.username().cmp(b.username()));

            assert_eq!(users[0].username(), "admin");
            assert_eq!(users[1].username(), "test");
            assert_eq!(users[2].username(), "user");
            assert_eq!(users.len(), 3);

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn delete_user_by_username() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // BEFORE
            let mut users = User::read_all(db_connection).expect("users");
            users.sort_by(|a, b| a.username().cmp(b.username()));

            assert_eq!(users[0].username(), "admin");
            assert_eq!(users[1].username(), "dummy");
            assert_eq!(users[2].username(), "test");
            assert_eq!(users[3].username(), "user");
            assert_eq!(users.len(), 4);

            // TEST
            let deleted = User::delete_by_username(db_connection, "user").expect("user deletion");
            assert_eq!(deleted, 1);

            // AFTER
            let mut users = User::read_all(db_connection).expect("users");
            users.sort_by(|a, b| a.username().cmp(b.username()));

            assert_eq!(users[0].username(), "admin");
            assert_eq!(users[1].username(), "dummy");
            assert_eq!(users[2].username(), "test");
            assert_eq!(users.len(), 3);

            // TEST DOUBLE DELETE DOES NOTHING
            let deleted = User::delete_by_username(db_connection, "user").expect("user deletion");
            assert_eq!(deleted, 0);

            // AFTER DOUBLE DELETE
            let mut users = User::read_all(db_connection).expect("users");
            users.sort_by(|a, b| a.username().cmp(b.username()));

            assert_eq!(users[0].username(), "admin");
            assert_eq!(users[1].username(), "dummy");
            assert_eq!(users[2].username(), "test");
            assert_eq!(users.len(), 3);

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn password_does_not_leak() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        let user = User::read_by_username(&mut db_connection, "user").expect("user");
        assert!(user.verify_password(&Secret::new(String::from("1234567890"))));
        let leaked_password = user.password.0.expose_secret();

        let user_json = serde_json::to_string(&user).expect("json string");
        assert!(!user_json.contains(leaked_password));

        let user_string = format!("{user:?}");
        assert!(!user_string.contains(leaked_password));
    }

    #[test]
    fn deserialize_user() {
        let uuid = uuid::Uuid::default();
        let password = bcrypt::hash("password", bcrypt::DEFAULT_COST)
            .expect("password hash");
        let user_json = format!(r#"{{ "user_id": "{uuid}", "username": "jsonny", "password": "{password}" }}"#);
        let user: User = serde_json::from_str(&user_json).expect("valid json");

        assert_eq!(user.id(), uuid);
        assert_eq!(user.username(), "jsonny");
        assert!(user.verify_password(&Secret::new(String::from("password"))));
    }
}