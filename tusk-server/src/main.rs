#![warn(missing_docs)]

//! This is the main executable of Tusk Server.
//!
//! It runs a different setup depending on the operating system;
//! in both cases, after the setup, the executable runs as a service/daemon in background, logging
//! to the system logger.

pub mod api;
pub mod error;
pub mod gui;
pub mod os;

use actix_session::SessionMiddleware;
use actix_web::{App, guard, HttpServer, web};
use actix_web::middleware::Logger;

use tusk_backend::error::Result;
use tusk_backend::config::{TuskConfigurationFile, TuskData};

fn main() {
    if let Err(e) = os::run() {
        log::error!("{e}");
    }
}

/// Spawns a new Actix server without activating it.
///
/// To activate the server, simply use `.await` on the `Ok()` result.
///
/// # Error
///
/// This function may return an error.
/// The most common causes are:
/// - The configuration file cannot be found, cannot be read, has an invalid format or has missing items.
/// - TODO
pub fn server_spawn() -> Result<(actix_web::dev::Server, TuskData)> {
    os::initialize_logger();

    let tusk = TuskConfigurationFile::import()?
        .into_tusk()?;
    log::info!("Configuration loaded");
    let redis_store = actix_web::rt::System::new().block_on(tusk.redis_store());
    log::info!("Connected to Redis ");

    tusk.apply_migrations()?;
    let config = tusk.tls_config();

    let data = tusk.to_data();
    let app_data = data.clone();

    let server = HttpServer::new(move || App::new()
        .app_data(app_data.clone())
        .wrap(SessionMiddleware::builder(redis_store.clone(), tusk.session_key())
            .session_lifecycle(tusk.session_lifecycle())
            .cookie_secure(false)
            .build()
        ).wrap(Logger::default())
        .service(web::scope("/v1")
            .guard(guard::Host(tusk.api_domain()))
            .configure(api::configure)
        ).service(web::scope("")
            .guard(guard::Host(tusk.www_domain()))
            .configure(gui::configure)
    )).bind_rustls(("0.0.0.0", 443), config)?
        .run();

    Ok((server, data))
}

#[actix_web::main]
async fn server_run(server: actix_web::dev::Server) -> std::io::Result<()> {
    server.await
}

#[cfg(test)]
mod test {
    use log::LevelFilter;

    pub fn init() {
        let _ = env_logger::builder()
            .filter_level(LevelFilter::Info)
            .is_test(true)
            .try_init();
    }
}