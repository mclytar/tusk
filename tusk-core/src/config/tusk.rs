use serde::Deserialize;

pub mod contacts;
pub mod serve;
pub mod ui;

/// Represents the `tusk` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct Tusk {
    pub log_level: log::LevelFilter,
    pub www_domain: String,
    pub api_domain: String,
    pub contacts: contacts::Contacts,
    pub serve: serve::Serve,
    pub ui: ui::Ui
}