use actix_web::{HttpResponse, Responder};
use actix_web::web::{self, ServiceConfig};
use tusk_derive::rest_resource;
use crate::TuskConfiguration;

pub struct GUIResource;
impl GUIResource {
    async fn get(data: web::Data<TuskConfiguration>, path: web::Path<String>) -> impl Responder {
        let data = data.as_ref();
        let mut path = path.into_inner();
        if path.is_empty() { path = String::from("index"); }
        let context = tera::Context::new();

        let tera = match data.tera.read() {
            Ok(t) => t,
            Err(e) => {
                log::error!("Poison error: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        };

        let body = match tera.render(&format!("pages/{path}.tera"), &context) {
            Ok(b) => b,
            Err(e) => return match e.kind {
                tera::ErrorKind::TemplateNotFound(e) => {
                    log::error!("Template not found: {e}");
                    HttpResponse::NotFound().finish()
                },
                _ => {
                    log::error!("Tera error: {e}");
                    HttpResponse::InternalServerError().finish()
                }
            }
        };

        HttpResponse::Ok().body(body)
    }
}

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
    cfg.service(actix_files::Files::new("/static", "_srv/http/static"));
    cfg.route("/{path:.*}", web::get().to(GUIResource::get));
}