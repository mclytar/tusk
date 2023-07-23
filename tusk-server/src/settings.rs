use std::sync::{Arc, RwLock};

#[allow(unused)] use log::{error, warn, info, debug, trace};

use actix_web::web;
use serde::Deserialize;
use tera::Tera;

#[derive(Clone, Debug, Deserialize)]
pub struct RedisConfigurationSection {
    pub url: String
}

#[derive(Clone, Debug, Deserialize)]
pub struct TuskConfigurationSection {
    pub log_level: log::LevelFilter,
    pub www_domain: String,
    pub api_domain: String
}

#[derive(Clone, Debug, Deserialize)]
pub struct TuskConfigurationFile {
    pub redis: RedisConfigurationSection,
    pub tusk: TuskConfigurationSection
}
impl TuskConfigurationFile {
    pub fn import() -> std::io::Result<TuskConfigurationFile> {
        let data = std::fs::read_to_string(crate::os::CONFIGURATION_FILE_PATH)?;
        let file = match toml::from_str(&data) {
            Ok(f) => f,
            Err(_) => panic!("I don't know how to handle this error yet.")
        };

        Ok(file)
    }

    pub fn into_tusk(self) -> TuskConfiguration {
        let tera = match Tera::new("/srv/http/**/*.tera") {
            Ok(t) => t,
            Err(e) => {
                error!("Cannot load Tera templates: {}", e);
                ::std::process::exit(1);
            }
        };

        for template in tera.get_template_names() {
            info!("Loaded Tera template {template}");
        }

        let tera = Arc::new(RwLock::new(tera));

        let TuskConfigurationSection { log_level, www_domain, api_domain } = self.tusk;
        log::set_max_level(log_level);

        TuskConfiguration {
            tera,
            www_domain,
            api_domain
        }
    }
}

#[derive(Clone, Debug)]
pub struct TuskConfiguration {
    pub tera: Arc<RwLock<Tera>>,
    www_domain: String,
    api_domain: String
}
impl TuskConfiguration {
    pub fn to_data(&self) -> web::Data<TuskConfiguration> {
        web::Data::new(self.clone())
    }

    pub fn www_domain(&self) -> &str {
        &self.www_domain
    }

    pub fn api_domain(&self) -> &str {
        &self.api_domain
    }

    pub fn tera_context(&self) -> tera::Context {
        let mut context = tera::Context::new();

        context.insert("protocol", "http");
        context.insert("www_domain", &self.www_domain);
        context.insert("api_domain", &self.api_domain);

        context
    }
}