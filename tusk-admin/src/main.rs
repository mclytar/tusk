//! This crate is a CLI executable to administrate the Tera server.

#![warn(missing_docs)]

pub mod os;
pub mod user;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    verbose: Verbosity<WarnLevel>
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Installs Tusk as a service/daemon.
    Install,
    /// Uninstalls Tusk as a service/daemon.
    Uninstall,
    /// Starts Tusk as a service/daemon.
    Start,
    /// Stops Tusk as a service/daemon.
    Stop,
    /// Reloads the configuration.
    Reload,
    /// Role management commands.
    //Role(role::Role),
    /// User management commands.
    User(user::User)
}

fn main() {
    let args = Args::parse();

    if !args.verbose.is_silent() {
        env_logger::builder()
            .filter_level(args.verbose.log_level_filter())
            .init();
    }

    let result = match args.command {
        Command::Install => os::service_install(),
        Command::Uninstall => os::service_uninstall(),
        Command::Start => os::service_start(),
        Command::Stop => os::service_stop(),
        Command::Reload => os::service_reload(),
        //Command::Role(role) => role::main(role),
        Command::User(args) => user::main(args)
    };

    if let Err(e) = result {
        os::print_error(e);
    }
}
