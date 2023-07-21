use actix_web::{web, App, HttpResponse, HttpServer, Responder, dev};
#[allow(unused)] use log::{error, warn, info, debug, trace};
use simple_logger::SimpleLogger;
use tusk_derive::rest_resource;

pub struct IndexResource;

#[rest_resource("/")]
impl IndexResource {
    async fn get() -> impl Responder {
        debug!("GET /");
        HttpResponse::Ok().body("This is the index!")
    }

    async fn post(req_body: String) -> impl Responder {
        debug!("POST /");
        HttpResponse::Ok().body(req_body)
    }

    async fn delete() -> impl Responder {
        debug!("DELETE /");
        HttpResponse::Ok().body("Resource deleted!")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    SimpleLogger::new().init()
        .expect("a functioning logger");

    info!("Dummy configuration loaded.");

    info!("Starting server...");

    HttpServer::new(|| {
        App::new()
            .service(IndexResource)
    })
        .bind(("0.0.0.0", 80))?
        .run()
        .await
}
