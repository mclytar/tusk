//! This module contains all the database resources, parsed as Rust data structures.

pub mod role;
pub mod user;

pub use role::Role;
pub use user::User;

#[cfg(test)]
mod test {
    use diesel::Connection;
    use secrecy::Secret;
    use crate::config::TEST_CONFIGURATION;
    use crate::resources::{Role, User};

    #[test]
    fn user_crud() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // CREATE

            let user = User::create(db_connection, "user1234", Secret::new(String::from("password1234")))
                .expect("user");

            let user_id = user.id();
            assert_eq!(user.username(), "user1234");
            assert!(user.verify_password(&Secret::new(String::from("password1234"))));
            assert!(!user.verify_password(&Secret::new(String::from("another password"))));

            // READ

            let user = User::read(db_connection, user_id)
                .expect("user");
            assert_eq!(user.username(), "user1234");
            assert!(user.verify_password(&Secret::new(String::from("password1234"))));

            let user = User::read_by_username(db_connection, "user")
                .expect("user");
            assert_eq!(user.username(), "user");
            assert!(user.verify_password(&Secret::new(String::from("user"))));

            let mut directory_users = User::read_by_role_name(db_connection, "directory")
                .expect("users");
            directory_users.sort_by(|a, b| a.username().cmp(b.username()));
            assert_eq!(directory_users[0].username(), "admin");
            assert_eq!(directory_users[1].username(), "test");
            assert_eq!(directory_users[2].username(), "user");
            assert_eq!(directory_users.len(), 3);

            let mut users = User::read_all(db_connection)
                .expect("users");
            users.sort_by(|a, b| a.username().cmp(b.username()));
            assert_eq!(users[0].username(), "admin");
            assert_eq!(users[1].username(), "dummy");
            assert_eq!(users[2].username(), "test");
            assert_eq!(users[3].username(), "user");
            assert_eq!(users[4].username(), "user1234");
            assert_eq!(users.len(), 5);

            // UPDATE

            let user = User::read(db_connection, user_id)
                .expect("user");
            assert_eq!(user.username(), "user1234");
            assert!(user.verify_password(&Secret::new(String::from("password1234"))));
            let user = user.update_username(db_connection, "user5678")
                .expect("user");
            assert_eq!(user.username(), "user5678");
            assert!(user.verify_password(&Secret::new(String::from("password1234"))));
            let user = user.update_password(db_connection, &Secret::new(String::from("password5678")))
                .expect("user");
            assert_eq!(user.username(), "user5678");
            assert!(user.verify_password(&Secret::new(String::from("password5678"))));

            // DELETE

            assert_eq!(User::delete_by_id(db_connection, user_id).expect("deleted"), 1);
            assert_eq!(User::delete_by_id(db_connection, user_id).expect("deleted"), 0);
            assert_eq!(User::delete_by_username(db_connection, "dummy").expect("deleted"), 1);
            assert_eq!(User::delete_by_username(db_connection, "dummy").expect("deleted"), 0);

            let mut users = User::read_all(db_connection)
                .expect("users");
            users.sort_by(|a, b| a.username().cmp(b.username()));
            assert_eq!(users[0].username(), "admin");
            assert_eq!(users[1].username(), "test");
            assert_eq!(users[2].username(), "user");
            assert_eq!(users.len(), 3);

            Ok::<_, ()>(())
        });
    }

    #[test]
    fn role_crud() {
        let mut db_connection = TEST_CONFIGURATION.database_connect()
            .expect("database connection");

        db_connection.test_transaction(|db_connection| {
            // CREATE

            let role = Role::create(db_connection, "fancy_role", "Fancy Role")
                .expect("role");

            let role_id = role.id();
            assert_eq!(role.name(), "fancy_role");
            assert_eq!(role.display(), "Fancy Role");

            // READ

            let role = Role::read(db_connection, role_id)
                .expect("role");
            assert_eq!(role.name(), "fancy_role");
            assert_eq!(role.display(), "Fancy Role");

            let role_admin = Role::read_by_name(db_connection, "admin")
                .expect("role");
            assert_eq!(role_admin.name(), "admin");
            assert_eq!(role_admin.display(), "Admin");

            let mut roles_of_admin = Role::read_by_user_username(db_connection, "admin")
                .expect("roles");
            roles_of_admin.sort_by(|a, b| a.name().cmp(b.name()));
            assert_eq!(roles_of_admin[0].name(), "admin");
            assert_eq!(roles_of_admin[1].name(), "directory");
            assert_eq!(roles_of_admin[2].name(), "user");
            assert_eq!(roles_of_admin[0].display(), "Admin");
            assert_eq!(roles_of_admin[1].display(), "Directory");
            assert_eq!(roles_of_admin[2].display(), "User");
            assert_eq!(roles_of_admin.len(), 3);

            let mut roles = Role::read_all(db_connection)
                .expect("roles");
            roles.sort_by(|a, b| a.name().cmp(b.name()));
            assert_eq!(roles[0].name(), "admin");
            assert_eq!(roles[1].name(), "directory");
            assert_eq!(roles[2].name(), "fancy_role");
            assert_eq!(roles[3].name(), "user");
            assert_eq!(roles[0].display(), "Admin");
            assert_eq!(roles[1].display(), "Directory");
            assert_eq!(roles[2].display(), "Fancy Role");
            assert_eq!(roles[3].display(), "User");
            assert_eq!(roles.len(), 4);

            // UPDATE

            // -- NOT YET IMPLEMENTED --

            // ASSIGN (i.e. CREATE/DELETE on User >-< Role relation)

            let fancy_role_users = User::read_by_role_name(db_connection, "fancy_role")
                .expect("users");
            assert_eq!(fancy_role_users.len(), 0);

            Role::assign(db_connection, "fancy_role")
                .to("admin")
                .expect("role assignment");

            let mut fancy_role_users = User::read_by_role_name(db_connection, "fancy_role")
                .expect("users");
            fancy_role_users.sort_by(|a, b| a.username().cmp(b.username()));
            assert_eq!(fancy_role_users[0].username(), "admin");
            assert_eq!(fancy_role_users.len(), 1);

            let mut role_assign = Role::assign(db_connection, "fancy_role");
            role_assign.to("dummy").expect("role assignment");
            role_assign.to("test").expect("role assignment");
            role_assign.cancel_from("admin").expect("role assignment");

            let mut fancy_role_users = User::read_by_role_name(db_connection, "fancy_role")
                .expect("users");
            fancy_role_users.sort_by(|a, b| a.username().cmp(b.username()));
            assert_eq!(fancy_role_users[0].username(), "dummy");
            assert_eq!(fancy_role_users[1].username(), "test");
            assert_eq!(fancy_role_users.len(), 2);

            // DELETE

            assert_eq!(Role::delete_by_id(db_connection, role_id).expect("deleted"), 1);
            assert_eq!(Role::delete_by_id(db_connection, role_id).expect("deleted"), 0);
            assert_eq!(Role::delete_by_name(db_connection, "directory").expect("deleted"), 1);
            assert_eq!(Role::delete_by_name(db_connection, "directory").expect("deleted"), 0);

            Ok::<_, ()>(())
        });
    }
}