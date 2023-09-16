#![warn(missing_docs)]

//! This is the `tusk-server` library supporting the main executable.
//!
//! Contains all the functionality for the server to run.

pub mod api;
pub mod ui;
pub mod os;

use std::path::PathBuf;
use actix_web::{App, guard, HttpServer, web};
use actix_web::middleware::Logger;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use tusk_core::error::TuskResult;
use tusk_core::config::{TuskConfiguration, TuskConfigurationFile};

/// Spawns a Tusk configuration imported from a file.
pub fn spawn_tusk() -> TuskResult<TuskConfiguration> {
    let tusk = TuskConfigurationFile::import_from_default_locations()?
        .into_tusk()?;
    log::info!("Configuration loaded");

    tusk.apply_migrations()?;
    tusk.check_user_directories()?;

    Ok(tusk)
}

/// Spawns a new Actix server without activating it.
///
/// To activate the server, simply use `.await` on the `Ok()` result.
pub fn spawn_server(tusk: &TuskConfiguration) -> TuskResult<actix_web::dev::Server> {
    let tls_config = tusk.tls_config();
    let app_data = tusk.to_data();
    let api_domain = tusk.api_domain().to_owned();
    let www_domain = tusk.www_domain().to_owned();
    let serve_from = tusk.static_files();

    let server = HttpServer::new(move || App::new()
        .app_data(app_data.clone())
        .wrap(app_data.session_middleware())
        .wrap(Logger::default())
        .service(web::scope("/v1")
            .guard(guard::Host(api_domain.clone()))
            .configure(|cfg| api::configure(cfg))
        ).service(web::scope("")
        .guard(guard::Host(www_domain.clone()))
        .configure(|cfg| ui::configure(cfg, serve_from.clone()))
    )).bind_rustls(("0.0.0.0", 443), tls_config)?
        .run();

    Ok(server)
}

/// Spawns a new Actix TEST server.
pub fn spawn_test_server(tusk: &TuskConfiguration) -> actix_test::TestServer {
    let app_data = tusk.to_data();
    let api_domain = tusk.api_domain().to_owned();
    let www_domain = tusk.www_domain().to_owned();
    let serve_from = tusk.static_files();

    let server = actix_test::start(move || App::new()
        .app_data(app_data.clone())
        .wrap(app_data.session_middleware())
        .service(web::scope("/v1")
            .guard(guard::Host(api_domain.clone()))
            .configure(|cfg| api::configure(cfg))
        ).service(web::scope("")
        .guard(guard::Host(www_domain.clone()))
        .configure(|cfg| ui::configure(cfg, serve_from.clone()))
    ));

    server
}

/// Spawns a watcher that watches the Tera templates storage for changes, and reloads Tera if
/// something changed.
pub fn spawn_watcher(tusk: &TuskConfiguration) -> RecommendedWatcher {
    let tusk = tusk.to_data();
    let watch_dir = PathBuf::from(tusk.tera_templates());
    log::info!("Starting watcher for storage `{}`", watch_dir.display());

    let mut watcher = notify::recommended_watcher(move |res| {
        match res {
            Ok(_) => {
                log::info!("Reloading Tera templates after changes...");
                let mut tera = match tusk.tera_mut() {
                    Ok(lock) => lock,
                    Err(e) => { log::error!("{e}"); return; }
                };
                match tera.full_reload() {
                    Ok(()) => {},
                    Err(e) => { log::error!("{e}"); }
                }
            },
            Err(e) => {
                log::error!("{e}");
            }
        }
    }).expect("event watcher");

    watcher.watch(&watch_dir, RecursiveMode::Recursive)
        .expect("watcher set up");

    watcher
}

/// Runs the server.
#[actix_web::main]
#[allow(unused_braces)]
pub async fn run_server(server: actix_web::dev::Server) -> std::io::Result<()> { server.await }