const SERVICE_FILE_CONTENTS: &'static str = include_str!("tusk.service");
const SYSTEMD_UNIT_PATH: &'static str = "/etc/systemd/system/tusk.service";

use std::time::{Duration, Instant};
use indicatif::{ProgressBar, ProgressStyle};
use tusk_backend::error::Result;

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

pub fn service_start() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap().tick_chars("|/-\\ "));
    pb.enable_steady_tick(Duration::from_millis(50));
    pb.set_message("Starting service...");

    systemctl::start("tusk.service")?;

    pb.finish_with_message("Done!");

    Ok(())
}

pub fn service_stop() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap().tick_chars("|/-\\ "));
    pb.enable_steady_tick(Duration::from_millis(50));
    pb.set_message("Starting service...");

    systemctl::stop("tusk.service")?;

    pb.finish_with_message("Done!");

    Ok(())
}

pub fn service_reload() -> Result<()> {
    service_stop()?;
    service_start()?;
    Ok(())
}

pub fn print_error(e: tusk_backend::error::Error) {
    println!("Cannot perform operation: {e}")
}