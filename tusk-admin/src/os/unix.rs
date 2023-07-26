const SERVICE_FILE_CONTENTS: &'static str = include_str!("tusk.service");
const SYSTEMD_UNIT_PATH: &'static str = "/etc/systemd/system/tusk.service";

use tusk_backend::error::Result;

pub fn service_install() -> Result<()> {
    println!("Creating unit file...");

    std::fs::write(SYSTEMD_UNIT_PATH, SERVICE_FILE_CONTENTS)?;

    println!("Enabling unit file...");

    systemctl::enable("tusk.service")?;

    println!("Done!");

    Ok(())
}

pub fn service_uninstall() -> Result<()> {
    println!("Disabling unit file...");

    systemctl::disable("tusk.service")?;

    println!("Removing unit file...");

    std::fs::remove_file(SYSTEMD_UNIT_PATH)?;

    println!("Done!");

    Ok(())
}

pub fn service_start() -> Result<()> {
    println!("Starting service...");

    systemctl::start("tusk.service")?;

    println!("Done!");

    Ok(())
}

pub fn service_stop() -> Result<()> {
    println!("Stopping service...");

    systemctl::stop("tusk.service")?;

    println!("Done!");

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