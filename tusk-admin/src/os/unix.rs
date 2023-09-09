//! This module contains the necessary functions to manage the server as a Unix daemon.

const SERVICE_FILE_CONTENTS: &'static str = include_str!("tusk.service");
const SYSTEMD_UNIT_PATH: &'static str = "/etc/systemd/system/tusk.service";

use std::time::{Duration};
use indicatif::{ProgressBar, ProgressStyle};
use tusk_core::error::TuskResult;

/// Installs the server as a Unix daemon.
pub fn service_install() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap().tick_chars("|/-\\ "));
    pb.enable_steady_tick(Duration::from_millis(50));
    pb.set_message("Creating unit file...");

    std::fs::write(SYSTEMD_UNIT_PATH, SERVICE_FILE_CONTENTS)?;

    pb.set_message("Enabling unit file...");

    systemctl::enable("tusk.service")?;

    pb.finish_with_message("Done!");

    Ok(())
}
/// Uninstalls the server as a Unix daemon.
pub fn service_uninstall() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap().tick_chars("|/-\\ "));
    pb.enable_steady_tick(Duration::from_millis(50));
    pb.set_message("Disabling unit file...");

    systemctl::disable("tusk.service")?;

    pb.set_message("Removing unit file...");

    std::fs::remove_file(SYSTEMD_UNIT_PATH)?;

    pb.finish_with_message("Done!");

    Ok(())
}
/// Starts the server as a Unix daemon.
pub fn service_start() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap().tick_chars("|/-\\ "));
    pb.enable_steady_tick(Duration::from_millis(50));
    pb.set_message("Starting service...");

    systemctl::start("tusk.service")?;

    pb.finish_with_message("Done!");

    Ok(())
}
/// Stops the server as a Unix daemon.
pub fn service_stop() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap().tick_chars("|/-\\ "));
    pb.enable_steady_tick(Duration::from_millis(50));
    pb.set_message("Starting service...");

    systemctl::stop("tusk.service")?;

    pb.finish_with_message("Done!");

    Ok(())
}
/// Reloads the server and its configuration.
pub fn service_reload() -> Result<()> {
    service_stop()?;
    service_start()?;
    Ok(())
}
/// Prints an error.
pub fn print_error(e: tusk_core::error::TuskError) {
    println!("Cannot perform operation: {e}")
}