//! This module contains the necessary functions and data structures for the subcommand `user`.

use clap::{Parser, Subcommand};
use secrecy::{ExposeSecret, Secret};
use tabled::{Table, Tabled};
use tabled::settings::Style;

use tusk_core::config::TuskConfigurationFile;
use tusk_core::error::{TuskError, TuskResult};
use tusk_core::{Connection, DieselError, Message, PgConnection};

/// Encapsulates the user data to be displayed in a table.
#[derive(Tabled)]
#[tabled(rename_all = "CamelCase")]
pub struct UserTable {
    identification: String,
    name: String,
    roles: String
}
impl UserTable {
    /// Creates a table row from an existing user.
    pub fn from_user(db_connection: &mut PgConnection, user: &tusk_core::resources::User) -> TuskResult<UserTable> {
        let identification = format!("{}", user.email());
        let name = user.display().to_owned();
        let roles = user.roles(db_connection)?
            .into_iter()
            .map(|r| r.display().to_owned())
            .collect::<Vec<String>>()
            .join(",");
        Ok(UserTable {
            identification,
            name,
            roles
        })
    }
    /// Creates a table row from an existing user, displaying also the UUID of the user.
    pub fn from_user_with_uuid(db_connection: &mut PgConnection, user: &tusk_core::resources::User) -> TuskResult<UserTable> {
        let identification = format!("• {}\n{}", user.email(), console::style(user.id()).yellow());
        let name = user.display().to_owned();
        let roles = user.roles(db_connection)?
            .into_iter()
            .map(|r| r.display().to_owned())
            .collect::<Vec<String>>()
            .join(",");
        Ok(UserTable {
            identification,
            name,
            roles
        })
    }
}

/// User management.
///
/// This command allows to add, remove, list and, in general, administrate all the users.
#[derive(Parser, Debug)]
pub struct User {
    #[command(subcommand)]
    command: UserCommand,
}

/// Enumerator containing the possible `user` commands.
#[derive(Subcommand, Debug)]
pub enum UserCommand {
    /// Adds a new user in the database.
    ///
    /// Asks for a password.
    Add(UserCommandAdd),
    /// Assigns a role to a given user.
    Assign {
        /// User to which assign the role.
        #[clap(long = "to")]
        user: String,
        /// Role to assign.
        role: String,
    },
    /// Lists all the users.
    List {
        /// Displays the ID of every user.
        #[clap(long)]
        uuid: bool
    },
    /// Removes an user from the database.
    Remove {
        /// Email of the user.
        ///
        /// If omitted, will be asked.
        user: Option<String>
    },
    /// Revokes a role from a given user.
    Revoke {
        /// User from which revoke the role.
        #[clap(long = "from")]
        user: String,
        /// Role to revoke.
        role: String,
    },
}

/// Adds a new user in the database.
#[derive(Parser, Debug)]
pub struct UserCommandAdd {
    /// Email of the user.
    ///
    /// If omitted, will be asked.
    email: Option<String>,
    /// Displayed name in the application.
    ///
    /// If omitted, will be the same as the email.
    #[clap(short='D', long="display-as")]
    display_as: Option<String>,
    /// Set this flag to set now a password.
    ///
    /// If this flag is not set, then an activation link is sent to the given email.
    #[clap(short='P', long="with-password")]
    with_password: bool,
}
impl UserCommandAdd {
    /// Completes the command line arguments by asking further information to the user, if needed;
    /// then, collects all the information into a single [`UserAddParameters`] structure.
    pub fn complete(self) -> TuskResult<UserAddParameters> {
        let email = if let Some(email) = self.email {
            email
        } else {
            dialoguer::Input::new()
                .with_prompt("Email of the new account")
                .interact()?
        };

        let name = if let Some(display_as) = self.display_as {
            display_as
        } else {
            email.clone()
        };

        let password = if self.with_password {
            let prompt = if email == name {
                format!("Type password for user '{email}'")
            } else {
                format!("Type password for user '{name} <{email}>'")
            };

            let password = dialoguer::Password::new()
                .with_prompt(prompt)
                .with_confirmation("Confirm password", "Password mismatching")
                .interact()?;

            Some(Secret::from(password))
        } else {
            None
        };

        Ok(UserAddParameters {
            email,
            name,
            password
        })
    }
}
/// Contains all the necessary information to create a new user.
pub struct UserAddParameters {
    email: String,
    name: String,
    password: Option<Secret<String>>
}
impl UserAddParameters {
    /// Returns the display name of the new user, using the email as default if it was not given
    /// as a parameter.
    pub fn display(&self) -> &str {
        &self.name
    }
    /// Returns the email of the new user, asking for it if it was not given as a parameter.
    pub fn email(&self) -> &str {
        &self.email
    }
    /// Returns the password of the new user, asking for it if it was not given as a parameter and
    /// the option `--with-password` was specified.
    pub fn password(&self) -> Option<&Secret<String>> {
        self.password.as_ref()
    }
    /// Runs the subcommand, adding a new user in the database.
    pub fn run(self) -> TuskResult<()> {
        let tusk = TuskConfigurationFile::import_from_default_locations()?
            .into_tusk()?;
        let mut db_connection = tusk.db()?;
        let server_email = "noreply@localhost";
        let server_support_email = "support@localhost";
        let server_address = tusk.www_domain();

        let email = self.email();
        let name = self.display();

        let email_receiver = if email == name {
            log::info!("User `{email}` created successfully.");
            email.to_owned()
        } else {
            log::info!("User `{name} <{email}>` created successfully.");
            format!("{name} <{email}>")
        };

        if let Some(password) = self.password() {
            let (_, None) = tusk_core::resources::User::builder(email)
                .display(name)
                .password(password.expose_secret())
                .build(&mut db_connection)? else { unreachable!() };

            let message = Message::builder()
                .from(format!("Tusk Server <{server_email}>").parse().unwrap())
                .to(email_receiver.parse().unwrap())
                .subject("Account creation")
                .body(format!(r#"Hello, {name}!
An account has been created for you at https://{server_address}/ and a password has already been set.
If you requested the account, you do not have any more steps to perform: simply go to https://{server_address}/login and use your credentials to log in.
If, however, you think that this is a mistake, please write an email to {server_support_email} and explain the situation.

Have a nice stay in this server!

Best,
Tusk"#))
                .unwrap();
            tusk.send_email(&message)?;
        } else {
            let (_, Some(request)) = tusk_core::resources::User::builder(email)
                .display(name)
                .build(&mut db_connection)? else { unreachable!() };

            let token = request.token();

            let message = Message::builder()
                .from(format!("Tusk Server <{server_email}>").parse().unwrap())
                .to(email_receiver.parse().unwrap())
                .subject("Account creation")
                .body(format!(r#"Hello, {name}!
An account has been created for you at https://{server_address}/, but a password has not yet been set.
If you requested the account, you can set up a password by visiting https://{server_address}/password_reset/verify?token={token} and following the steps.
If, however, you think that this is a mistake, please write an email to {server_support_email} and explain the situation.

Note: the above link expires after 24 hours. In this case, you can request a new link by visiting https://{server_address}/password_reset/request and following the steps.

Have a nice stay in this server!

Best,
Tusk"#))
                .unwrap();
            tusk.send_email(&message)?;
        }

        Ok(())
    }
}

/// Main entry point for the `user` command.
pub fn main(args: User) -> TuskResult<()> {
    match args.command {
        UserCommand::Add(add) => add.complete()?.run(),
        UserCommand::Assign { user, role } => assign(role, user),
        UserCommand::List{ uuid } => list(uuid),
        UserCommand::Remove { user } => remove(user),
        UserCommand::Revoke { user, role } => revoke(role, user),
    }
}

/// Assigns the specified `role` to the given `user`.
pub fn assign(role: String, user: String) -> TuskResult<()> {
    let tusk = TuskConfigurationFile::import_from_default_locations()?
        .into_tusk()?;
    let mut db_connection = tusk.db()?;

    let user = db_connection.transaction(|db_connection| {
        let user = tusk_core::resources::User::from_email(db_connection, &user)?
            .ok_or(DieselError::NotFound)?;
        tusk_core::resources::Role::from_name(db_connection, &role)?
            .ok_or(DieselError::NotFound)?
            .assign_to(db_connection, &user)?;
        Ok::<_, TuskError>(user)
    })?;

    let user_directory = user.directory(&tusk)?;
    if &role == "directory" && !user_directory.exists() {
        log::warn!("Warning: path `{}` does not exist.", user_directory.display());
    }
    log::info!("Done!");

    Ok(())
}

/// Lists all the users.
pub fn list(uuid: bool) -> TuskResult<()> {
    let tusk = TuskConfigurationFile::import_from_default_locations()?
        .into_tusk()?;
    let mut db_connection = tusk.db()?;

    let table: Vec<UserTable> = db_connection.transaction(|db_connection| {
        tusk_core::resources::User::list_all(db_connection)?
            .into_iter()
            .map(|u| if uuid {
                UserTable::from_user_with_uuid(db_connection, &u)
            } else {
                UserTable::from_user(db_connection, &u)
            })
            .collect()
    })?;

    let mut table = Table::new(table);
    table.with(Style::sharp());

    println!("{table}");

    Ok(())
}

/// Removes an user from the database.
pub fn remove(email: Option<String>) -> TuskResult<()> {
    let tusk = TuskConfigurationFile::import_from_default_locations()?
        .into_tusk()?;
    let mut db_connection = tusk.db()?;

    let email = if let Some(email) = email {
        email
    } else {
        dialoguer::Input::new()
            .with_prompt("Email of the account to be removed")
            .interact()?
    };

    db_connection.transaction(|db_connection| {
        tusk_core::resources::User::from_email(db_connection, email)?
            .ok_or(DieselError::NotFound)?
            .delete(db_connection)?;
        Ok::<_, TuskError>(())
    })?;

    log::info!("User has been deleted");

    Ok(())
}

/// Revokes the specified `role` to the given `user`.
pub fn revoke(role: String, user: String) -> TuskResult<()> {
    let tusk = TuskConfigurationFile::import_from_default_locations()?
        .into_tusk()?;
    let mut db_connection = tusk.db()?;

    let user = db_connection.transaction(|db_connection| {
        let user = tusk_core::resources::User::from_email(db_connection, &user)?
            .ok_or(DieselError::NotFound)?;
        tusk_core::resources::Role::from_name(db_connection, &role)?
            .ok_or(DieselError::NotFound)?
            .revoke_from(db_connection, &user)?;
        Ok::<_, TuskError>(user)
    })?;

    let user_directory = user.directory(&tusk)?;
    if &role == "directory" && user_directory.exists() {
        log::warn!("Warning: path `{}` still exists.", user_directory.display());
    }
    log::info!("Done!");

    Ok(())
}