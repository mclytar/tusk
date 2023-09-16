use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use serde::Deserialize;
use tera::Tera;
use crate::error::TuskResult;

/// Represents the `tusk.serve` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct Serve {
    root: String,
    tera_templates: Option<String>,
    static_files: Option<String>,
    user_directories: Option<String>
}
impl Serve {
    pub fn root(&self) -> PathBuf {
        PathBuf::from(&self.root)
    }

    pub fn tera_templates(&self) -> PathBuf {
        if let Some(path) = &self.tera_templates {
            PathBuf::from(path)
        } else {
            let mut path = self.root();
            path.push("tera");
            path
        }
    }

    pub fn tera(&self) -> TuskResult<Arc<RwLock<Tera>>> {
        log::debug!("Loading Tera section");

        let mut tera_path = self.tera_templates();
        tera_path.push("**");
        tera_path.push("*.tera");
        let mut tera = Tera::new(tera_path.to_string_lossy().as_ref())?;
        for template in tera.get_template_names() {
            log::info!("Loaded Tera template {template}");
        }
        tera.autoescape_on(vec![".html", ".tera"]);

        Ok(Arc::new(RwLock::new(tera)))
    }

    pub fn static_files(&self) -> PathBuf {
        if let Some(path) = &self.static_files {
            PathBuf::from(path)
        } else {
            let mut path = self.root();
            path.push("static");
            path
        }
    }

    pub fn user_directories(&self) -> PathBuf {
        if let Some(path) = &self.user_directories {
            PathBuf::from(path)
        } else {
            let mut path = self.root();
            path.push("storage");
            path
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::config::tusk::serve::Serve;

    const TEST_FILE: &'static str = r#"
    root = "/server"
    tera_templates = "/server/other_tera"
    static_files = "/server/other_static"
    user_directories = "/server/other_storage"
    "#;

    #[test]
    fn it_works() {
        let test_file: Serve = toml::from_str(TEST_FILE)
            .expect("Valid TOML");

        assert_eq!(test_file.root(), PathBuf::from("/server"));
        assert_eq!(test_file.tera_templates(), PathBuf::from("/server/other_tera"));
        assert_eq!(test_file.static_files(), PathBuf::from("/server/other_static"));
        assert_eq!(test_file.user_directories(), PathBuf::from("/server/other_storage"));
    }

    #[test]
    fn automatic_directories() {
        let test_file: Serve = toml::from_str(r#"root = "/main""#)
            .expect("Valid TOML");

        assert_eq!(test_file.root(), PathBuf::from("/main"));
        assert_eq!(test_file.tera_templates(), PathBuf::from("/main/tera"));
        assert_eq!(test_file.static_files(), PathBuf::from("/main/static"));
        assert_eq!(test_file.user_directories(), PathBuf::from("/main/storage"));
    }
}