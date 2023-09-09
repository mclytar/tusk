//! Defines the necessary functions to make the server run as a Unix daemon.

use systemd::daemon;
use systemd_journal_logger::JournalLog;

use tusk_core::error::TuskResult;

/// Runs the server.
pub fn run() -> TuskResult<()> {
    JournalLog::default().install()
        .expect("a functioning logger");

    let (server, tusk) = crate::server_spawn()?;

    daemon::notify(false, [(daemon::STATE_READY, "1")].iter())?;

    // Drop privileges!
    match nix::unistd::Group::from_name("tusk")? {
        Some(group) => nix::unistd::setgid(group.gid),
        None => Err(nix::Error::last())
    }?;
    match nix::unistd::User::from_name("tusk")? {
        Some(user) => nix::unistd::setuid(user.uid),
        None => Err(nix::Error::last())
    }?;

    let _w = super::spawn_watcher(tusk);

    crate::server_run(server)?;
    Ok(())
}