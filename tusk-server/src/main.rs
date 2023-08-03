mod api;
mod gui;
mod os;

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
    info!("Configuration loaded");
    let redis_store = actix_web::rt::System::new().block_on(tusk.redis_store());
    info!("Connected to Redis ");

    tusk.apply_migrations()?;
    let config = tusk.tls_config();

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