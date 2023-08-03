mod api;
mod gui;
mod os;

use std::fs::File;
use std::io::BufReader;
#[allow(unused)] use log::{debug, error, info, trace, warn};

use actix_session::SessionMiddleware;
use actix_web::{App, guard, HttpServer, web};
use actix_web::middleware::Logger;

use tusk_backend::error::Result;
use tusk_backend::config::TuskConfigurationFile;

fn main() {
    if let Err(e) = os::run() {
        error!("{e}");
    }
}

pub fn server_spawn() -> Result<actix_web::dev::Server> {
    os::initialize_logger();

    let tusk = TuskConfigurationFile::import()?
        .into_tusk()?;
    let redis_store = actix_web::rt::System::new().block_on(tusk.redis_store());

    info!("Configuration loaded");

    #[cfg(windows)]
    let file = File::open("C:\\ProgramData\\Tusk\\tusk.crt")?;
    #[cfg(unix)]
    let file = File::open("/etc/tusk/domains/server-dev.local/cert.pem")?;
    let mut reader = BufReader::new(file);
    let certs: Vec<_> = rustls_pemfile::certs(&mut reader)?
        .into_iter()
        .map(rustls::Certificate)
        .collect();

    info!("Found {} certificates.", certs.len());

    #[cfg(windows)]
    let file = File::open("C:\\ProgramData\\Tusk\\tusk.key")?;
    #[cfg(unix)]
    let file = File::open("/etc/tusk/domains/server-dev.local/key.pem")?;
    let mut reader = BufReader::new(file);
    let keys: Vec<_> = rustls_pemfile::pkcs8_private_keys(&mut reader)?
        .into_iter()
        .map(rustls::PrivateKey)
        .collect();

    info!("Found {} keys, using the first one available.", keys.len());

    let key = keys.into_iter()
        .next()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No key in file 'tusk.key'."))?;

    info!("Key file loaded");

    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    info!("Starting server...");
    
    let server = HttpServer::new(move || App::new()
        .app_data(tusk.to_data())
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

    Ok(server)
}

#[actix_web::main]
async fn server_run(server: actix_web::dev::Server) -> std::io::Result<()> {
    server.await
}

#[cfg(test)]
mod tests {
    use actix_web::test;

    #[actix_web::test]
    async fn test_get_() {
        let app = test::init_service(App::new().configure(super::configure)).await;
        let req = test::TestRequest::get()
            .uri("http://localhost/")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(body::to_bytes(resp.into_body()).await.unwrap(), "This is the index!");
    }
}