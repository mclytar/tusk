//! Web interface.
//!
//! This module contains the necessary functionalities to handle the web requests for the HTML
//! interface (_i.e._, static files, web pages, etc.).

use actix_session::Session;
use actix_web::{HttpResponse};
use actix_web::http::header::LOCATION;
use actix_web::web::{self, ServiceConfig};
use tusk_core::config::TuskConfiguration;
use crate::error::{HttpError, HttpIfError, HttpOkOr, HttpResult, WrapResult};

/// Defines the global resource for a web page.
///
/// Handles all the GUI requests that are not in `/static`, _i.e._, web pages. Every web page is a
/// Tera template which is parsed and then sent to the client as a response.
pub struct GUIResource;
impl GUIResource {
    async fn get(session: Session, data: web::Data<TuskConfiguration>, path: web::Path<String>) -> HttpResult {
        let tusk = data.into_inner();
        let mut path = path.into_inner();
        if path.is_empty() { path = String::from("index"); }

        let mut context = tusk.tera_context();
        context.insert("page", path.as_str());

        let tera = match tusk.tera() {
            Ok(tera) => tera,
            Err(e) => return HttpError::internal_server_error()
                .wrap_err()
                .with(|_| log::error!("{e}"))
        };

        if &path != "login" {
            let username = match session.get::<String>("username") {
                Ok(Some(username)) => username,
                Ok(None) => return HttpResponse::Found()
                    .insert_header((LOCATION, "/login"))
                    .finish()
                    .wrap_ok(),
                Err(e) => {
                    log::error!("{e}");
                    return HttpError::internal_server_error().wrap_err()
                }
            };

            let mut db_connection = tusk.database_connect()
                .or_internal_server_error()?;
            let roles = tusk_core::resources::Role::read_by_user_username(&mut db_connection, &username)
                .or_internal_server_error()?;

            context.insert("username", &username);
            context.insert("roles", &roles);

            let mut user_dir = std::path::PathBuf::from(tusk.user_directories());
            user_dir.push(&username);
            context.insert("has_own_dir", &user_dir.exists());
        }

        let body = match tera.render(&format!("pages/{path}.tera"), &context) {
            Ok(b) => b,
            Err(e) => return match e.kind {
                tera::ErrorKind::TemplateNotFound(e) => {
                    log::error!("Template not found: {e}");
                    HttpError::not_found()
                },
                _ => {
                    log::error!("Tera error: {e}");
                    HttpError::internal_server_error()
                }
            }.wrap_err()
        };

        HttpResponse::Ok().body(body).wrap_ok()
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