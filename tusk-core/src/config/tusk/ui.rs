use serde::Deserialize;

/// Represents the `tusk.ui` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct Ui {
    pub icon_filetype: String
}