//! Data structures for the `role` table.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;

/// Partially built query to assign a role to an user.
pub struct RoleAssign<'a> {
    db_connection: &'a mut PgConnection,
    name: String
}
impl<'a> RoleAssign<'a> {
    /// Assigns the role to the specified user.
    pub fn to<S: AsRef<str>>(&mut self, username: S) -> Result<()> {
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
    pub fn cancel_from<S: AsRef<str>>(&mut self, username: S) -> Result<()> {
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
    pub fn create<R: AsRef<str>, D: AsRef<str>>(db_connection: &mut PgConnection, name: R, display: D) -> Result<Role> {
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
    pub fn read(db_connection: &mut PgConnection, role_id: Uuid) -> Result<Role> {
        use crate::schema::role;

        let role = role::table
            .filter(role::role_id.eq(role_id))
            .first(db_connection)?;

        Ok(role)
    }
    /// Reads a role from the table, given the name.
    pub fn read_by_name<R: AsRef<str>>(db_connection: &mut PgConnection, name: R) -> Result<Role> {
        use crate::schema::role;
        let name = name.as_ref();

        let role = role::table
            .filter(role::name.eq(name))
            .first(db_connection)?;

        Ok(role)
    }
    /// Reads all roles from the table.
    pub fn read_all(db_connection: &mut PgConnection) -> Result<Vec<Role>> {
        use crate::schema::role;

        let roles = role::table
            .load(db_connection)?;

        Ok(roles)
    }
    /// Reads all the roles assigned to a specific user, given by username.
    pub fn read_by_user_username<S: AsRef<str>>(db_connection: &mut PgConnection, username: S) -> Result<Vec<Role>> {
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
    pub fn delete_by_id(db_connection: &mut PgConnection, role_id: Uuid) -> Result<usize> {
        use crate::schema::role;

        let selected = role::table
            .filter(role::role_id.eq(role_id));

        let num_deleted = diesel::delete(selected)
            .execute(db_connection)?;

        Ok(num_deleted)
    }
    /// Deletes a role given the role name.
    pub fn delete_by_name<R: AsRef<str>>(db_connection: &mut PgConnection, name: R) -> Result<usize> {
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