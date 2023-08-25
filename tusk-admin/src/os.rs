//! This module loads the necessary functions to manage the server as either a Unix daemon
//! or a Windows service, depending on the operating system.

#[cfg(windows)]
pub mod windows;
#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub use windows::*;
#[cfg(unix)]
pub use unix::*;