mod gui;
mod api;

use actix_web::{App, guard, HttpServer, web};
use actix_web::web::ServiceConfig;
#[allow(unused)] use log::{error, warn, info, debug, trace};
use simple_logger::SimpleLogger;

fn configure(cfg: &mut ServiceConfig) {
    // Configure GUI
    cfg.service(
        web::scope("")
            .guard(guard::Host("localhost"))
            .configure(gui::configure)
    );
    // Configure API
    cfg.service(
        web::scope("/v1")
            .guard(guard::Host("api.localhost"))
            .configure(api::configure)
    );
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    SimpleLogger::new().init()
        .expect("a functioning logger");

    info!("Dummy configuration loaded.");

    info!("Starting server...");

    HttpServer::new(|| App::new()
        .configure(configure)
    ).bind(("0.0.0.0", 80))?
        .run()
        .await
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