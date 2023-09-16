//! Defines the necessary functions to make the server run as a Unix daemon.

use log::LevelFilter;
use systemd::daemon;
use systemd_journal_logger::JournalLog;

use tusk_core::error::TuskResult;

/// Starts the system logger.
pub fn start_logger() {
    JournalLog::default().install().unwrap();
}

/// Drops the privileges of the process.
pub fn drop_privileges() -> TuskResult<()> {
    match nix::unistd::Group::from_name("tusk")? {
        Some(group) => nix::unistd::setgid(group.gid),
        None => Err(nix::Error::last())
    }?;
    match nix::unistd::User::from_name("tusk")? {
        Some(user) => nix::unistd::setuid(user.uid),
        None => Err(nix::Error::last())
    }?;
    Ok(())
}

/// Runs the server.
pub fn run() -> TuskResult<()> {
    let tusk = crate::spawn_tusk()?;
    let server = crate::spawn_server(&tusk)?;

    daemon::notify(false, [(daemon::STATE_READY, "1")].iter())?;

    drop_privileges()?;

    let _w = crate::spawn_watcher(tusk);

    crate::run_server(server)?;

    Ok(())
}