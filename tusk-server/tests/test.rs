use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use actix_http::Method;
use actix_test::TestServer;
use actix_web::cookie::Cookie;
use actix_web::http::{header, StatusCode};
use log::LevelFilter;
use once_cell::sync::Lazy;
use serde::Serialize;
use tusk_core::config::{TuskConfiguration, TuskConfigurationFile};
use tusk_core::resources::{Role, User};
use tusk_core::test::{diesel_migrations, embed_migrations, EmbeddedMigrations, MigrationHarness};
use tusk_server::spawn_test_server;

pub mod api;
pub mod ui;

pub const READY_STATE_NONE: usize = 0;
pub const READY_STATE_PENDING: usize = 1;
pub const READY_STATE_OK: usize = 2;
pub const READY_STATE_ERR: usize = 3;
pub static READY_STATE: AtomicUsize = AtomicUsize::new(READY_STATE_NONE);

#[derive(Clone, Debug, Serialize)]
pub struct SessionPostData<'a> {
    email: &'a str,
    password: &'a str
}

pub struct Session {
    auth: Option<Cookie<'static>>,
    server: TestServer
}
impl Session {
    pub fn new() -> Session {
        Session {
            auth: None,
            server: spawn_test_server(&TUSK)
        }
    }

    pub async fn new_authenticated(user: &User, password: &str) -> Session {
        let mut session = Self::new();
        session.authenticate(user, password).await;
        session
    }

    pub async fn authenticate(&mut self, user: &User, password: &str) {
        let server = spawn_test_server(&TUSK);
        let response = server.post("/v1/session")
            .timeout(Duration::from_secs(60))
            .insert_header((header::HOST, "localhost"))
            .send_json(&SessionPostData { email: user.email(), password })
            .await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED, "ERROR: {response:?}");
        let cookie = response.cookie("id")
            .unwrap();

        self.auth = Some(cookie);
    }

    pub async fn verify_password<E: AsRef<str>, P: AsRef<str>>(&self, email: E, password: P) -> bool {
        let email = email.as_ref();
        let password = password.as_ref();
        let response = self.server.post("/v1/session")
            .timeout(Duration::from_secs(60))
            .insert_header((header::HOST, "localhost"))
            .send_json(&SessionPostData { email, password })
            .await.unwrap();
        response.status() == StatusCode::CREATED
    }

    pub fn request(&self, method: Method, path: impl AsRef<str>) -> awc::ClientRequest {
        let mut req = self.server.request(method, self.server.url(path.as_ref()))
            .timeout(Duration::from_secs(60))
            .insert_header((header::HOST, "localhost"));
        if let Some(auth) = &self.auth {
            req = req.cookie(auth.to_owned());
        }
        req
    }
}
pub static TUSK: Lazy<TuskConfiguration> = Lazy::new(|| {
    env_logger::builder().filter_level(LevelFilter::Trace).init();

    let tusk = match TuskConfigurationFile::import_from_locations(["tusk-test.toml"]) {
        Ok(tusk) => tusk,
        Err(e) => {
            log::error!("{e}");
            panic!("{e}");
        }
    };
    log::info!("Configuration imported from `tusk-test.toml`");
    let tusk = match tusk.into_tusk() {
        Ok(tusk) => tusk,
        Err(e) => {
            log::error!("{e}");
            panic!("{e}");
        }
    };
    log::info!("Configuration built");

    // Clear the database.
    log::info!("Clearing the database...");
    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../migrations");
    let mut db = tusk.db()
        .expect("Connection to database");
    db.revert_all_migrations(MIGRATIONS)
        .expect("All migrations reverted");
    tusk.apply_migrations()
        .expect("All migrations applied");
    log::info!("Database cleared!");

    // Clear the directories.
    log::info!("Creating directory structure...");
    let _ = std::fs::remove_dir_all("test_srv");
    std::fs::create_dir("test_srv")
        .expect("Directory created");
    std::fs::create_dir("test_srv/mail")
        .expect("Directory created");
    std::fs::create_dir("test_srv/tera")
        .expect("Directory created");
    std::fs::create_dir("test_srv/tera/pages")
        .expect("Directory created");
    std::fs::create_dir("test_srv/tera/pages/password_reset")
        .expect("Directory created");
    std::fs::create_dir("test_srv/static")
        .expect("Directory created");
    std::fs::create_dir("test_srv/storage")
        .expect("Directory created");
    std::fs::create_dir("test_srv/storage/.public")
        .expect("Directory created");
    log::info!("Directory structure created!");

    log::info!("Creating test pages...");
    std::fs::write(format!("test_srv/tera/pages/index.tera"), r#"<html lang="en">
<head><title>Tusk Index</title></head>
<body>Welcome back, {{ user.display }}!</body>
</html>"#)
        .expect("File created");

    std::fs::write(format!("test_srv/tera/pages/login.tera"), r#"<html lang="en">
<head><title>Tusk Index</title></head>
<body><form><input name="email" type="email" /><input name="password" type="password" /></form></body>
</html>"#)
        .expect("File created");

    std::fs::write(format!("test_srv/tera/pages/password_reset/request.tera"), r#"<html lang="en">
<head><title>Tusk Index</title></head>
<body><form><input name="email" type="email" /></form></body>
</html>"#)
        .expect("File created");

    std::fs::write(format!("test_srv/tera/pages/password_reset/verify.tera"), r#"<html lang="en">
<head><title>Tusk Index</title></head>
<body><form><input name="token" type="hidden" value="{{ get['token'] }}" /><input name="email" type="email" /></form></body>
</html>"#)
        .expect("File created");
    log::info!("Test pages created!");

    tusk
});

// ----------------------------------------------------------------
// RETRIEVE ROLE admin
// ----------------------------------------------------------------
pub static ROLE_ADMIN: Lazy<Role> = Lazy::new(|| {
    let mut db = TUSK.db()
        .expect("Connection to database");

    Role::from_name(&mut db, "admin")
        .expect("Connection to database")
        .expect("Role `admin`")
});

// ----------------------------------------------------------------
// RETRIEVE ROLE directory
// ----------------------------------------------------------------
pub static ROLE_DIRECTORY: Lazy<Role> = Lazy::new(|| {
    let mut db = TUSK.db()
        .expect("Connection to database");

    Role::from_name(&mut db, "directory")
        .expect("Connection to database")
        .expect("Role `directory`")
});

// ----------------------------------------------------------------
// RETRIEVE ROLE user
// ----------------------------------------------------------------
pub static ROLE_USER: Lazy<Role> = Lazy::new(|| {
    let mut db = TUSK.db()
        .expect("Connection to database");

    Role::from_name(&mut db, "user")
        .expect("Connection to database")
        .expect("Role `user`")
});

// ----------------------------------------------------------------
// CREATE USER Alice
// ----------------------------------------------------------------
pub static PASSWORD_ALICE: &'static str = "alice#777aFUsb8SVg";
pub static USER_ALICE: Lazy<User> = Lazy::new(|| {
    let mut db = TUSK.db()
        .expect("Connection to database");

    let (user, None) = User::builder("alice@localhost")
        .display("Alice")
        .password(PASSWORD_ALICE)
        .build(&mut db)
        .expect("Created user") else { unreachable!("Password is already set") };

    ROLE_USER.assign_to(&mut db, &user)
        .expect("Role assigned");
    // ROLE_DIRECTORY.assign_to(&mut db, &user)
    //     .expect("Role assigned");
    //
    // std::fs::create_dir(format!("test_srv/storage/{}", user.id()))
    //     .expect("Directory created");

    log::info!("Created user `Alice <alice@localhost>` with roles `User`");

    user
});

// ----------------------------------------------------------------
// CREATE USER Bob
// ----------------------------------------------------------------
pub static PASSWORD_BOB_1: &'static str = "bob#777aFUsb8SVg";
pub static PASSWORD_BOB_2: &'static str = "bob#5fdRG23rGSfg";
/// Only used to test password reset functionality;
/// do not use anywhere else.
pub static USER_BOB: Lazy<User> = Lazy::new(|| {
    let mut db = TUSK.db()
        .expect("Connection to database");

    let (user, None) = User::builder("bob@example.com")
        .display("Bob")
        .password(PASSWORD_BOB_1)
        .build(&mut db)
        .expect("Created user") else { unreachable!("Password is already set") };

    ROLE_USER.assign_to(&mut db, &user)
        .expect("Role assigned");
    // ROLE_DIRECTORY.assign_to(&mut db, &user)
    //     .expect("Role assigned");
    //
    // std::fs::create_dir(format!("test_srv/storage/{}", user.id()))
    //     .expect("Directory created");

    log::info!("Created user `Bob <bob@example.com>` with roles `User`");

    user
});

// ----------------------------------------------------------------
// CREATE USER Charlie
// ----------------------------------------------------------------
pub static PASSWORD_CHARLIE_1: &'static str = "charlie#dKf7PqL412mR";
pub static PASSWORD_CHARLIE_2: &'static str = "charlie#BkQsLtD1214R";
/// Only used to test password update functionality;
/// do not use anywhere else.
pub static USER_CHARLIE: Lazy<User> = Lazy::new(|| {
    let mut db = TUSK.db()
        .expect("Connection to database");

    let (user, None) = User::builder("charlie@example.com")
        .display("Charlie")
        .password(PASSWORD_CHARLIE_1)
        .build(&mut db)
        .expect("Created user") else { unreachable!("Password is already set") };

    ROLE_USER.assign_to(&mut db, &user)
        .expect("Role assigned");
    // ROLE_DIRECTORY.assign_to(&mut db, &user)
    //     .expect("Role assigned");
    //
    // std::fs::create_dir(format!("test_srv/storage/{}", user.id()))
    //     .expect("Directory created");

    log::info!("Created user `Charlie <charlie@example.com>` with roles `User`");

    user
});

// ----------------------------------------------------------------
// CREATE USER Charlie
// ----------------------------------------------------------------
pub static PASSWORD_DANIEL: &'static str = "daniel#dKf7PqL412mR";
/// Daniel, the **D**irectory user.
pub static USER_DANIEL: Lazy<User> = Lazy::new(|| {
    let mut db = TUSK.db()
        .expect("Connection to database");

    let (user, None) = User::builder("daniel@localhost")
        .display("Daniel")
        .password(PASSWORD_DANIEL)
        .build(&mut db)
        .expect("Created user") else { unreachable!("Password is already set") };

    ROLE_USER.assign_to(&mut db, &user)
        .expect("Role assigned");
    ROLE_DIRECTORY.assign_to(&mut db, &user)
        .expect("Role assigned");

    let user_id = user.id();
    std::fs::create_dir(format!("test_srv/storage/{user_id}"))
        .expect("Directory created");
    std::fs::create_dir(format!("test_srv/storage/{user_id}/Documents"))
        .expect("Directory created");
    std::fs::create_dir(format!("test_srv/storage/{user_id}/Documents/Apartment"))
        .expect("Directory created");
    std::fs::create_dir(format!("test_srv/storage/{user_id}/Documents/Other"))
        .expect("Directory created");
    std::fs::create_dir(format!("test_srv/storage/{user_id}/Documents/University"))
        .expect("Directory created");
    std::fs::create_dir(format!("test_srv/storage/{user_id}/Documents/Workplace"))
        .expect("Directory created");
    std::fs::create_dir(format!("test_srv/storage/{user_id}/Multimedia"))
        .expect("Directory created");
    std::fs::create_dir(format!("test_srv/storage/{user_id}/Quick Notes"))
        .expect("Directory created");
    std::fs::create_dir(format!("test_srv/storage/{user_id}/Quick Notes/Temp"))
        .expect("Directory created");
    std::fs::create_dir(format!("test_srv/storage/{user_id}/Quick Notes/Temp/Something"))
        .expect("Directory created");

    std::fs::write(format!("test_srv/storage/{user_id}/README.txt"), r#"This is your new storage!"#)
        .expect("File created");

    std::fs::write(format!("test_srv/storage/{user_id}/Documents/README.txt"), r#"Put here all your documents"#)
        .expect("File created");

    std::fs::write(format!("test_srv/storage/{user_id}/Quick Notes/Shopping List.txt"), r#"Shopping list:
- Salt
- Pepper
- Sugar (possibly syntactic)
- Ajax
- Vim"#)
        .expect("File created");

    std::fs::write(format!("test_srv/storage/{user_id}/Quick Notes/scrap.txt"), r#"Delete me!"#)
        .expect("File created");

    std::fs::write(format!("test_srv/storage/{user_id}/Quick Notes/Temp/scrap.txt"), r#"Delete me!"#)
        .expect("File created");

    std::fs::write(format!("test_srv/storage/{user_id}/Quick Notes/Temp/Something/scrap.txt"), r#"Delete me!"#)
        .expect("File created");

    log::info!("Created user `Daniel <daniel@example.com>` with roles `Directory, User`");

    user
});

// ----------------------------------------------------------------
// CREATE USER Charlie
// ----------------------------------------------------------------
pub static PASSWORD_EVE: &'static str = "eve#dKf7PqL412mR";
/// Malicious user.
pub static USER_EVE: Lazy<User> = Lazy::new(|| {
    let mut db = TUSK.db()
        .expect("Connection to database");

    let (user, None) = User::builder("eve@example.com")
        .display("Eve")
        .password(PASSWORD_EVE)
        .build(&mut db)
        .expect("Created user") else { unreachable!("Password is already set") };

    ROLE_USER.assign_to(&mut db, &user)
        .expect("Role assigned");
    ROLE_DIRECTORY.assign_to(&mut db, &user)
        .expect("Role assigned");

    std::fs::create_dir(format!("test_srv/storage/{}", user.id()))
        .expect("Directory created");

    log::info!("Created user `Eve <eve@example.com>` with roles `Directory, User`");

    user
});

/// Runs all the lazy closures for the users, actually loading them in memory and creating the respective file structure.
pub fn await_tusk() {
    loop {
        match READY_STATE.compare_exchange(READY_STATE_NONE, READY_STATE_PENDING, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(READY_STATE_NONE) => continue,
            Err(READY_STATE_PENDING) => {
                let start = Instant::now();
                while READY_STATE.load(Ordering::SeqCst) == READY_STATE_PENDING {
                    if start - Instant::now() > Duration::from_secs(30) {
                        READY_STATE.store(READY_STATE_ERR, Ordering::SeqCst);
                        panic!("Initialization has been poisoned!");
                    }
                }
                if READY_STATE.load(Ordering::SeqCst) == READY_STATE_ERR {
                    panic!("Initialization has been poisoned!");
                }
            },
            Err(READY_STATE_OK) => return,
            Err(_) => {
                READY_STATE.store(READY_STATE_ERR, Ordering::SeqCst);
                panic!("Initialization has been poisoned!");
            }
        }
    }

    let _tusk = Lazy::force(&TUSK);

    let _admin = Lazy::force(&ROLE_ADMIN);
    let _directory = Lazy::force(&ROLE_DIRECTORY);
    let _user = Lazy::force(&ROLE_USER);

    let alice = std::thread::spawn(|| Lazy::force(&USER_ALICE));
    let bob = std::thread::spawn(|| Lazy::force(&USER_BOB));
    let charlie = std::thread::spawn(|| Lazy::force(&USER_CHARLIE));
    let daniel = std::thread::spawn(|| Lazy::force(&USER_DANIEL));
    let eve = std::thread::spawn(|| Lazy::force(&USER_EVE));

    alice.join().unwrap();
    bob.join().unwrap();
    charlie.join().unwrap();
    daniel.join().unwrap();
    eve.join().unwrap();

    match READY_STATE.compare_exchange(READY_STATE_PENDING, READY_STATE_OK, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(_) => {},
        Err(e) => {
            READY_STATE.store(READY_STATE_ERR, Ordering::SeqCst);
            panic!("Initialization has been poisoned (previous state: {e})!");
        }
    }
}

#[test]
fn test_setup_works() {
    await_tusk();
}