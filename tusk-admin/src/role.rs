//! This module contains the necessary functions and data structures for the subcommand `user`.

use std::path::PathBuf;
use clap::{Parser, Subcommand};

use tusk_core::config::TuskConfigurationFile;
use tusk_core::error::Result;

/// User management.
///
/// This command allows to add, remove, list and, in general, administrate all the users.
#[derive(Parser, Debug)]
pub struct Role {
    #[command(subcommand)]
    command: RoleCommand
}

/// Enumerator containing the possible `user` commands.
#[derive(Subcommand, Debug)]
pub enum RoleCommand {
    /// Adds a new role in the database.
    Add {
        /// Name of the role.
        ///
        /// If omitted, will be asked.
        name: Option<String>,
        /// Display name of the role.
        ///
        /// If omitted, will be the same as `name` with capitalized initial.
        #[clap(short, long)]
        display: Option<String>
    },
    /// Assigns a role to an user.
    Assign {
        /// Name of the role.
        role: String,
        /// Username of the user.
        #[clap(long = "to")]
        username: String
    },
    /// Removes a role from an user.
    Cancel {
        /// Name of the role.
        role: String,
        /// Username of the user.
        #[clap(long = "from")]
        username: String
    },
    /// Lists all the roles.
    List,
    /// Removes an user from the database.
    Remove {
        /// Name of the role.
        ///
        /// If omitted, will be asked.
        name: Option<String>
    },
}

/// Main entry point for the `role` command.
pub fn main(args: Role) -> Result<()> {
    match args.command {
        RoleCommand::Add { name, display } => add(name, display),
        RoleCommand::Assign { role, username } => assign(role, username),
        RoleCommand::Cancel { role, username } => cancel(role, username),
        RoleCommand::List => list(),
        RoleCommand::Remove { name } => remove(name)
    }
}

/// Adds a new role in the database.
pub fn add(name: Option<String>, display: Option<String>) -> Result<()> {
    let name = if let Some(name) = name {
        name
    } else {
        dialoguer::Input::new()
            .with_prompt("Specify role name")
            .interact()?
    };
    let display = if let Some(display) = display {
        display
    } else {
        name.clone()
    };

    let tusk = TuskConfigurationFile::import()?
        .into_tusk()?;

    let mut db_connection = tusk.database_connect()?;

    let role = tusk_core::resources::Role::create(&mut db_connection, name, display)
        .unwrap_or_else(|_| panic!("TODO: handle error"));

    println!("Created role `{}` \"{}\"", role.name(), role.display());

    Ok(())
}
/// Assigns a role to an user.
pub fn assign(role: String, username: String) -> Result<()> {
    let tusk = TuskConfigurationFile::import()?
        .into_tusk()?;

    let mut db_connection = tusk.database_connect()?;

    tusk_core::resources::Role::assign(&mut db_connection, &role)
        .to(&username)?;

    let mut user_dir_path = PathBuf::from(tusk.user_directories());
    user_dir_path.push(&username);

    if &role == "directory" && !user_dir_path.exists() {
        println!("Warning: path `{}` does not exist.", user_dir_path.display());
    }

    println!("Role `{role}` assigned to user `{username}`.");

    Ok(())
}
/// Cancels a role assignation to an user.
pub fn cancel(role: String, username: String) -> Result<()> {
    let tusk = TuskConfigurationFile::import()?
        .into_tusk()?;

    let mut db_connection = tusk.database_connect()?;

    tusk_core::resources::Role::assign(&mut db_connection, &role)
        .cancel_from(&username)?;

    println!("Role `{role}` cancelled from user `{username}`.");

    Ok(())
}
/// Lists all the roles.
pub fn list() -> Result<()> {
    let tusk = TuskConfigurationFile::import()?
        .into_tusk()?;

    let mut db_connection = tusk.database_connect()?;

    let roles = tusk_core::resources::Role::read_all(&mut db_connection)
        .unwrap_or_else(|_| panic!("TODO: handle error"));

    let role_max_len = roles.iter()
        .map(|role| role.name().len())
        .max()
        .unwrap_or(4)
        .max(4);
    let display_max_len = roles.iter()
        .map(|role| role.display().len())
        .max()
        .unwrap_or(4)
        .max(4);

    println!("{:role_max_len$}  {:display_max_len$}", "Role", "Display");
    println!("{:-^role_max_len$}  {:-^display_max_len$}", "", "");
    for role in roles {
        println!("{:role_max_len$}  {:display_max_len$}", role.name(), role.display());
    }

    Ok(())
}
/// Removes an user from the database.
pub fn remove(name: Option<String>) -> Result<()> {
    let name = if let Some(name) = name {
        name
    } else {
        dialoguer::Input::new()
            .with_prompt("Specify role name")
            .interact()?
    };

    let tusk = TuskConfigurationFile::import()?
        .into_tusk()?;

    let mut db_connection = tusk.database_connect()?;

    let deleted = tusk_core::resources::Role::delete_by_name(&mut db_connection, name)?;

    println!("Deleted {deleted} role");

    Ok(())
}