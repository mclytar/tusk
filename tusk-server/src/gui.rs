use actix_session::Session;
use actix_web::{HttpResponse, Responder};
use actix_web::http::header::LOCATION;
use actix_web::web::{self, ServiceConfig};
use tusk_derive::rest_resource;
use crate::TuskConfiguration;

pub struct GUIResource;
impl GUIResource {
    async fn get(session: Session, data: web::Data<TuskConfiguration>, path: web::Path<String>) -> impl Responder {
        let tusk = data.as_ref();
        let mut path = path.into_inner();
        if path.is_empty() { path = String::from("index"); }

        if &path != "login" {
            let _ = match session.get::<String>("username") {
                Ok(Some(username)) => username,
                Ok(None) => return HttpResponse::Found()
                    .insert_header((LOCATION, "/login"))
                    .finish(),
                Err(e) => {
                    log::error!("{e}");
                    return HttpResponse::InternalServerError().finish()
                }
            };
        }

        let context = tusk.tera_context();

        let tera = match tusk.tera.read() {
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
    cfg.service(actix_files::Files::new("/static", "/srv/http/static"));
    cfg.route("/{path:.*}", web::get().to(GUIResource::get));
}