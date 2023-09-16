use serde::Deserialize;

/// Represents the `tusk.contacts` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct Contacts {
    pub noreply: String,
    pub support: String
}