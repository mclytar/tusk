use actix_web::{HttpResponse, Responder};
use actix_web::web::ServiceConfig;
use tusk_derive::rest_resource;

pub struct IndexResource;

#[rest_resource("/")]
impl IndexResource {
    async fn get() -> impl Responder {
        HttpResponse::Ok().body("This is the index!")
    }

    async fn post(req_body: String) -> impl Responder {
        HttpResponse::Ok().body(req_body)
    }

    async fn delete() -> impl Responder {
        HttpResponse::Ok().body("Resource deleted!")
    }
}

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(IndexResource);
}