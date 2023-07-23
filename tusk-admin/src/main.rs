mod os;


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
    Reload
}

fn main() {
    let args = Args::parse();

    let result = match args.command {
        Command::Install => os::daemon_install(),
        Command::Uninstall => os::daemon_uninstall(),
        Command::Start => os::daemon_start(),
        Command::Stop => os::daemon_stop(),
        Command::Reload => os::daemon_reload(),
    };

    if let Err(e) = result {
        os::print_error(e);
    }
}
