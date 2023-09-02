//! Defines OS-specific behavior.

#[cfg(windows)]
pub mod windows;
#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub use windows::*;
#[cfg(unix)]
pub use unix::*;

use std::path::PathBuf;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tusk_core::config::TuskData;

/// Spawns a watcher that watches the Tera templates directory for changes, and reloads Tera if
/// something changed.
pub fn spawn_watcher(tusk: TuskData) -> RecommendedWatcher {
    let watch_dir = PathBuf::from(tusk.tera_templates());
    log::info!("Starting watcher for directory `{}`", watch_dir.display());

    let mut watcher = notify::recommended_watcher(move |res| {
        match res {
            Ok(_) => {
                log::info!("Reloading Tera templates after changes...");
                let mut tera = match tusk.tera_mut() {
                    Ok(lock) => lock,
                    Err(e) => { log::error!("{e}"); return; }
                };
                match tera.full_reload() {
                    Ok(()) => {},
                    Err(e) => { log::error!("{e}"); }
                }
            },
            Err(e) => {
                log::error!("{e}");
            }
        }
    }).expect("event watcher");

    watcher.watch(&watch_dir, RecursiveMode::Recursive)
        .expect("watcher set up");

    watcher
}