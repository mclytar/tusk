//! Web interface.
//!
//! This module contains the necessary functionalities to handle the web requests for the HTML
//! interface (_i.e._, static files, web pages, etc.).

use std::path::PathBuf;
use actix_web::{get, HttpResponse, ResponseError};
use actix_web::http::header::LOCATION;
use actix_web::http::StatusCode;
use actix_web::web::{Path, ServiceConfig};
use tusk_core::config::{Tusk};
use tusk_core::error::{HttpOkOr, TuskHttpResult};

#[get("/login")]
async fn login(tusk: Tusk) -> TuskHttpResult {
    let context = tusk.context();
    let page = tusk.render("pages/login.tera", &context)
        .log_warn()?;
    Ok(HttpResponse::Ok().body(page))
}

#[get("/password_reset/request")]
async fn password_reset_request(tusk: Tusk) -> TuskHttpResult {
    let context = tusk.context();
    let page = tusk.render("pages/password_reset/request.tera", &context)?;
    Ok(HttpResponse::Ok()
        .insert_header((actix_web::http::header::REFERRER_POLICY, "no-referrer"))
        .body(page) )
}

#[get("/password_reset/verify")]
async fn password_reset_verify(tusk: Tusk) -> TuskHttpResult {
    let context = tusk.context();
    let page = tusk.render("pages/password_reset/verify.tera", &context)?;
    Ok(HttpResponse::Ok()
        .insert_header((actix_web::http::header::REFERRER_POLICY, "no-referrer"))
        .body(page) )
}

#[get("/{page}")]
async fn root_page(tusk: Tusk, page: Path<String>) -> TuskHttpResult {
    page_handler(tusk, page).await
}

#[get("/")]
async fn index(tusk: Tusk) -> TuskHttpResult {
    page_handler(tusk, Path::from("index".to_owned())).await
}

async fn page_handler(tusk: Tusk, page: Path<String>) -> TuskHttpResult {
    let mut context = tusk.context();
    let mut db = tusk.db()
        .log_error()?;
    let auth_session = match tusk.authenticate() {
        Ok(user) => user,
        Err(e) if e.status_code() == StatusCode::UNAUTHORIZED => {
            let response = HttpResponse::Found()
                .insert_header((LOCATION, "/login"))
                .finish();
            return Ok(response);
        },
        Err(e) => Err(e)
            .log_error()?
    };
    let roles = auth_session.roles(&mut db)
        .log_error()?;

    context.insert("user", &auth_session);
    context.insert("roles", &roles);

    let user_dir = auth_session.directory(tusk.config())
        .log_error()?;
    context.insert("has_own_dir", &user_dir.exists());

    let page = tusk.render(&format!("pages/{page}.tera"), &context)
        .log_error()?;

    Ok(HttpResponse::Ok().body(page))
}

/// Configures the server by adding the `/static` service for serving static files and the `/*`
/// service for serving web pages.
pub fn configure(cfg: &mut ServiceConfig, serve_from: PathBuf) {
    cfg
        .service(actix_files::Files::new("/static", serve_from))
        .service(login)
        .service(password_reset_request)
        .service(password_reset_verify)
        .service(index)
        .service(root_page)
    ;
}

/*
#[cfg(test)]
mod test {
    use actix_session::Session;
    use actix_web::test::TestRequest;
    use actix_web::{FromRequest, HttpRequest, ResponseError, web};
    use actix_web::body::MessageBody;
    use actix_web::http::StatusCode;
    use uuid::Uuid;
    use tusk_core::config::{TEST_CONFIGURATION, TEST_USER_USER_UUID};
    use crate::api::session::test::{create_empty_session, authenticate_session};
    use crate::ui::GUIResource;

    pub async fn create_request(path: &str, user: Option<(&str, Uuid)>) -> HttpRequest {
        let tusk = TEST_CONFIGURATION.to_data();
        let req = TestRequest::get()
            .app_data(tusk)
            .param("path", path.to_owned())
            .to_http_request();
        if let Some((user, id)) = user {
            let mut session = actix_session::Session::extract(&req).await
                .expect("empty session");
            crate::api::session::test::authenticate_session(&mut session, user, id).await;
        }
        req
    }

    #[actix_web::test]
    async fn index_loaded_correctly() {
        let tusk = TEST_CONFIGURATION.to_data();
        let req = create_request("", Some(("user@example.com", TEST_USER_USER_UUID))).await;
        let session = Session::extract(&req).await
            .expect("A valid session");
        let query = web::Query::extract(&req).await
            .expect("A valid query");

        let path = web::Path::extract(&req).await.expect("A valid path");
        let resp = GUIResource::get(session, tusk, path, query).await
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
        let req = create_request("page", Some(("user@example.com", TEST_USER_USER_UUID))).await;
        let session = Session::extract(&req).await
            .expect("A valid session");
        let query = web::Query::extract(&req).await
            .expect("A valid query");

        let path = web::Path::extract(&req).await.expect("A valid path");
        let resp = GUIResource::get(session, tusk, path, query).await
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
        let req = create_request("", None).await;
        let session = Session::extract(&req).await
            .expect("A valid session");
        let query = web::Query::extract(&req).await
            .expect("A valid query");

        let path = web::Path::extract(&req).await.expect("A valid path");
        let resp = GUIResource::get(session, tusk, path, query).await
            .expect("FOUND");
        assert_eq!(resp.status(), StatusCode::FOUND);
        assert_eq!(resp.headers().get("Location").expect("Location header"), "/login");
    }

    #[actix_web::test]
    async fn serving_not_found_upon_non_existent_page() {
        let tusk = TEST_CONFIGURATION.to_data();
        let req = create_request("this_page_does_not_exist", Some(("user@example.com", TEST_USER_USER_UUID))).await;
        let session = Session::extract(&req).await
            .expect("A valid session");
        let query = web::Query::extract(&req).await
            .expect("A valid query");

        let path = web::Path::extract(&req).await.expect("A valid path");
        let resp = GUIResource::get(session, tusk, path, query).await
            .expect_err("NOT FOUND");
        assert_eq!(resp.status_code(), StatusCode::NOT_FOUND);
    }
}

 */