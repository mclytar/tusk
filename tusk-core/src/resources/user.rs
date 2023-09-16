//! Data structures for the `user` table.

use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use diesel::deserialize::FromSql;
use diesel::prelude::*;
use lettre::message::Mailbox;
use rand::distributions::Alphanumeric;
use rand::Rng;
use secrecy::{ExposeSecret, Secret};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use crate::config::TuskConfiguration;

use crate::error::{TuskResult};
use crate::resources::{PasswordResetRequest, Role};

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

/// Helper structure to construct a user.
#[warn(unused_must_use)]
pub struct UserBuilder {
    email: String,
    display: Option<String>,
    password: Option<Secret<String>>
}
impl UserBuilder {
    /// Sets the display name of the user to be created.
    pub fn display<S: Into<String>>(mut self, display: S) -> Self {
        self.display = Some(display.into());
        self
    }
    /// Sets the password of the user to be created.
    pub fn password<P: Into<String>>(mut self, password: P) -> Self {
        self.password = Some(Secret::new(password.into()));
        self
    }
    /// Creates the user.
    ///
    /// If a password has been set, then the user will have the given password and this function
    /// will not set a [`PasswordResetRequest`]; conversely, if no password was given, this
    /// function generates a random password for the user and then creates a password reset request
    /// to allow the newly created user to change his password.
    pub fn build(self, db_connection: &mut PgConnection) -> TuskResult<(User, Option<PasswordResetRequest>)> {
        use crate::schema::user;
        let UserBuilder { email, display, password } = self;
        let display = display.unwrap_or_else(|| email.clone());

        db_connection.transaction(|db_connection| {
            let password_hash = if let Some(password) = &password {
                bcrypt::hash(password.expose_secret(), bcrypt::DEFAULT_COST)
                    .unwrap()
            } else {
                let random_password: String = rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(24)
                    .map(char::from)
                    .collect();
                bcrypt::hash(random_password, bcrypt::DEFAULT_COST)
                    .unwrap()
            };

            let user: User = diesel::insert_into(user::table)
                .values(
                    (user::email.eq(email), user::display.eq(display), user::password.eq(&password_hash))
                ).get_result(db_connection)?;

            let pw_request = if password.is_none() {
                Some(PasswordResetRequest::create(db_connection, user.user_id)?)
            } else {
                None
            };

            Ok((user, pw_request))
        })
    }
}

#[derive(Clone, Debug, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::pg::Pg))]
/// Defines a user.
pub struct User {
    user_id: Uuid,
    email: String,
    display: String,
    #[serde(skip_serializing)]
    password: Password
}
impl User {
    /// Creates a new user using the builder pattern.
    ///
    /// The builder does not create any user until the function [`UserBuilder::build`] is invoked.
    pub fn builder<S: Into<String>>(email: S) -> UserBuilder {
        UserBuilder {
            email: email.into(),
            display: None,
            password: None
        }
    }

    /// Returns the ID of the current user.
    pub fn id(&self) -> Uuid { self.user_id }
    /// Returns the email of the current user.
    pub fn email(&self) -> &str { &self.email }
    /// Returns the display name of the current user.
    pub fn display(&self) -> &str { &self.display }
    /// Returns a wrapper that allows to easily display the user as email contact data.
    pub fn mailbox(&self) -> TuskResult<Mailbox> {
        let mailbox = if self.email == self.display {
            Mailbox::new(None, self.email.parse()?)
        } else {
            Mailbox::new(Some(self.display.clone()), self.email.parse()?)
        };
        Ok(mailbox)
    }
    /// Leaks the password of the user, for test purposes.
    #[cfg(feature = "test_utils")]
    pub fn _test_leak_password(&self) -> &str {
        self.password.0.expose_secret()
    }


    /// Deletes the user.
    ///
    /// **Warning:** this operation is irreversible; use with caution.
    pub fn delete(self, db_connection: &mut PgConnection) -> TuskResult<()> {
        use crate::schema::user;

        let selected = user::table
            .filter(user::user_id.eq(self.user_id));

        let _ = diesel::delete(selected)
            .execute(db_connection)?;

        Ok(())
    }
    /// Returns the absolute path to the directory in which the user files would be stored
    /// if the user has role `directory`.
    pub fn directory(&self, tusk: &TuskConfiguration) -> TuskResult<PathBuf> {
        let mut path = tusk.user_directories()
            .canonicalize()?;
        path.push(self.user_id.to_string());
        Ok(path)
    }
    /// Fakes the verification of a password to avoid information leakage about the existence of
    /// a user.
    pub fn fake_password_check<P: AsRef<str>>(password: P) -> String {
        bcrypt::hash(password.as_ref(), bcrypt::DEFAULT_COST)
            .unwrap()
    }
    /// Reads an user from the table, given the username.
    pub fn from_email<S: AsRef<str>>(db_connection: &mut PgConnection, email: S) -> TuskResult<Option<User>> {
        use crate::schema::user;
        let email = email.as_ref();

        let user = user::table
            .filter(user::email.eq(email))
            .first(db_connection)
            .optional()?;

        Ok(user)
    }
    /// Reads an user from the table, given the user ID.
    pub fn from_id(db_connection: &mut PgConnection, user_id: Uuid) -> TuskResult<User> {
        use crate::schema::user;

        let user = user::table
            .filter(user::user_id.eq(user_id))
            .first(db_connection)?;

        Ok(user)
    }
    /// Returns a list of all the users.
    pub fn list_all(db_connection: &mut PgConnection) -> TuskResult<Vec<User>> {
        use crate::schema::user;

        let users = user::table
            .load(db_connection)?;

        Ok(users)
    }
    /// Creates a new password reset request, to be satisfied within 24 hours.
    pub fn request_password_reset(&self, db_connection: &mut PgConnection) -> TuskResult<PasswordResetRequest> {
        PasswordResetRequest::create(db_connection, self.user_id)
    }
    /// Returns the roles this user belongs to.
    pub fn roles(&self, db_connection: &mut PgConnection) -> TuskResult<Vec<Role>> {
        use crate::schema::{user, role, user_role};

        let roles = user::table
            .inner_join(user_role::table.inner_join(role::table))
            .filter(user::user_id.eq(self.user_id))
            .select(Role::as_select())
            .load::<Role>(db_connection)?;

        Ok(roles)
    }
    /// Updates the email of the user.
    pub fn update_email<U: AsRef<str>>(&mut self, db_connection: &mut PgConnection, email: U) -> TuskResult<()> {
        use crate::schema::user;

        let user: User = diesel::update(user::table)
            .filter(user::user_id.eq(self.user_id))
            .set(user::email.eq(email.as_ref()))
            .get_result(db_connection)?;

        *self = user;

        Ok(())
    }
    /// Updates the display name of the user.
    pub fn update_display_name<D: AsRef<str>>(&mut self, db_connection: &mut PgConnection, display: D) -> TuskResult<()> {
        use crate::schema::user;

        let user: User = diesel::update(user::table)
            .filter(user::user_id.eq(self.user_id))
            .set(user::display.eq(display.as_ref()))
            .get_result(db_connection)?;

        *self = user;

        Ok(())
    }
    /// Updates the password of the selected user.
    pub fn update_password(&mut self, db_connection: &mut PgConnection, password: &Secret<String>) -> TuskResult<()> {
        use crate::schema::user;

        let password = bcrypt::hash(password.expose_secret(), bcrypt::DEFAULT_COST)
            .unwrap();

        let user: User = diesel::update(user::table)
            .filter(user::user_id.eq(self.user_id))
            .set(user::password.eq(password))
            .get_result(db_connection)?;

        *self = user;

        Ok(())
    }
    /// Verifies correctness of the user's password by comparing its hash with the hash stored
    /// in the database.
    pub fn verify_password(&self, password: &Secret<String>) -> bool {
        let password = password.expose_secret();
        bcrypt::verify(password, self.password.0.expose_secret())
            .unwrap()
    }
}
impl Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "{} <{}>", self.email, self.display)
        } else {
            write!(f, "{}", self.display)
        }
    }
}

/*
#[cfg(test)]
mod test {
    use std::convert::Infallible;
    use diesel::Connection;
    use secrecy::{ExposeSecret, Secret};
    use uuid::Uuid;
    use crate::config::TEST_CONFIGURATION;
    use crate::resources::{User};

    #[test]
    fn create_user_with_password() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let (user, request) = User::builder("user1234@example.com")
                .display("User1234")
                .password("Password1234")
                .build(db_connection)
                .expect("User");

            assert_eq!(user.email(), "user1234@example.com");
            assert!(user.verify_password(&Secret::new(String::from("Password1234"))));
            assert!(request.is_none());

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn create_user_without_password() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let (user, request) = User::builder("user1234@example.com")
                .display("User1234")
                .build(db_connection)
                .expect("User");

            assert_eq!(user.email(), "user1234@example.com");

            let request = request.expect("A password reset request");
            assert_eq!(request.user_id(), user.id());

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn cannot_create_user_twice() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let _ = User::builder("user1234@example.com")
                .display("User1234")
                .password("Password1234")
                .build(db_connection)
                .expect("User");

            let _ = User::builder("user1234@example.com")
                .display("User1234")
                .password("Password1234")
                .build(db_connection)
                .expect_err("User already exists");

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn read_user_by_id() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let user_id = Uuid::from_u64_pair(0, 1);
            let user = User::from_id(db_connection, user_id)
                .expect("User");
            assert_eq!(user.id(), user_id);
            assert_eq!(user.email(), "test@example.com");
            assert_eq!(user.display(), "Test");
            assert!(user.verify_password(&Secret::new(String::from("test#7U5c"))));

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn read_user_by_email() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let user = User::from_email(db_connection, "admin@example.com")
                .expect("User")
                .expect("An existing user");
            assert_eq!(user.email(), "admin@example.com");
            assert!(user.verify_password(&Secret::new(String::from("admin#f9E5"))));

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn read_users() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let mut users = User::list_all(db_connection).expect("users");
            users.sort_by(|a, b| a.email().cmp(b.email()));

            assert_eq!(users[0].email(), "admin@example.com");
            assert_eq!(users[1].email(), "dummy@example.com");
            assert_eq!(users[2].email(), "test@example.com");
            assert_eq!(users[3].email(), "user@example.com");
            assert_eq!(users.len(), 4);

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn read_roles_of_user() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let mut roles = User::from_id(db_connection, Uuid::from_u64_pair(0, 4))
                .expect("User")
                .roles(db_connection)
                .expect("Roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(roles[0].name(), "directory");
            assert_eq!(roles[0].display(), "Directory");
            assert_eq!(roles[1].name(), "user");
            assert_eq!(roles[1].display(), "User");
            assert_eq!(roles.len(), 2);

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn update_email() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // SETUP
            let mut user = User::from_email(db_connection, "test@example.com")
                .expect("User")
                .expect("An existing user");

            assert_eq!(user.email(), "test@example.com");
            assert!(user.verify_password(&Secret::new(String::from("test#7U5c"))));

            // TEST
            user.update_email(db_connection, "user5678@example.com")
                .expect("user");
            assert_eq!(user.email(), "user5678@example.com");
            assert!(user.verify_password(&Secret::new(String::from("test#7U5c"))));

            let mut users = User::list_all(db_connection).expect("users");
            users.sort_by(|a, b| a.email().cmp(b.email()));

            assert_eq!(users[0].email(), "admin@example.com");
            assert_eq!(users[1].email(), "dummy@example.com");
            assert_eq!(users[2].email(), "user5678@example.com");
            assert_eq!(users[3].email(), "user@example.com");
            assert_eq!(users.len(), 4);

            assert_eq!(users[2].id(), user.id());
            assert_eq!(users[2].email(), user.email());
            assert_eq!(users[2].password.0.expose_secret(), user.password.0.expose_secret());

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn update_password() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // SETUP
            let mut user = User::from_email(db_connection, "test@example.com")
                .expect("User")
                .expect("An existing user");

            assert_eq!(user.email(), "test@example.com");
            assert!(user.verify_password(&Secret::new(String::from("test#7U5c"))));

            // TEST
            user.update_password(db_connection, &Secret::new(String::from("test#FFcc")))
                .expect("user");
            assert_eq!(user.email(), "test@example.com");
            assert!(user.verify_password(&Secret::new(String::from("test#FFcc"))));

            let mut users = User::list_all(db_connection).expect("users");
            users.sort_by(|a, b| a.email().cmp(b.email()));

            assert_eq!(users[0].email(), "admin@example.com");
            assert_eq!(users[1].email(), "dummy@example.com");
            assert_eq!(users[2].email(), "test@example.com");
            assert_eq!(users[3].email(), "user@example.com");
            assert_eq!(users.len(), 4);

            assert_eq!(users[2].id(), user.id());
            assert_eq!(users[2].email(), user.email());
            assert_eq!(users[2].password.0.expose_secret(), user.password.0.expose_secret());

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn delete_user() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // SETUP
            let user_id = Uuid::from_u64_pair(0, 2);
            let user_dummy = User::from_id(db_connection, user_id)
                .expect("User");

            // BEFORE
            let mut users = User::list_all(db_connection).expect("users");
            users.sort_by(|a, b| a.email().cmp(b.email()));

            assert_eq!(users[0].email(), "admin@example.com");
            assert_eq!(users[1].email(), "dummy@example.com");
            assert_eq!(users[2].email(), "test@example.com");
            assert_eq!(users[3].email(), "user@example.com");
            assert_eq!(users.len(), 4);

            // TEST
            user_dummy.delete(db_connection)
                .expect("User deletion");

            // AFTER
            let mut users = User::list_all(db_connection).expect("users");
            users.sort_by(|a, b| a.email().cmp(b.email()));

            assert_eq!(users[0].email(), "admin@example.com");
            assert_eq!(users[1].email(), "test@example.com");
            assert_eq!(users[2].email(), "user@example.com");
            assert_eq!(users.len(), 3);

            let _ = User::from_id(db_connection, user_id)
                .expect_err("The user does not exist");

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn password_does_not_leak() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        let user = User::from_email(&mut db_connection, "user@example.com")
            .expect("User")
            .expect("An existing user");
        assert!(user.verify_password(&Secret::new(String::from("user#vX78"))));
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
        let user_json = format!(r#"{{ "user_id": "{uuid}", "email": "jsonny@example.com", "display": "Jsonny", "password": "{password}" }}"#);
        let user: User = serde_json::from_str(&user_json).expect("valid json");

        assert_eq!(user.id(), uuid);
        assert_eq!(user.email(), "jsonny@example.com");
        assert!(user.verify_password(&Secret::new(String::from("password"))));
    }
}

 */