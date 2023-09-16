//! Defines OS-specific behavior.

#[cfg(windows)]
pub mod windows;
#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub use windows::*;
#[cfg(unix)]
pub use unix::*;