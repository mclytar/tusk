use systemd::daemon;
use systemd_journal_logger::JournalLog;

pub fn run() -> std::io::Result<()> {
    let server = crate::server_spawn()?;

    daemon::notify(false, [(daemon::STATE_READY, 1)])?;

    crate::server_run(server)?;
    Ok(())
}

pub fn initialize_logger() {
    JournalLog::default().install()
        .expect("a functioning logger");
}