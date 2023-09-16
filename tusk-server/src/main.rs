#![warn(missing_docs)]

//! This is the main executable of Tusk Server.
//!
//! It runs a different setup depending on the operating system;
//! in both cases, after the setup, the executable runs as a service/daemon in background, logging
//! to the system logger.

use clap::Parser;
use log::LevelFilter;
use tusk_core::error::TuskResult;
use tusk_server::{os, run_server, spawn_server, spawn_tusk, spawn_watcher};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    cli: bool
}

fn main() -> TuskResult<()> {
    let args = Args::parse();

    if args.cli {
        env_logger::builder()
            .filter_level(LevelFilter::Info)
            .init();

        let tusk = spawn_tusk()?;
        let server = spawn_server(&tusk)?;
        let _w = spawn_watcher(&tusk);

        os::drop_privileges()?;

        run_server(server)?;
    } else {
        os::start_logger();

        os::run()?;
    }

    Ok(())
}