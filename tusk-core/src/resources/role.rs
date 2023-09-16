//! Data structures for the `role` table.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::TuskResult;
use crate::resources::User;

#[derive(Clone, Debug, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::role)]
#[diesel(check_for_backend(diesel::pg::Pg))]
/// Defines a role.
pub struct Role {
    #[serde(skip_serializing)]
    role_id: Uuid,
    name: String,
    display: String
}
impl Role {
    /// Inserts a new role in the table and returns the corresponding result.
    pub fn create<R: AsRef<str>, D: AsRef<str>>(db_connection: &mut PgConnection, name: R, display: D) -> TuskResult<Role> {
        use crate::schema::role;
        let name = name.as_ref();
        let display = display.as_ref();

        let role = diesel::insert_into(role::table)
            .values(
                (role::name.eq(name), role::display.eq(&display))
            ).get_result(db_connection)?;

        Ok(role)
    }

    /// Returns the ID of the current role.
    pub fn id(&self) -> Uuid { self.role_id }
    /// Returns the name of the current role.
    pub fn name(&self) -> &str { &self.name }
    /// Returns the display name of the current role.
    pub fn display(&self) -> &str { &self.display }

    /// Assigns the role to the given user.
    pub fn assign_to(&self, db_connection: &mut PgConnection, user: &User) -> TuskResult<()> {
        use crate::schema::user_role;

        diesel::insert_into(user_role::table)
            .values(
                (user_role::user_id.eq(user.id()), user_role::role_id.eq(self.role_id))
            ).execute(db_connection)?;

        Ok(())
    }
    /// Deletes the role.
    ///
    /// **Warning:** this operation is irreversible; use with caution.
    pub fn delete(self, db_connection: &mut PgConnection) -> TuskResult<()> {
        use crate::schema::role;

        let selected = role::table
            .filter(role::role_id.eq(self.role_id));

        let _ = diesel::delete(selected)
            .execute(db_connection)?;

        Ok(())
    }
    /// Reads a role from the table, given the role ID.
    pub fn from_id(db_connection: &mut PgConnection, role_id: Uuid) -> TuskResult<Role> {
        use crate::schema::role;

        let role = role::table
            .filter(role::role_id.eq(role_id))
            .first(db_connection)?;

        Ok(role)
    }
    /// Reads a role from the table, given the name.
    pub fn from_name<R: AsRef<str>>(db_connection: &mut PgConnection, name: R) -> TuskResult<Option<Role>> {
        use crate::schema::role;
        let name = name.as_ref();

        let role = role::table
            .filter(role::name.eq(name))
            .first(db_connection)
            .optional()?;

        Ok(role)
    }
    /// Reads all roles from the table.
    pub fn list_all(db_connection: &mut PgConnection) -> TuskResult<Vec<Role>> {
        use crate::schema::role;

        let roles = role::table
            .load(db_connection)?;

        Ok(roles)
    }
    /// Removes the role from the given user.
    pub fn revoke_from(&self, db_connection: &mut PgConnection, user: &User) -> TuskResult<()> {
        use crate::schema::user_role;

        diesel::delete(user_role::table)
            .filter(user_role::user_id.eq(user.id()))
            .filter(user_role::role_id.eq(self.role_id))
            .execute(db_connection)?;

        Ok(())
    }
    /// Returns a list of all the users assigned to this role.
    pub fn users(&self, db_connection: &mut PgConnection) -> TuskResult<Vec<User>> {
        use crate::schema::{user, role, user_role};

        let users = user::table
            .inner_join(user_role::table.inner_join(role::table))
            .filter(role::role_id.eq(self.role_id))
            .select(User::as_select())
            .load::<User>(db_connection)?;

        Ok(users)
    }
}

/*
#[cfg(test)]
mod test {
    use std::convert::Infallible;
    use diesel::Connection;
    use uuid::Uuid;
    use crate::config::TEST_CONFIGURATION;
    use crate::resources::{Role, User};

    #[test]
    fn create_role() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let role = Role::create(db_connection, "fancy_role", "Fancy Role")
                .expect("Role");

            assert_eq!(role.name(), "fancy_role");
            assert_eq!(role.display(), "Fancy Role");

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn cannot_create_role_twice() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            Role::create(db_connection, "fancy_role", "Fancy Role")
                .expect("Role");

            Role::create(db_connection, "fancy_role", "Fancy Role")
                .expect_err("Role already exists");

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn read_role_by_id() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // SETUP
            let role = Role::create(db_connection, "fancy_role", "Fancy Role")
                .expect("Role");
            let role_id = role.id();

            // TEST
            let role = Role::from_id(db_connection, role_id)
                .expect("Role");
            assert_eq!(role.name(), "fancy_role");
            assert_eq!(role.display(), "Fancy Role");

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn read_role_by_name() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let role = Role::from_name(db_connection, "admin")
                .expect("Role")
                .expect("An existing role");
            assert_eq!(role.name(), "admin");
            assert_eq!(role.display(), "Admin");

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn read_roles() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let mut roles = Role::list_all(db_connection).expect("roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(roles[0].name(), "admin");
            assert_eq!(roles[0].display(), "Admin");
            assert_eq!(roles[1].name(), "directory");
            assert_eq!(roles[1].display(), "Directory");
            assert_eq!(roles[2].name(), "user");
            assert_eq!(roles[2].display(), "User");
            assert_eq!(roles.len(), 3);

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn read_users_by_role() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let mut users = Role::from_name(db_connection, "directory")
                .expect("Role")
                .expect("An existing role")
                .users(db_connection)
                .expect("Users");
            users.sort_by(|a, b| a.email().cmp(b.email()));

            assert_eq!(users[0].email(), "admin@example.com");
            assert_eq!(users[1].email(), "test@example.com");
            assert_eq!(users[2].email(), "user@example.com");
            assert_eq!(users.len(), 3);

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn delete_role() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // SETUP
            let role_directory = Role::from_name(db_connection, "directory")
                .expect("Role")
                .expect("An existing role");
            let role_id = role_directory.id();

            // BEFORE
            let mut roles = Role::list_all(db_connection).expect("roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(roles[0].name(), "admin");
            assert_eq!(roles[1].name(), "directory");
            assert_eq!(roles[2].name(), "user");
            assert_eq!(roles.len(), 3);

            // TEST
            role_directory.delete(db_connection)
                .expect("Role deletion");

            // AFTER
            let mut roles = Role::list_all(db_connection).expect("users");
            roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(roles[0].name(), "admin");
            assert_eq!(roles[1].name(), "user");
            assert_eq!(roles.len(), 2);

            let _ = Role::from_id(db_connection, role_id)
                .expect_err("The role does not exist");

            Ok::<_, Infallible>(())
        });
    }

    #[test]
    fn assign_to_user() {
        let mut db_connection = TEST_CONFIGURATION.db()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let user_user = User::from_id(db_connection, Uuid::from_u64_pair(0, 4))
                .expect("User");
            let user_test = User::from_id(db_connection, Uuid::from_u64_pair(0, 1))
                .expect("User");

            // Initial setting.
            let mut roles = user_user.roles(db_connection)
                .expect("Roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));
            assert_eq!(roles[0].name(), "directory");
            assert_eq!(roles[1].name(), "user");
            assert_eq!(roles.len(), 2);
            // Initial setting for other user.
            let mut roles = user_test.roles(db_connection)
                .expect("Roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));
            assert_eq!(roles[0].name(), "directory");
            assert_eq!(roles[1].name(), "user");
            assert_eq!(roles.len(), 2);

            // Role assignment to user.
            Role::from_name(db_connection, "admin")
                .expect("Role")
                .expect("An existing role")
                .assign_to(db_connection, &user_user)
                .expect("Role assignment");

            // New roles.
            let mut roles = user_user.roles(db_connection)
                .expect("Roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));
            assert_eq!(roles[0].name(), "admin");
            assert_eq!(roles[1].name(), "directory");
            assert_eq!(roles[2].name(), "user");
            assert_eq!(roles.len(), 3);
            // Other user remains unchanged.
            let mut roles = user_test.roles(db_connection)
                .expect("Roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));
            assert_eq!(roles[0].name(), "directory");
            assert_eq!(roles[1].name(), "user");
            assert_eq!(roles.len(), 2);

            // Remove role from user.
            Role::from_name(db_connection, "admin")
                .expect("Role")
                .expect("An existing role")
                .revoke_from(db_connection, &user_user)
                .expect("Role assignment");

            // Back to normal.
            let mut roles = user_user.roles(db_connection)
                .expect("Roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));
            assert_eq!(roles[0].name(), "directory");
            assert_eq!(roles[1].name(), "user");
            assert_eq!(roles.len(), 2);
            // Other user still unchanged.
            let mut roles = user_test.roles(db_connection)
                .expect("Roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));
            assert_eq!(roles[0].name(), "directory");
            assert_eq!(roles[1].name(), "user");
            assert_eq!(roles.len(), 2);

            Ok::<_, ()>(())
        });
    }
}
 */