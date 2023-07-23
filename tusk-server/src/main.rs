mod api;
mod error;
mod gui;
mod os;

use std::sync::{Arc, RwLock};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{App, guard, HttpServer, web, cookie::Key};
use actix_web::middleware::Logger;
use actix_web::web::ServiceConfig;
#[allow(unused)] use log::{error, warn, info, debug, trace};
use simple_logger::SimpleLogger;
use tera::Tera;

fn main() -> std::io::Result<()> {
    os::run().unwrap();
    Ok(())
}

fn configure(cfg: &mut ServiceConfig) {
    // Configure API
    cfg.service(
        web::scope("/v1")
            //.guard(guard::Host("api.localhost"))
            .configure(api::configure)
    );
    // Configure GUI
    cfg.service(
        web::scope("")
            //.guard(guard::Host("localhost"))
            .configure(gui::configure)
    );
}

#[derive(Clone, Debug)]
pub struct TuskConfiguration {
    tera: Arc<RwLock<Tera>>
}
impl TuskConfiguration {
    pub fn to_data(&self) -> web::Data<TuskConfiguration> {
        web::Data::new(self.clone())
    }
}
impl Default for TuskConfiguration {
    fn default() -> Self {
        let tera = match Tera::new("_srv/http/**/*.tera") {
            Ok(t) => t,
            Err(e) => {
                error!("Cannot load Tera templates: {}", e);
                ::std::process::exit(1);
            }
        };

        for template in tera.get_template_names() {
            info!("Loaded Tera template {template}");
        }

        let tera = Arc::new(RwLock::new(tera));

        TuskConfiguration {
            tera
        }
    }
}

pub fn server_spawn() -> std::io::Result<actix_web::dev::Server> {
    SimpleLogger::new().init()
        .expect("a functioning logger");

    log::set_max_level(log::LevelFilter::Debug);
    let config = TuskConfiguration::default();
    info!("Dummy configuration loaded.");

    info!("Starting server...");
    let server = HttpServer::new(move || App::new()
        .app_data(config.to_data())
        .wrap(SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
            .cookie_secure(false)
            .build()
        ).wrap(Logger::default())
        .configure(configure)
    ).bind(("0.0.0.0", 80))?
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