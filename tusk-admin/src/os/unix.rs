const SERVICE_FILE_CONTENTS: &'static str = include_str!("tusk.service");
const SYSTEMD_UNIT_PATH: &'static str = "/etc/systemd/system/tusk.service";

pub fn service_install() -> std::io::Result<()> {
    println!("Creating unit file...");

    std::fs::write(SYSTEMD_UNIT_PATH, SERVICE_FILE_CONTENTS)?;

    println!("Enabling unit file...");

    systemctl::enable("tusk.service")?;

    println!("Done!");

    Ok(())
}

pub fn service_uninstall() -> std::io::Result<()> {
    println!("Disabling unit file...");

    systemctl::disable("tusk.service")?;

    println!("Removing unit file...");

    std::fs::remove_file(SYSTEMD_UNIT_PATH)?;

    println!("Done!");

    Ok(())
}

pub fn service_start() -> std::io::Result<()> {
    println!("Starting service...");

    systemctl::start("tusk.service")?;

    println!("Done!");

    Ok(())
}

pub fn service_stop() -> std::io::Result<()> {
    println!("Stopping service...");

    systemctl::stop("tusk.service")?;

    println!("Done!");

    Ok(())
}

pub fn service_reload() -> std::io::Result<()> {
    service_stop()?;
    service_start()?;
    Ok(())
}

pub fn print_error(e: std::io::Error) {
    println!("Cannot perform operation: {e}");
}