//! This module contains OS specific details and implementations.

/// Path where the Tusk configuration file is stored.
#[cfg(windows)]
pub const CONFIGURATION_FILE_PATH: &str = "C:\\ProgramData\\Tusk\\tusk.toml";
/// Path where the Tusk configuration file is stored.
#[cfg(unix)]
pub const CONFIGURATION_FILE_PATH: &str = "/etc/tusk/tusk.toml";