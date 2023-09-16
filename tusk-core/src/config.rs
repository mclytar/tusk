//! This module contains the necessary structures and functions to load the configuration from
//! `tusk.toml`.

mod diesel;
mod mail;
mod redis;
mod ssl;
mod tusk;

use std::collections::HashMap;
use std::future::Future;
use std::io::{ErrorKind};
use std::path::{PathBuf};
use std::pin::Pin;
use std::sync::{Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard};

use actix_web::{cookie, FromRequest, HttpRequest, web};
use ::diesel::{r2d2::{ConnectionManager, Pool, PooledConnection}, PgConnection, Connection};
use ::diesel_migrations::{embed_migrations, MigrationHarness};
use actix_session::{Session, SessionMiddleware};
use actix_session::config::{PersistentSession, TtlExtensionPolicy};
use actix_session::storage::RedisSessionStore;
use actix_web::dev::Payload;
use lettre::{Message, SmtpTransport, Transport};
use lettre::transport::smtp::response::Response;
use serde::Deserialize;
use tera::{Context, Tera};
use crate::{DieselError, PooledPgConnection};

use crate::error::{HttpOkOr, TuskError, TuskResult};
use crate::resources::User;
use crate::session::AuthenticatedSession;

/// `actix_web::web::Data` wrapper for [`TuskConfiguration`].
pub type TuskData = web::Data<TuskConfiguration>;
/// Boxed async block type to deal with custom implementors of [`FromRequest`].
pub type BoxedAsyncBlock<T> = Pin<Box<dyn Future<Output = Result<T, <T as FromRequest>::Error>>>>;

/// Main extractor for all the requests.
///
/// Contains configuration data, session data and several utility methods to easily address
/// the HTTP requests.
pub struct Tusk {
    config: Arc<TuskConfiguration>,
    get: HashMap<String, String>,
    session: Session
}
impl Tusk {
    /// Returns an [`AuthenticatedSession`] if the user is logged in, and an `UNAUTHORIZED` error
    /// otherwise.
    pub fn authenticate(&self) -> TuskResult<AuthenticatedSession> {
        AuthenticatedSession::try_from(&self.session)
    }
    /// Creates a new session for the specified user, effectively logging in the user.
    pub fn log_in(&self, user: &User) -> TuskResult<()> {
        self.session.renew();
        self.session.insert("auth_session", AuthenticatedSession::from(user))
            .or_internal_server_error()
            .log_error()
    }
    /// Deletes the session from the browser and from the backend,
    /// effectively logging out the user.
    pub fn log_out(&self) {
        self.session.clear();
        self.session.purge();
    }
    /// Returns a reference to the configuration data loaded from the file.
    pub fn config(&self) -> &TuskConfiguration {
        self.config.as_ref()
    }
    /// Returns a Tera context with the default information.
    pub fn context(&self) -> Context {
        let mut context = tera::Context::new();

        context.insert("protocol", "https");
        context.insert("www_domain", self.config.www_domain());
        context.insert("api_domain", self.config.api_domain());
        context.insert("ui_icon_filetype", self.config.ui_icon_filetype());
        context.insert("get", &self.get);

        context
    }
    /// Returns a connection to the database.
    pub fn db(&self) -> TuskResult<PooledPgConnection> {
        Ok(self.config.db()?)
    }
    /// Returns a rendered Tera page.
    pub fn render<S: AsRef<str>>(&self, name: S, context: &Context) -> TuskResult<String> {
        let tera = match self.config.tera.read() {
            Ok(tera) => tera,
            Err(_) => return TuskError::internal_server_error().bail()
        };
        let result = tera.render(name.as_ref(), context)?;
        Ok(result)
    }
    /// Sends the given message by email.
    pub fn send_email(&self, message: &Message) -> TuskResult<Response> {
        self.config.send_email(message)
    }
}
impl FromRequest for Tusk {
    type Error = TuskError;
    type Future = BoxedAsyncBlock<Self>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let config_future = web::Data::<TuskConfiguration>::extract(req);
        let get_future = web::Query::<HashMap<String, String>>::extract(req);
        let session_future = Session::extract(req);

        Box::pin(async move {
            let config = match config_future.await {
                Ok(config) => config.into_inner(),
                Err(e) => {
                    log::error!("{e}");
                    return TuskError::internal_server_error().bail();
                }
            };
            let get = match get_future.await {
                Ok(get) => get.into_inner(),
                Err(e) => {
                    log::error!("{e}");
                    return TuskError::internal_server_error().bail();
                }
            };
            let session = match session_future.await {
                Ok(session) => session,
                Err(e) => {
                    log::error!("{e}");
                    return TuskError::internal_server_error().bail();
                }
            };

            Ok(Tusk {
                config,
                get,
                session
            })
        })
    }
}

/// Represents the file `tusk.toml`.
#[derive(Clone, Debug, Deserialize)]
pub struct TuskConfigurationFile {
    diesel: diesel::Diesel,
    mail: mail::Mail,
    redis: redis::Redis,
    ssl: ssl::Ssl,
    tusk: tusk::Tusk
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
        #[allow(unused)]
        let tusk::Tusk {
            log_level,
            www_domain,
            api_domain,
            contacts,
            serve,
            ui: tusk::ui::Ui {
                icon_filetype: ui_icon_filetype
            }
        } = self.tusk;

        let tera_templates = serve.tera_templates();
        let static_files = serve.static_files();
        let user_directories = serve.user_directories();

        #[cfg(not(test))]
        log::set_max_level(log_level);

        log::info!("Loading Tera templates from `{}`", tera_templates.display());
        log::info!("Loading static files from `{}`", static_files.display());
        log::info!("Loading user directories from `{}`", user_directories.display());

        let tera = serve.tera()?;
        let database_pool = self.diesel.pool()?;
        let tls_server_configuration = self.ssl.into_server_configuration()?;
        let mailer = self.mail.mailer()?;

        let session_key = cookie::Key::generate();
        let session_store = self.redis.session_storage()?;

        let config = TuskConfiguration {
            tera,
            serve,
            www_domain,
            api_domain,
            database_pool,
            session_key,
            session_store,
            tls_server_configuration,
            ui_icon_filetype,
            mailer,
            email_contacts: contacts
        };

        Ok(config)
    }
}

/// Represents a configuration for the Tusk server.
#[derive(Clone)]
pub struct TuskConfiguration {
    tera: Arc<RwLock<Tera>>,
    serve: tusk::serve::Serve,
    www_domain: String,
    api_domain: String,
    database_pool: Arc<Pool<ConnectionManager<PgConnection>>>,
    session_key: cookie::Key,
    session_store: RedisSessionStore,
    tls_server_configuration: rustls::ServerConfig,
    ui_icon_filetype: String,
    mailer: SmtpTransport,
    email_contacts: tusk::contacts::Contacts
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
    pub fn tera_templates(&self) -> PathBuf {
        self.serve.tera_templates()
    }
    /// Returns the path from which to serve static files.
    pub fn static_files(&self) -> PathBuf {
        self.serve.static_files()
    }
    /// Returns the path where user files are stored.
    pub fn user_directories(&self) -> PathBuf {
        self.serve.user_directories()
    }
    /// Returns the file extension for the UI icons.
    pub fn ui_icon_filetype(&self) -> &str {
        &self.ui_icon_filetype
    }
    /// Sends the given message by email.
    pub fn send_email(&self, message: &Message) -> TuskResult<Response> {
        let response = self.mailer.send(message)?;
        Ok(response)
    }
    /// Returns the email contacts relative to the server.
    pub fn email_contacts(&self) -> &tusk::contacts::Contacts {
        &self.email_contacts
    }
    /// Returns a connection to the database.
    pub fn db(&self) -> TuskResult<PooledConnection<ConnectionManager<PgConnection>>> {
        let db_pool = self.database_pool.get()?;
        Ok(db_pool)
    }
    /// Applies all the pending migrations.
    pub fn apply_migrations(&self) -> TuskResult<()> {
        const MIGRATIONS: diesel_migrations::EmbeddedMigrations = embed_migrations!("../migrations");

        let mut db_connection = self.db()?;

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
    /// Returns a new session middleware constructed from the internal configuration.
    pub fn session_middleware(&self) -> SessionMiddleware<RedisSessionStore> {
        SessionMiddleware::builder(self.session_store.clone(), self.session_key.clone())
            .session_lifecycle(PersistentSession::default()
                .session_ttl(cookie::time::Duration::minutes(15))
                .session_ttl_extension_policy(TtlExtensionPolicy::OnEveryRequest)
            ).build()
    }
    /// Returns the current TLS configuration.
    pub fn tls_config(&self) -> rustls::ServerConfig {
        self.tls_server_configuration.clone()
    }
    /// Checks whether all the users with role `storage` actually have a storage, and logs
    /// a warning in case not.
    pub fn check_user_directories(&self) -> TuskResult<usize> {
        let mut db_connection = self.db()?;
        let mut count = 0;

        let directory_users = db_connection.transaction(|db_connection| {
            crate::resources::Role::from_name(db_connection, "directory")?
                .ok_or(DieselError::NotFound)?
                .users(db_connection)
        })?;
        let directory_path = self.user_directories();

        log::info!("Checking directories in `{}`", directory_path.display());
        for dir_user in directory_users {
            let mut user_dir_path = directory_path.clone();
            user_dir_path.push(dir_user.id().to_string());
            if !user_dir_path.exists() {
                count += 1;
                log::warn!("Missing storage for user `{}`", dir_user.email());
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::TuskConfigurationFile;

    #[test]
    fn test_basic_derives() {
        let tusk_file = TuskConfigurationFile::import_from_locations(["../tusk-server/tusk-test.toml"])
            .unwrap();

        // It clones.
        let tusk_file = tusk_file.clone();
        // It debugs.
        let _dbg_string = format!("{tusk_file:?}");
    }
}

/*
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
//#[cfg(any(feature = "test_utils", test))]
#[cfg(test)]
pub static TEST_CONFIGURATION: once_cell::sync::Lazy<TuskConfiguration> = once_cell::sync::Lazy::new(|| {
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

    let mut db_connection = config.db()
        .expect("database connection");

    fn create_user(db_connection: &mut PgConnection, index: u64, email: &str, display: &str, password: &str) -> User {
        use diesel::prelude::*;
        use crate::schema::user;

        let id = Uuid::from_u64_pair(0, index);
        let password = bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .unwrap();

        diesel::insert_into(user::table)
            .values(
                (user::user_id.eq(id), user::email.eq(email), user::display.eq(display), user::password.eq(&password))
            ).get_result(db_connection)
            .expect("Database connection")
    }

    let _user_test = create_user(&mut db_connection, 1, "test@example.com", "Test", "test#7U5c");
    let user_dummy = create_user(&mut db_connection, 2, "dummy@example.com", "Dummy", "dummy#aW74Qz7");
    let user_admin = create_user(&mut db_connection, 3, "admin@example.com", "Admin", "admin#f9E5");
    let user_user = create_user(&mut db_connection, 4, "user@example.com", "User", "user#vX78");

    let user_test = User::from_id(&mut db_connection, Uuid::from_u64_pair(0, 1))
        .expect("User");
    assert_eq!(user_test.id().to_string(), "00000000-0000-0000-0000-000000000001".to_owned());

    Role::from_name(&mut db_connection, "admin")
        .expect("Role")
        .expect("Existing role")
        .assign_to(&mut db_connection, &user_admin)
        .expect("Role assignment");

    let role_directory = Role::from_name(&mut db_connection, "directory")
        .expect("Role")
        .expect("Existing role");
    role_directory.assign_to(&mut db_connection, &user_admin).expect("Role assignment");
    role_directory.assign_to(&mut db_connection, &user_test).expect("Role assignment");
    role_directory.assign_to(&mut db_connection, &user_user).expect("Role assignment");

    let role_user = Role::from_name(&mut db_connection, "user")
        .expect("Role")
        .expect("Existing role");
    role_user.assign_to(&mut db_connection, &user_admin).expect("Role assignment");
    role_user.assign_to(&mut db_connection, &user_dummy).expect("Role assignment");
    role_user.assign_to(&mut db_connection, &user_test).expect("Role assignment");
    role_user.assign_to(&mut db_connection, &user_user).expect("Role assignment");

    config
});

/// Contains the UUID for the test user named `test@example.com`.
#[cfg(any(feature = "test_utils", test))]
pub const TEST_USER_TEST_UUID: Uuid = Uuid::from_u64_pair(0, 1);
/// Contains the UUID for the test user named `dummy@example.com`.
#[cfg(any(feature = "test_utils", test))]
pub const TEST_USER_DUMMY_UUID: Uuid = Uuid::from_u64_pair(0, 2);
/// Contains the UUID for the test user named `admin@example.com`.
#[cfg(any(feature = "test_utils", test))]
pub const TEST_USER_ADMIN_UUID: Uuid = Uuid::from_u64_pair(0, 3);
/// Contains the UUID for the test user named `user@example.com`.
#[cfg(any(feature = "test_utils", test))]
pub const TEST_USER_USER_UUID: Uuid = Uuid::from_u64_pair(0, 4);

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
}*/