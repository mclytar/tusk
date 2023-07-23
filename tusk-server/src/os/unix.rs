use systemd_journal_logger::JournalLog;

pub fn run() -> std::io::Result<()> {
    let server = crate::server_spawn()?;
    crate::server_run(server)?;
    Ok(())
}

pub fn initialize_logger() {
    JournalLog::default().install()
        .expect("a functioning logger");
}