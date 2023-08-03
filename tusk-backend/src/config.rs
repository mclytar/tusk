use std::sync::{Arc, RwLock};

#[allow(unused)] use log::{error, warn, info, debug, trace};

use actix_web::web;
use diesel::{r2d2::{ConnectionManager, Pool, PooledConnection}, PgConnection};
use serde::Deserialize;
use tera::Tera;

use crate::error::Result;

pub type TuskData = web::Data<TuskConfiguration>;

#[derive(Clone, Debug, Deserialize)]
pub struct DieselConfigurationSection {
    pub url: String
}

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
    pub diesel: DieselConfigurationSection,
    pub redis: RedisConfigurationSection,
    pub tusk: TuskConfigurationSection
}
impl TuskConfigurationFile {
    pub fn import() -> Result<TuskConfigurationFile> {
        let data = std::fs::read_to_string(crate::os::CONFIGURATION_FILE_PATH)?;
        let file = toml::from_str(&data)?;

        Ok(file)
    }

    pub fn into_tusk(self) -> Result<TuskConfiguration> {
        let tera = Tera::new("/srv/http/**/*.tera")?;
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

        let connection_manager = ConnectionManager::new(self.diesel.url);
        let database_pool = Pool::new(connection_manager)?;
        let database_pool = Arc::new(database_pool);

        let config = TuskConfiguration {
            tera,
            www_domain,
            api_domain,
            database_pool,
            session_configuration
        };

        Ok(config)
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
    database_pool: Arc<Pool<ConnectionManager<PgConnection>>>,
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

    pub fn database_connect(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>> {
        let db_pool = self.database_pool.get()?;
        Ok(db_pool)
    }

    pub fn tera_context(&self) -> tera::Context {
        let mut context = tera::Context::new();

        context.insert("protocol", "https");
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