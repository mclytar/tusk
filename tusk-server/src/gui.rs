//! Web interface.
//!
//! This module contains the necessary functionalities to handle the web requests for the HTML
//! interface (_i.e._, static files, web pages, etc.).

use actix_session::Session;
use actix_web::{HttpResponse, Responder};
use actix_web::http::header::LOCATION;
use actix_web::web::{self, ServiceConfig};
use tusk_core::config::TuskConfiguration;

/// Defines the global resource for a web page.
///
/// Handles all the GUI requests that are not in `/static`, _i.e._, web pages. Every web page is a
/// Tera template which is parsed and then sent to the client as a response.
pub struct GUIResource;
impl GUIResource {
    async fn get(session: Session, data: web::Data<TuskConfiguration>, path: web::Path<String>) -> impl Responder {
        let tusk = data.as_ref();
        let mut path = path.into_inner();
        if path.is_empty() { path = String::from("index"); }

        let mut context = tusk.tera_context();
        context.insert("page", path.as_str());

        let tera = match tusk.tera() {
            Ok(t) => t,
            Err(e) => {
                log::error!("Poison error: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        };

        if &path != "login" {
            let username = match session.get::<String>("username") {
                Ok(Some(username)) => username,
                Ok(None) => return HttpResponse::Found()
                    .insert_header((LOCATION, "/login"))
                    .finish(),
                Err(e) => {
                    log::error!("{e}");
                    return HttpResponse::InternalServerError().finish()
                }
            };

            context.insert("username", &username);
        }

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

/// Configures the server by adding the `/static` service for serving static files and the `/*`
/// service for serving web pages.
pub fn configure(cfg: &mut ServiceConfig, tusk: &TuskConfiguration) {
    cfg.service(actix_files::Files::new("/static", tusk.static_files()));
    cfg.route("/{path:.*}", web::get().to(GUIResource::get));
}

#[cfg(test)]
mod test {
    // TODO: Add tests for GUIResource.
}