//! This module contains the necessary structures and functions to load the configuration from
//! `tusk.toml`.

use std::fs::File;
use std::io::{BufReader, ErrorKind};
use std::path::PathBuf;
use std::sync::{Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard};

use actix_web::web;
use diesel::{r2d2::{ConnectionManager, Pool, PooledConnection}, PgConnection};
use diesel_migrations::{embed_migrations, MigrationHarness};
use serde::Deserialize;
use tera::Tera;

use crate::error::{Error, Result};

/// `actix_web::web::Data` wrapper for [`TuskConfiguration`].
pub type TuskData = web::Data<TuskConfiguration>;

/// Returns a TLS server configuration.
pub fn spawn_tls_configuration() -> Result<rustls::ServerConfig> {
    #[cfg(test)]
        let file = File::open("cert.pem")?;
    #[cfg(all(windows, not(test)))]
        let file = File::open("C:\\ProgramData\\Tusk\\tusk.crt")?;
    #[cfg(all(unix, not(test)))]
        let file = File::open("/etc/tusk/domains/server-dev.local/cert.pem")?;
    let mut reader = BufReader::new(file);
    let certs: Vec<_> = rustls_pemfile::certs(&mut reader)?
        .into_iter()
        .map(rustls::Certificate)
        .collect();

    log::info!("Found {} certificates.", certs.len());

    #[cfg(test)]
        let file = File::open("key.pem")?;
    #[cfg(all(windows, not(test)))]
        let file = File::open("C:\\ProgramData\\Tusk\\tusk.key")?;
    #[cfg(all(unix, not(test)))]
        let file = File::open("/etc/tusk/domains/server-dev.local/key.pem")?;
    let mut reader = BufReader::new(file);
    let keys: Vec<_> = rustls_pemfile::pkcs8_private_keys(&mut reader)?
        .into_iter()
        .map(rustls::PrivateKey)
        .collect();

    log::info!("Found {} keys, using the first one available.", keys.len());

    let key = keys.into_iter()
        .next()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No key in file 'tusk.key'."))?;

    log::info!("Key file loaded");

    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(config)
}
/// Represents the `diesel` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct DieselConfigurationSection {
    url: String
}
/// Represents the `redis` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct RedisConfigurationSection {
    url: String
}
/// Represents the `tusk` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct TuskConfigurationSection {
    log_level: log::LevelFilter,
    www_domain: String,
    api_domain: String,
    tera_templates: String,
    static_files: String,
    user_directories: String,
    ui_icon_filetype: String
}
/// Represents the file `tusk.toml`.
#[derive(Clone, Debug, Deserialize)]
pub struct TuskConfigurationFile {
    diesel: DieselConfigurationSection,
    redis: RedisConfigurationSection,
    tusk: TuskConfigurationSection
}
impl TuskConfigurationFile {
    /// Imports `tusk.toml` from a known location.
    ///
    /// ## On Windows
    /// Tries to import `tusk.toml` from the same folder as the executable;
    /// if this fails, falls back to importing `C:\ProgramData\Tusk\tusk.toml`.
    ///
    /// ## On Unix
    /// Tries to import `tusk.toml` from the same folder as the executable;
    /// if this fails, falls back to importing `/etc/tusk/tusk.toml`.
    pub fn import() -> Result<TuskConfigurationFile> {
        log::info!("Executing from {}.", PathBuf::from(".").canonicalize()?.display());

        #[cfg(not(test))]
        let first_path = "./tusk.toml";
        #[cfg(test)]
        let first_path = "../tusk.toml";

        let data = match std::fs::read_to_string(first_path) {
            Ok(data) => {
                log::info!("Loaded configuration from tusk.toml.");
                data
            },
            Err(e) if e.kind() == ErrorKind::NotFound => {
                let data = std::fs::read_to_string(crate::os::CONFIGURATION_FILE_PATH)?;
                log::info!("Loaded configuration from {}.", crate::os::CONFIGURATION_FILE_PATH);
                data
            },
            Err(e) => Err(e)?
        };
        let file = toml::from_str(&data)?;

        Ok(file)
    }

    /// Finalizes the configuration file and constructs a [`TuskConfiguration`] structure.
    pub fn into_tusk(self) -> Result<TuskConfiguration> {
        #[allow(unused)]
        let TuskConfigurationSection { log_level, www_domain, api_domain, tera_templates, static_files, user_directories, ui_icon_filetype } = self.tusk;

        #[cfg(not(test))]
        log::set_max_level(log_level);

        let mut tera_path = PathBuf::from(&tera_templates);
        tera_path.push("**");
        tera_path.push("*.tera");
        let tera = Tera::new(tera_path.to_string_lossy().as_ref())?;
        for template in tera.get_template_names() {
            log::info!("Loaded Tera template {template}");
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

        let connection_manager = ConnectionManager::new(self.diesel.url);
        let database_pool = Pool::new(connection_manager)?;
        let database_pool = Arc::new(database_pool);

        let tls_server_configuration = spawn_tls_configuration()?;

        let config = TuskConfiguration {
            tera,
            www_domain,
            api_domain,
            tera_templates,
            static_files,
            user_directories,
            database_pool,
            session_configuration,
            tls_server_configuration,
            ui_icon_filetype
        };

        Ok(config)
    }
}

/// Represents a configuration for the Redis session storage.
#[derive(Clone)]
pub struct SessionConfiguration {
    redis_uri: String,
    session_key: actix_web::cookie::Key,
    session_lifecycle: actix_session::config::PersistentSession
}
/// Represents a configuration for the Tusk server.
#[derive(Clone)]
pub struct TuskConfiguration {
    tera: Arc<RwLock<Tera>>,
    www_domain: String,
    api_domain: String,
    tera_templates: String,
    static_files: String,
    user_directories: String,
    database_pool: Arc<Pool<ConnectionManager<PgConnection>>>,
    session_configuration: SessionConfiguration,
    tls_server_configuration: rustls::ServerConfig,
    ui_icon_filetype: String
}
impl TuskConfiguration {
    /// Returns a configuration wrapped in `actix_web::web::Data` to store into the web server.
    pub fn to_data(&self) -> web::Data<TuskConfiguration> {
        web::Data::new(self.clone())
    }
    /// Returns the domain from which the HTML pages and the static files are served.
    pub fn www_domain(&self) -> &str {
        &self.www_domain
    }
    /// Returns the domain from which the REST API is served.
    pub fn api_domain(&self) -> &str {
        &self.api_domain
    }
    /// Returns the path from which the Tera templates are loaded.
    pub fn tera_templates(&self) -> &str {
        &self.tera_templates
    }
    /// Returns the path from which to serve static files.
    pub fn static_files(&self) -> &str {
        &self.static_files
    }
    /// Returns the path where user files are stored.
    pub fn user_directories(&self) -> &str {
        &self.user_directories
    }
    /// Returns the file extension for the UI icons.
    pub fn ui_icon_filetype(&self) -> &str {
        &self.ui_icon_filetype
    }
    /// Returns a connection to the database.
    pub fn database_connect(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>> {
        let db_pool = self.database_pool.get()?;
        Ok(db_pool)
    }
    /// Applies all the pending migrations.
    pub fn apply_migrations(&self) -> Result<()> {
        const MIGRATIONS: diesel_migrations::EmbeddedMigrations = embed_migrations!("../migrations");

        let mut db_connection = self.database_connect()?;

        let pending_migrations_count = db_connection.pending_migrations(MIGRATIONS)
            .map_err(Error::from_migration_error)?
            .len();

        if pending_migrations_count > 0 {
            log::info!("Found {pending_migrations_count} migration(s)");

            db_connection.run_pending_migrations(MIGRATIONS)
                .map_err(Error::from_migration_error)?;

            log::info!("Applied {pending_migrations_count} migration(s)");
        } else {
            log::info!("No pending migrations found")
        }

        Ok(())
    }
    /// Returns a reference to the current Tera state.
    pub fn tera(&self) -> LockResult<RwLockReadGuard<Tera>> {
        self.tera.read()
    }
    /// Returns a mutable reference to the current Tera state.
    pub fn tera_mut(&self) -> LockResult<RwLockWriteGuard<Tera>> {
        self.tera.write()
    }
    /// Builds a new Tera context embedding the global variables given by the configuration.
    pub fn tera_context(&self) -> tera::Context {
        let mut context = tera::Context::new();

        context.insert("protocol", "https");
        context.insert("www_domain", &self.www_domain);
        context.insert("api_domain", &self.api_domain);
        context.insert("ui_icon_filetype", &self.ui_icon_filetype);

        context
    }
    /// Builds and returns a Redis connection to store the session cookies.
    pub async fn redis_store(&self) -> actix_session::storage::RedisSessionStore {
        actix_session::storage::RedisSessionStore::new(&self.session_configuration.redis_uri)
            .await
            .expect("Redis connection")
    }
    /// Returns the current session key.
    pub fn session_key(&self) -> actix_web::cookie::Key {
        self.session_configuration.session_key.clone()
    }
    /// Returns the current session life cycle.
    pub fn session_lifecycle(&self) -> actix_session::config::PersistentSession {
        self.session_configuration.session_lifecycle.clone()
    }
    /// Returns the current TLS configuration.
    pub fn tls_config(&self) -> rustls::ServerConfig {
        self.tls_server_configuration.clone()
    }
    /// Checks whether all the users with role `directory` actually have a directory, and logs
    /// a warning in case not.
    pub fn check_user_directories(&self) -> Result<usize> {
        let mut db_connection = self.database_connect()?;
        let mut count = 0;

        let directory_users = crate::resources::User::read_by_role_name(&mut db_connection, "directory")?;
        let directory_path = PathBuf::from(&self.user_directories);

        log::info!("Checking directories in `{}`", directory_path.display());
        for dir_user in directory_users {
            let mut user_dir_path = directory_path.clone();
            user_dir_path.push(dir_user.username());
            if !user_dir_path.exists() {
                count += 1;
                log::warn!("Missing directory for user `{}`", dir_user.username());
            }
        }

        Ok(count)
    }
}

#[cfg(any(feature = "test_utils", test))]
pub static TEST_CONFIGURATION: once_cell::sync::Lazy<TuskConfiguration> = once_cell::sync::Lazy::new(|| {
    use diesel::{Connection};
    use log::LevelFilter;
    use secrecy::Secret;

    use crate::resources::{Role, User};

    let _ = env_logger::builder()
        .filter_level(LevelFilter::Info)
        .is_test(true)
        .try_init();

    let mut configuration_file = TuskConfigurationFile::import()
        .expect("configuration_file");

    configuration_file.diesel.url += "_test";

    log::info!("Emptying test database");

    const MIGRATIONS: diesel_migrations::EmbeddedMigrations = embed_migrations!("../migrations");

    let mut db_connection = PgConnection::establish(configuration_file.diesel.url.as_ref())
        .expect("database connection");

    db_connection.revert_all_migrations(MIGRATIONS).expect("all migrations reverted");

    let config = configuration_file.clone()
        .into_tusk()
        .expect("configuration");

    config.apply_migrations().expect("database migration");

    let mut db_connection = config.database_connect()
        .expect("database connection");

    User::create(&mut db_connection, "test", Secret::new(String::from("test"))).expect("database connection");
    User::create(&mut db_connection, "dummy", Secret::new(String::from("dummy"))).expect("database connection");
    User::create(&mut db_connection, "admin", Secret::new(String::from("admin"))).expect("database connection");
    User::create(&mut db_connection, "user", Secret::new(String::from("user"))).expect("database connection");

    Role::assign(&mut db_connection, "admin").to("admin").expect("role assignment");

    let mut directory_role_assign = Role::assign(&mut db_connection, "directory");
    directory_role_assign.to("admin").expect("role assignment");
    directory_role_assign.to("test").expect("role assignment");
    directory_role_assign.to("user").expect("role assignment");

    let mut user_role_assign = Role::assign(&mut db_connection, "user");
    user_role_assign.to("admin").expect("role assignment");
    user_role_assign.to("test").expect("role assignment");
    user_role_assign.to("user").expect("role assignment");
    user_role_assign.to("dummy").expect("role assignment");

    config
});

#[cfg(test)]
pub mod test {
    use std::path::PathBuf;
    use super::TEST_CONFIGURATION;

    #[test]
    fn configuration_loaded_correctly() {
        let _config = &*super::TEST_CONFIGURATION;
    }

    #[test]
    fn tera_context_creation() {
        let context = TEST_CONFIGURATION.tera_context();

        assert_eq!(context.get("protocol").unwrap(), "https");
        assert_eq!(context.get("www_domain").unwrap(), &TEST_CONFIGURATION.www_domain);
        assert_eq!(context.get("api_domain").unwrap(), &TEST_CONFIGURATION.api_domain);
        assert_eq!(context.get("ui_icon_filetype").unwrap(), &TEST_CONFIGURATION.ui_icon_filetype);
    }

    #[test]
    fn directory_users_missing_directory() {
        let users_without_directory = TEST_CONFIGURATION.check_user_directories()
            .expect("user directory testing");

        let path = PathBuf::from(TEST_CONFIGURATION.user_directories())
            .canonicalize()
            .expect("canonicalized path");
        log::info!("Loading directories from `{}`", path.display());

        assert_eq!(users_without_directory, 1);
    }
}