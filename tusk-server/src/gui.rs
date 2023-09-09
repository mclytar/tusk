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
    use actix_web::test::TestRequest;
    use actix_web::{FromRequest, ResponseError, web};
    use actix_web::body::MessageBody;
    use actix_web::http::StatusCode;
    use tusk_core::config::TEST_CONFIGURATION;
    use crate::api::session::test::{create_empty_session, create_session_for_user};
    use crate::gui::GUIResource;

    #[actix_web::test]
    async fn index_loaded_correctly() {
        let tusk = TEST_CONFIGURATION.to_data();
        let req = TestRequest::get()
            .param("path", "")
            .to_http_request();
        let session = create_session_for_user("user").await;

        let path = web::Path::extract(&req).await.expect("A valid path");
        let resp = GUIResource::get(session, tusk, path).await
            .expect("OK");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body()
            .try_into_bytes()
            .expect("Cannot extract body");
        let body = String::from_utf8(body.to_vec())
            .expect("Valid UTF-8");
        assert_eq!(&body, "<html lang=\"en\">\r\n<head><title>Test</title></head>\r\n<body>Index</body>\r\n</html>");
    }

    #[actix_web::test]
    async fn web_page_loaded_correctly() {
        let tusk = TEST_CONFIGURATION.to_data();
        let req = TestRequest::get()
            .param("path", "page")
            .to_http_request();
        let session = create_session_for_user("user").await;

        let path = web::Path::extract(&req).await.expect("A valid path");
        let resp = GUIResource::get(session, tusk, path).await
            .expect("OK");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body()
            .try_into_bytes()
            .expect("Cannot extract body");
        let body = String::from_utf8(body.to_vec())
            .expect("Valid UTF-8");
        assert_eq!(&body, "<html lang=\"en\">\r\n<head><title>Test</title></head>\r\n<body>Hello, Body</body>\r\n</html>");
    }

    #[actix_web::test]
    async fn unauthorized_user_redirected_to_login() {
        let tusk = TEST_CONFIGURATION.to_data();
        let req = TestRequest::get()
            .param("path", "")
            .to_http_request();
        let session = create_empty_session().await;

        let path = web::Path::extract(&req).await.expect("A valid path");
        let resp = GUIResource::get(session, tusk, path).await
            .expect("FOUND");
        assert_eq!(resp.status(), StatusCode::FOUND);
        assert_eq!(resp.headers().get("Location").expect("Location header"), "/login");
    }

    #[actix_web::test]
    async fn serving_not_found_upon_non_existent_page() {
        let tusk = TEST_CONFIGURATION.to_data();
        let req = TestRequest::get()
            .param("path", "this_page_does_not_exist")
            .to_http_request();
        let session = create_session_for_user("user").await;

        let path = web::Path::extract(&req).await.expect("A valid path");
        let resp = GUIResource::get(session, tusk, path).await
            .expect_err("NOT FOUND");
        assert_eq!(resp.status_code(), StatusCode::NOT_FOUND);
    }
}