//! Data structures for the `role` table.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::TuskResult;

/// Partially built query to assign a role to an user.
pub struct RoleAssign<'a> {
    db_connection: &'a mut PgConnection,
    name: String
}
impl<'a> RoleAssign<'a> {
    /// Assigns the role to the specified user.
    pub fn to<S: AsRef<str>>(&mut self, username: S) -> TuskResult<()> {
        use crate::schema::user_role;
        let username = username.as_ref();

        self.db_connection.transaction(|db_connection| {
            let role = Role::read_by_name(db_connection, &self.name)?;
            let user = crate::resources::User::read_by_username(db_connection, username)?;

            diesel::insert_into(user_role::table)
                .values(
                    (user_role::user_id.eq(user.id()), user_role::role_id.eq(role.id()))
                ).execute(db_connection)?;

            Ok(())
        })
    }
    /// Removes the role from the specified user.
    pub fn cancel_from<S: AsRef<str>>(&mut self, username: S) -> TuskResult<()> {
        use crate::schema::user_role;
        let username = username.as_ref();

        self.db_connection.transaction(|db_connection| {
            let role = Role::read_by_name(db_connection, &self.name)?;
            let user = crate::resources::User::read_by_username(db_connection, username)?;

            diesel::delete(user_role::table)
                .filter(user_role::user_id.eq(user.id()))
                .filter(user_role::role_id.eq(role.id()))
                .execute(db_connection)?;

            Ok(())
        })
    }
}

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
    /// Reads a role from the table, given the role ID.
    pub fn read(db_connection: &mut PgConnection, role_id: Uuid) -> TuskResult<Role> {
        use crate::schema::role;

        let role = role::table
            .filter(role::role_id.eq(role_id))
            .first(db_connection)?;

        Ok(role)
    }
    /// Reads a role from the table, given the name.
    pub fn read_by_name<R: AsRef<str>>(db_connection: &mut PgConnection, name: R) -> TuskResult<Role> {
        use crate::schema::role;
        let name = name.as_ref();

        let role = role::table
            .filter(role::name.eq(name))
            .first(db_connection)?;

        Ok(role)
    }
    /// Reads all roles from the table.
    pub fn read_all(db_connection: &mut PgConnection) -> TuskResult<Vec<Role>> {
        use crate::schema::role;

        let roles = role::table
            .load(db_connection)?;

        Ok(roles)
    }
    /// Reads all the roles assigned to a specific user, given by username.
    pub fn read_by_user_username<S: AsRef<str>>(db_connection: &mut PgConnection, username: S) -> TuskResult<Vec<Role>> {
        let username = username.as_ref();

        use crate::schema::{user, role, user_role};

        let roles = user::table
            .inner_join(user_role::table.inner_join(role::table))
            .filter(user::username.eq(username))
            .select(Role::as_select())
            .load::<Role>(db_connection)?;

        Ok(roles)
    }
    /// Deletes a role given the user ID.
    pub fn delete_by_id(db_connection: &mut PgConnection, role_id: Uuid) -> TuskResult<usize> {
        use crate::schema::role;

        let selected = role::table
            .filter(role::role_id.eq(role_id));

        let num_deleted = diesel::delete(selected)
            .execute(db_connection)?;

        Ok(num_deleted)
    }
    /// Deletes a role given the role name.
    pub fn delete_by_name<R: AsRef<str>>(db_connection: &mut PgConnection, name: R) -> TuskResult<usize> {
        let name = name.as_ref();

        use crate::schema::role;

        let selected = role::table
            .filter(role::name.eq(name));

        let num_deleted = diesel::delete(selected)
            .execute(db_connection)?;

        Ok(num_deleted)
    }
    /// Returns the ID of the current role.
    pub fn id(&self) -> Uuid {
        self.role_id
    }
    /// Returns the name of the current role.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Returns the display name of the current role.
    pub fn display(&self) -> &str {
        &self.display
    }
    /// Returns a partially built query to assign the specified role to an user.
    pub fn assign<S: AsRef<str>>(db_connection: &mut PgConnection, name: S) -> RoleAssign {
        RoleAssign { name: name.as_ref().to_owned(), db_connection }
    }
}

#[cfg(test)]
mod test {
    use diesel::Connection;
    use crate::config::TEST_CONFIGURATION;
    use crate::resources::{Role};

    #[test]
    fn create_role() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let role = Role::create(db_connection, "fancy_role", "Fancy Role")
                .expect("role");

            assert_eq!(role.name(), "fancy_role");
            assert_eq!(role.display(), "Fancy Role");

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn cannot_create_role_twice() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            Role::create(db_connection, "fancy_role", "Fancy Role")
                .expect("role");

            assert!(Role::create(db_connection, "fancy_role", "Fancy Role 2").is_err());

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn read_role_by_id() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // SETUP
            let role = Role::create(db_connection, "fancy_role", "Fancy Role")
                .expect("role");
            let role_id = role.id();

            // TEST
            let role = Role::read(db_connection, role_id)
                .expect("role");
            assert_eq!(role.name(), "fancy_role");
            assert_eq!(role.display(), "Fancy Role");

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn read_role_by_name() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let role = Role::read_by_name(db_connection, "admin")
                .expect("role");
            assert_eq!(role.name(), "admin");
            assert_eq!(role.display(), "Admin");

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn read_roles() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let mut roles = Role::read_all(db_connection).expect("roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(roles[0].name(), "admin");
            assert_eq!(roles[0].display(), "Admin");
            assert_eq!(roles[1].name(), "directory");
            assert_eq!(roles[1].display(), "Directory");
            assert_eq!(roles[2].name(), "user");
            assert_eq!(roles[2].display(), "User");
            assert_eq!(roles.len(), 3);

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn read_roles_of_user() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let mut roles = Role::read_by_user_username(db_connection, "user").expect("roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(roles[0].name(), "directory");
            assert_eq!(roles[0].display(), "Directory");
            assert_eq!(roles[1].name(), "user");
            assert_eq!(roles[1].display(), "User");
            assert_eq!(roles.len(), 2);

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn delete_role_by_id() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // SETUP
            let role_directory = Role::read_by_name(db_connection, "directory").expect("role");
            let role_id = role_directory.id();

            // BEFORE
            let mut roles = Role::read_all(db_connection).expect("roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(roles[0].name(), "admin");
            assert_eq!(roles[1].name(), "directory");
            assert_eq!(roles[2].name(), "user");
            assert_eq!(roles.len(), 3);

            // TEST
            let deleted = Role::delete_by_id(db_connection, role_id).expect("role deletion");
            assert_eq!(deleted, 1);

            // AFTER
            let mut roles = Role::read_all(db_connection).expect("users");
            roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(roles[0].name(), "admin");
            assert_eq!(roles[1].name(), "user");
            assert_eq!(roles.len(), 2);

            // TEST DOUBLE DELETE DOES NOTHING
            let deleted = Role::delete_by_id(db_connection, role_id).expect("user deletion");
            assert_eq!(deleted, 0);

            // AFTER DOUBLE DELETE
            let mut roles = Role::read_all(db_connection).expect("users");
            roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(roles[0].name(), "admin");
            assert_eq!(roles[1].name(), "user");
            assert_eq!(roles.len(), 2);

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn delete_role_by_name() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // BEFORE
            let mut roles = Role::read_all(db_connection).expect("roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(roles[0].name(), "admin");
            assert_eq!(roles[1].name(), "directory");
            assert_eq!(roles[2].name(), "user");
            assert_eq!(roles.len(), 3);

            // TEST
            let deleted = Role::delete_by_name(db_connection, "directory").expect("role deletion");
            assert_eq!(deleted, 1);

            // AFTER
            let mut roles = Role::read_all(db_connection).expect("users");
            roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(roles[0].name(), "admin");
            assert_eq!(roles[1].name(), "user");
            assert_eq!(roles.len(), 2);

            // TEST DOUBLE DELETE DOES NOTHING
            let deleted = Role::delete_by_name(db_connection, "directory").expect("user deletion");
            assert_eq!(deleted, 0);

            // AFTER DOUBLE DELETE
            let mut roles = Role::read_all(db_connection).expect("users");
            roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(roles[0].name(), "admin");
            assert_eq!(roles[1].name(), "user");
            assert_eq!(roles.len(), 2);

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn assign_to_user() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            let mut roles = Role::read_by_user_username(db_connection, "user").expect("roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(roles[0].name(), "directory");
            assert_eq!(roles[1].name(), "user");
            assert_eq!(roles.len(), 2);

            Role::assign(db_connection, "admin")
                .to("user")
                .expect("role assignment");

            let mut user_roles = Role::read_by_user_username(db_connection, "user").expect("roles");
            user_roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(user_roles[0].name(), "admin");
            assert_eq!(user_roles[1].name(), "directory");
            assert_eq!(user_roles[2].name(), "user");
            assert_eq!(user_roles.len(), 3);

            let mut test_roles = Role::read_by_user_username(db_connection, "test").expect("roles");
            test_roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(test_roles[0].name(), "directory");
            assert_eq!(test_roles[1].name(), "user");
            assert_eq!(test_roles.len(), 2);

            let mut role_assign = Role::assign(db_connection, "directory");

            role_assign.cancel_from("user").expect("role assignment");
            role_assign.cancel_from("test").expect("role assignment");

            drop(role_assign);

            let mut user_roles = Role::read_by_user_username(db_connection, "user").expect("roles");
            user_roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(user_roles[0].name(), "admin");
            assert_eq!(user_roles[1].name(), "user");
            assert_eq!(user_roles.len(), 2);

            let mut test_roles = Role::read_by_user_username(db_connection, "test").expect("roles");
            test_roles.sort_by(|a, b| a.name().cmp(b.name()));

            assert_eq!(test_roles[0].name(), "user");
            assert_eq!(test_roles.len(), 1);

            Ok::<_, ()>(())
        });
    }
}