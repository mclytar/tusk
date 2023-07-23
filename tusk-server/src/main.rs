mod api;
mod error;
mod gui;
mod os;
mod settings;

#[allow(unused)] use log::{error, warn, info, debug, trace};

use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{App, guard, HttpServer, web, cookie::Key};
use actix_web::middleware::Logger;
use settings::TuskConfiguration;
use crate::settings::TuskConfigurationFile;

fn main() -> std::io::Result<()> {
    os::run().unwrap();
    Ok(())
}

pub fn server_spawn() -> std::io::Result<actix_web::dev::Server> {
    os::initialize_logger();

    let tusk = TuskConfigurationFile::import()?
        .into_tusk();

    info!("Starting server...");
    let server = HttpServer::new(move || App::new()
        .app_data(tusk.to_data())
        .wrap(SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
            .cookie_domain(Some(tusk.www_domain().to_owned()))
            .cookie_secure(false)
            .build()
        ).wrap(Logger::default())
        .service(web::scope("/v1")
            .guard(guard::Host(tusk.api_domain()))
            .configure(api::configure)
        ).service(web::scope("")
            .guard(guard::Host(tusk.www_domain()))
            .configure(gui::configure)
    )).bind(("0.0.0.0", 80))?
        .run();

    Ok(server)
}

#[actix_web::main]
async fn server_run(server: actix_web::dev::Server) -> std::io::Result<()> {
    server.await
}

#[cfg(test)]
mod tests {
    use actix_web::{body, http, App, test};

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