pub mod error;
mod os;
mod user;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command
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
    /// User management commands.
    User(user::User)
}

fn main() {
    let args = Args::parse();

    let result = match args.command {
        Command::Install => os::service_install(),
        Command::Uninstall => os::service_uninstall(),
        Command::Start => os::service_start(),
        Command::Stop => os::service_stop(),
        Command::Reload => os::service_reload(),
        Command::User(args) => user::main(args)
    };

    if let Err(e) = result {
        os::print_error(e);
    }
}
