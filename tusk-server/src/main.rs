use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
#[allow(unused)] use log::{error, warn, info, debug, trace};
use simple_logger::SimpleLogger;

#[get("/")]
async fn hello() -> impl Responder {
    debug!("Requested '/'.");
    HttpResponse::Ok().body("Hello world!")
}

#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    debug!("Requested '/echo'.");
    HttpResponse::Ok().body(req_body)
}

async fn manual_hello() -> impl Responder {
    debug!("Requested '/hey'.");
    HttpResponse::Ok().body("Hey there!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    SimpleLogger::new().init()
        .expect("a functioning logger");

    info!("Dummy configuration loaded.");

    info!("Starting server...");

    HttpServer::new(|| {
        App::new()
            .service(hello)
            .service(echo)
            .route("/hey", web::get().to(manual_hello))
    })
        .bind(("127.0.0.1", 80))?
        .run()
        .await
}
