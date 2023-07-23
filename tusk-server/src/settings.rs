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
            Err(e) => {
                panic!("Error: {e}\nI don't know how to handle this error yet.")
            }
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

        let redis_uri = self.redis.url;
        let session_key = actix_web::cookie::Key::generate();
        let session_lifecycle = actix_session::config::PersistentSession::default()
            .session_ttl(actix_web::cookie::time::Duration::minutes(15))
            .session_ttl_extension_policy(actix_session::config::TtlExtensionPolicy::OnEveryRequest);

        let session_configuration = SessionConfiguration {
            redis_uri,
            session_key,
            session_lifecycle
        };

        let TuskConfigurationSection { log_level, www_domain, api_domain } = self.tusk;
        log::set_max_level(log_level);

        TuskConfiguration {
            tera,
            www_domain,
            api_domain,
            session_configuration
        }
    }
}

#[derive(Clone)]
pub struct SessionConfiguration {
    redis_uri: String,
    session_key: actix_web::cookie::Key,
    session_lifecycle: actix_session::config::PersistentSession
}

#[derive(Clone)]
pub struct TuskConfiguration {
    pub tera: Arc<RwLock<Tera>>,
    www_domain: String,
    api_domain: String,
    session_configuration: SessionConfiguration
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

    pub async fn redis_store(&self) -> actix_session::storage::RedisSessionStore {
        actix_session::storage::RedisSessionStore::new(&self.session_configuration.redis_uri)
            .await
            .expect("Redis connection")
    }

    pub fn session_key(&self) -> actix_web::cookie::Key {
        self.session_configuration.session_key.clone()
    }

    pub fn session_lifecycle(&self) -> actix_session::config::PersistentSession {
        self.session_configuration.session_lifecycle.clone()
    }
}