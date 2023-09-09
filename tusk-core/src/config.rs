//! This module contains the necessary structures and functions to load the configuration from
//! `tusk.toml`.

use std::fs::File;
use std::io::{BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard};

use actix_web::web;
use diesel::{r2d2::{ConnectionManager, Pool, PooledConnection}, PgConnection};
use diesel_migrations::{embed_migrations, MigrationHarness};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use tera::Tera;

use crate::error::{TuskError, TuskResult};

/// `actix_web::web::Data` wrapper for [`TuskConfiguration`].
pub type TuskData = web::Data<TuskConfiguration>;

/// Represents the `diesel` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct DieselConfigurationSection {
    url: Secret<String>
}
/// Represents the `redis` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct RedisConfigurationSection {
    url: String
}
/// Represents the `ssl` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct SslConfigurationSection {
    cert_file: String,
    key_file: String
}
impl SslConfigurationSection {
    /// Converts this section into a TLS server configuration.
    pub fn into_server_configuration(self) -> TuskResult<rustls::ServerConfig> {
        let file = File::open(&self.cert_file)?;
        let mut reader = BufReader::new(file);
        let certs: Vec<_> = rustls_pemfile::certs(&mut reader)?
            .into_iter()
            .map(rustls::Certificate)
            .collect();

        if certs.len() == 0 {
            log::error!("No certificate found.");
            return Err(TuskError::CertificatesNotFound);
        }
        log::info!("Found {} certificates.", certs.len());

        let file = File::open(&self.key_file)?;
        let mut reader = BufReader::new(file);
        let keys: Vec<_> = rustls_pemfile::pkcs8_private_keys(&mut reader)?
            .into_iter()
            .map(rustls::PrivateKey)
            .collect();

        if keys.len() > 0 { log::info!("Found {} keys, using the first one available.", keys.len()) };

        let key = keys.into_iter()
            .next()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, format!("No key in file '{}'.", self.key_file)))?;

        log::info!("Key file loaded");

        let config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        Ok(config)
    }
}
/// Represents the `tusk.serve` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct TuskServeConfigurationSection {
    root: String,
    tera_templates: Option<String>,
    static_files: Option<String>,
    user_directories: Option<String>
}
/// Represents the `tusk.ui` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct TuskUiConfigurationSection {
    icon_filetype: String
}
/// Represents the `tusk` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct TuskConfigurationSection {
    log_level: log::LevelFilter,
    www_domain: String,
    api_domain: String,
    serve: TuskServeConfigurationSection,
    ui: TuskUiConfigurationSection
}
/// Represents the file `tusk.toml`.
#[derive(Clone, Debug, Deserialize)]
pub struct TuskConfigurationFile {
    diesel: DieselConfigurationSection,
    redis: RedisConfigurationSection,
    ssl: SslConfigurationSection,
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
    pub fn import_from_default_locations() -> TuskResult<TuskConfigurationFile> {
        #[cfg(windows)]
        let config_file = Self::import_from_locations([".\\tusk.toml", "C:\\ProgramData\\Tusk\\tusk.toml"])?;
        #[cfg(unix)]
        let config_file = Self::import_from_locations(["./tusk.toml", "/etc/tusk/tusk.toml"])?;

        Ok(config_file)
    }

    /// Imports `tusk.toml` from a known location.
    ///
    /// ## On Windows
    /// Tries to import `tusk.toml` from the same folder as the executable;
    /// if this fails, falls back to importing `C:\ProgramData\Tusk\tusk.toml`.
    ///
    /// ## On Unix
    /// Tries to import `tusk.toml` from the same folder as the executable;
    /// if this fails, falls back to importing `/etc/tusk/tusk.toml`.
    pub fn import_from_locations<S: AsRef<str>, L: IntoIterator<Item=S>>(locations: L) -> TuskResult<TuskConfigurationFile> {
        let mut locations = locations.into_iter();

        while let Some(location) = locations.next() {
            log::info!("Trying `{}`...", location.as_ref());
            let data = match std::fs::read_to_string(location.as_ref()) {
                Ok(data) => {
                    log::info!("Loaded configuration from `{}`.", location.as_ref());
                    data
                },
                Err(e) if e.kind() == ErrorKind::NotFound => continue,
                Err(e) => {
                    log::info!("Stopping research due to unexpected error.");
                    Err(e)?
                }
            };
            let file = toml::from_str(&data)?;
            return Ok(file);
        }

        Err(TuskError::ConfigurationNotFound)
    }

    /// Finalizes the configuration file and constructs a [`TuskConfiguration`] structure.
    pub fn into_tusk(self) -> TuskResult<TuskConfiguration> {
        let extract_from_path = |root: &PathBuf, custom: Option<String>, default: &'static str| match custom {
            Some(path) => PathBuf::from(&path),
            None => {
                let mut path = root.clone();
                path.push(default);
                path
            }
        };

        #[allow(unused)]
        let TuskConfigurationSection {
            log_level,
            www_domain,
            api_domain,
            serve: TuskServeConfigurationSection {
                root,
                tera_templates,
                static_files,
                user_directories
            },
            ui: TuskUiConfigurationSection {
                icon_filetype: ui_icon_filetype
            }
        } = self.tusk;

        let root = PathBuf::from(root);
        let tera_templates = extract_from_path(&root, tera_templates, "tera");
        let static_files = extract_from_path(&root, static_files, "static");
        let user_directories = extract_from_path(&root, user_directories, "storage");

        #[cfg(not(test))]
        log::set_max_level(log_level);

        let mut tera_path = tera_templates.clone();
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

        let connection_manager = ConnectionManager::new(self.diesel.url.expose_secret());
        #[cfg(test)]
        let database_pool = Pool::builder()
            .connection_timeout(std::time::Duration::from_millis(2_000))
            .build(connection_manager)?;
        #[cfg(not(test))]
        let database_pool = Pool::new(connection_manager)?;
        let database_pool = Arc::new(database_pool);

        let tls_server_configuration = self.ssl.into_server_configuration()?;

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
    tera_templates: PathBuf,
    static_files: PathBuf,
    user_directories: PathBuf,
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
    pub fn tera_templates(&self) -> &Path {
        &self.tera_templates
    }
    /// Returns the path from which to serve static files.
    pub fn static_files(&self) -> &Path {
        &self.static_files
    }
    /// Returns the path where user files are stored.
    pub fn user_directories(&self) -> &Path {
        &self.user_directories
    }
    /// Returns the file extension for the UI icons.
    pub fn ui_icon_filetype(&self) -> &str {
        &self.ui_icon_filetype
    }
    /// Returns a connection to the database.
    pub fn database_connect(&self) -> TuskResult<PooledConnection<ConnectionManager<PgConnection>>> {
        let db_pool = self.database_pool.get()?;
        Ok(db_pool)
    }
    /// Applies all the pending migrations.
    pub fn apply_migrations(&self) -> TuskResult<()> {
        const MIGRATIONS: diesel_migrations::EmbeddedMigrations = embed_migrations!("../migrations");

        let mut db_connection = self.database_connect()?;

        let pending_migrations_count = db_connection.pending_migrations(MIGRATIONS)
            .map_err(TuskError::from_migration_error)?
            .len();

        if pending_migrations_count > 0 {
            log::info!("Found {pending_migrations_count} migration(s)");

            db_connection.run_pending_migrations(MIGRATIONS)
                .map_err(TuskError::from_migration_error)?;

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
    /// Checks whether all the users with role `storage` actually have a storage, and logs
    /// a warning in case not.
    pub fn check_user_directories(&self) -> TuskResult<usize> {
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
                log::warn!("Missing storage for user `{}`", dir_user.username());
            }
        }

        Ok(count)
    }
}

/// Contains a test configuration for unit testing this and all the other related crates.
///
/// # Configuration
/// It requires a valid `tusk.toml` placed in the workspace storage, and a connection to a
/// Postgres database `tusk_test` (or, whatever name the original database has + `_test`).
/// Also, the unit tests for `tusk-core` assume the presence of the following files and directories
/// (and nothing else inside `srv/storage`):
/// - `srv/storage/admin`
/// - `srv/storage/user`
/// - `cert.pem` (that is, a valid certificate file)
/// - `key.pem` (that is, a valid key file for the above certificate)
///
/// # Initialization
/// The test database is initialized as follows.
/// Four users are created:
/// - `admin`, with roles `admin`, `storage`, `user`
/// - `dummy`, with role `user`
/// - `test`, with roles `storage`, `user`
/// - `user`, with roles `user`
#[cfg(any(feature = "test_utils", test))]
pub static TEST_CONFIGURATION: once_cell::sync::Lazy<TuskConfiguration> = once_cell::sync::Lazy::new(|| {
    use diesel::{Connection};
    use log::LevelFilter;

    use crate::resources::{Role, User};

    let _ = env_logger::builder()
        .filter_level(LevelFilter::Info)
        .is_test(true)
        .try_init();

    let mut configuration_file = TuskConfigurationFile::import_from_locations(["../tusk.toml"])
        .expect("configuration_file");

    configuration_file.diesel.url = Secret::from_str(&(configuration_file.diesel.url.expose_secret().to_owned() + "_test"))
        .unwrap();

    log::info!("Emptying test database");

    const MIGRATIONS: diesel_migrations::EmbeddedMigrations = embed_migrations!("../migrations");

    let mut db_connection = PgConnection::establish(configuration_file.diesel.url.expose_secret())
        .expect("database connection");

    db_connection.revert_all_migrations(MIGRATIONS).expect("all migrations reverted");

    let config = configuration_file.clone()
        .into_tusk()
        .expect("configuration");

    config.apply_migrations().expect("database migration");

    let mut db_connection = config.database_connect()
        .expect("database connection");

    User::create(&mut db_connection, "test", Secret::new(String::from("test#7U5c"))).expect("database connection");
    User::create(&mut db_connection, "dummy", Secret::new(String::from("dummy#aW74Qz7"))).expect("database connection");
    User::create(&mut db_connection, "admin", Secret::new(String::from("admin#f9E5"))).expect("database connection");
    User::create(&mut db_connection, "user", Secret::new(String::from("user#vX78"))).expect("database connection");

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
    use std::str::FromStr;
    use secrecy::Secret;
    use crate::config::TuskConfigurationFile;
    use super::TEST_CONFIGURATION;

    /// This test contains the expected configuration.
    #[test]
    fn test_configuration_is_correct() {
        let _config = &*TEST_CONFIGURATION;
    }

    #[test]
    fn secrets_do_not_leak() {
        let config_file = TuskConfigurationFile::import_from_locations(["/some/path/that/does/not/exist/tusk.toml", "../tusk.toml"])
            .expect("correct configuration");
        let debug_config_file = format!("{:?}", config_file);
        assert!(!debug_config_file.contains("postgres://tusk:"));
    }

    #[test]
    fn configuration_parsing() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Info)
            .is_test(true)
            .try_init();

        // No file passed.
        assert!(TuskConfigurationFile::import_from_locations(Vec::<String>::new()).is_err());
        // This is a folder.
        assert!(TuskConfigurationFile::import_from_locations(["/"]).is_err());
        // This file hopefully does not exist.
        assert!(TuskConfigurationFile::import_from_locations(["/some/file/that/hopefully/does/not/exist.1234"]).is_err());
        // This is not a TOML file.
        assert!(TuskConfigurationFile::import_from_locations(["../README.md"]).is_err());
        // The first file is not a TOML file, so the loader should return an error.
        assert!(TuskConfigurationFile::import_from_locations(["../README.md", "../tusk.toml"]).is_err());
        // The first file does not exist, so this is the only occasion in which the second file is loaded.
        let config_file = TuskConfigurationFile::import_from_locations(["/some/path/that/does/not/exist/tusk.toml", "../tusk.toml"])
            .expect("correct configuration");

        // Now testing everything that could go wrong.
        // ----------------------------------------------------------------
        // SSL
        // ----------------------------------------------------------------
        // Certificate file does not exist.
        let mut err_config_file = config_file.clone();
        err_config_file.ssl.cert_file = "/some/garbage/path/to/non-existent/location.pem".into();
        assert!(err_config_file.into_tusk().is_err());
        // Certificate file is not a valid certificate file.
        let mut err_config_file = config_file.clone();
        err_config_file.ssl.cert_file = "../test-empty.txt".into();
        assert!(err_config_file.into_tusk().is_err());
        // Key file does not exist.
        let mut err_config_file = config_file.clone();
        err_config_file.ssl.key_file = "/some/garbage/path/to/non-existent/location.pem".into();
        assert!(err_config_file.into_tusk().is_err());
        // Key file is not a valid key file.
        let mut err_config_file = config_file.clone();
        err_config_file.ssl.key_file = "../test-empty.txt".into();
        assert!(err_config_file.into_tusk().is_err());

        // ----------------------------------------------------------------
        // DIESEL
        // ----------------------------------------------------------------
        // Wrong password.
        let mut err_config_file = config_file.clone();
        err_config_file.diesel.url = Secret::from_str("postgres://tusk:wrong_password@localhost/tusk").unwrap();
        assert!(err_config_file.into_tusk().is_err());
        // Wrong username.
        let mut err_config_file = config_file.clone();
        err_config_file.diesel.url = Secret::from_str("postgres://wrong_username:wrong_password@localhost/tusk").unwrap();
        assert!(err_config_file.into_tusk().is_err());
        // Wrong protocol.
        let mut err_config_file = config_file.clone();
        err_config_file.diesel.url = Secret::from_str("http://wrong_username:wrong_password@localhost/tusk").unwrap();
        assert!(err_config_file.into_tusk().is_err());
        // Not a URL.
        let mut err_config_file = config_file.clone();
        err_config_file.diesel.url = Secret::from_str("Hey there!").unwrap();
        assert!(err_config_file.into_tusk().is_err());
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
            .expect("user storage testing");

        let path = PathBuf::from(TEST_CONFIGURATION.user_directories())
            .canonicalize()
            .expect("canonicalized path");
        log::info!("Loading directories from `{}`", path.display());

        assert_eq!(users_without_directory, 1);
    }
}