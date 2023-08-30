//! This module contains the necessary functions and data structures for the subcommand `user`.

use clap::{Parser, Subcommand};
use secrecy::Secret;

use tusk_core::config::TuskConfigurationFile;
use tusk_core::error::Result;

/// User management.
///
/// This command allows to add, remove, list and, in general, administrate all the users.
#[derive(Parser, Debug)]
pub struct User {
    #[command(subcommand)]
    command: UserCommand
}

/// Enumerator containing the possible `user` commands.
#[derive(Subcommand, Debug)]
pub enum UserCommand {
    /// Adds a new user in the database.
    ///
    /// Asks for a password.
    Add {
        /// Name of the user.
        ///
        /// If omitted, will be asked.
        username: Option<String>
    },
    /// Lists all the users.
    List,
    /// Removes an user from the database.
    Remove {
        /// Name of the user.
        ///
        /// If omitted, will be asked.
        username: Option<String>
    },
}

/// Main entry point for the `user` command.
pub fn main(args: User) -> Result<()> {
    match args.command {
        UserCommand::Add { username } => add(username),
        UserCommand::List => list(),
        UserCommand::Remove { username } => remove(username)
    }
}

/// Adds a new user in the database.
pub fn add(username: Option<String>) -> Result<()> {
    let username = if let Some(username) = username {
        username
    } else {
        dialoguer::Input::new()
            .with_prompt("Specify username")
            .interact()?
    };
    let password = dialoguer::Password::new()
        .with_prompt(format!("Type password for user '{username}'"))
        .with_confirmation("Confirm password", "Password mismatching")
        .interact()?;
    let password = Secret::new(password);

    let tusk = TuskConfigurationFile::import()?
        .into_tusk()?;

    let mut db_connection = tusk.database_connect()?;

    let user = tusk_core::resources::User::create(&mut db_connection, username, password)
        .unwrap_or_else(|_| panic!("TODO: handle error"));

    println!("Created user {}", user.username());

    Ok(())
}
/// Lists all the users.
pub fn list() -> Result<()> {
    let tusk = TuskConfigurationFile::import()?
        .into_tusk()?;

    let mut db_connection = tusk.database_connect()?;

    let users = tusk_core::resources::User::read_all(&mut db_connection)
        .unwrap_or_else(|_| panic!("TODO: handle error"));

    println!("Username");
    println!("--------");
    for user in users {
        println!("{}", user.username());
    }

    Ok(())
}
/// Removes an user from the database.
pub fn remove(username: Option<String>) -> Result<()> {
    let username = if let Some(username) = username {
        username
    } else {
        dialoguer::Input::new()
            .with_prompt("Specify username")
            .interact()?
    };

    let tusk = TuskConfigurationFile::import()?
        .into_tusk()?;

    let mut db_connection = tusk.database_connect()?;

    let deleted = tusk_core::resources::User::delete_by_username(&mut db_connection, username)?;

    println!("Deleted {deleted} user");

    Ok(())
}