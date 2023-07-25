use clap::{Parser, Subcommand};
use secrecy::Secret;

use tusk_backend::config::TuskConfigurationFile;
use tusk_backend::error::Result;

#[derive(Parser, Debug)]
pub struct User {
    #[command(subcommand)]
    command: UserCommand
}

#[derive(Subcommand, Debug)]
pub enum UserCommand {
    Add {
        /// Name of the user.
        ///
        /// If omitted, will be asked.
        username: Option<String>
    },
    List,
    Delete,
}

#[derive(Parser, Debug)]
pub struct UserAddParams {
    /// Name of the user.
    ///
    /// If omitted, will be asked.
    username: Option<String>,
}

pub fn main(args: User) -> Result<()> {
    match args.command {
        UserCommand::Add { username } => add(username),
        UserCommand::List => list(),
        UserCommand::Delete => delete()
    }
}

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

    let user = tusk_backend::resources::User::create(&mut db_connection, username, password)
        .unwrap_or_else(|_| panic!("TODO: handle error"));

    println!("Created user {}", user.username);

    Ok(())
}

pub fn list() -> Result<()> {
    let tusk = TuskConfigurationFile::import()?
        .into_tusk()?;

    let mut db_connection = tusk.database_connect()?;

    let users = tusk_backend::resources::User::read_all(&mut db_connection)
        .unwrap_or_else(|_| panic!("TODO: handle error"));

    println!("Username");
    println!("--------");
    for user in users {
        println!("{}", user.username);
    }

    Ok(())
}

pub fn delete() -> Result<()> {
    todo!()
}