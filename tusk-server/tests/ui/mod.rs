use actix_web::http::{Method, StatusCode};
use tusk_core::resources::PasswordResetRequest;
use crate::{await_tusk, PASSWORD_ALICE, Session, TUSK, USER_ALICE};

#[actix_web::test]
async fn get_login() {
    await_tusk();

    let session = Session::new();

    let mut resp = session.request(Method::GET, "/login")
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.body().await.unwrap(), r#"<html lang="en">
<head><title>Tusk Index</title></head>
<body><form><input name="email" type="email" /><input name="password" type="password" /></form></body>
</html>"#);
}

#[actix_web::test]
async fn get_index_unauthenticated() {
    await_tusk();

    let session = Session::new();

    // For some reason, it is not possible to ask the client to not follow the requests at this point.
    // Hence, we test against the contents of the login page, rather than the index page.
    let mut resp = session.request(Method::GET, "/index")
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.body().await.unwrap(), r#"<html lang="en">
<head><title>Tusk Index</title></head>
<body><form><input name="email" type="email" /><input name="password" type="password" /></form></body>
</html>"#);
}

#[actix_web::test]
async fn get_index_authenticated() {
    await_tusk();

    let session = Session::new_authenticated(&USER_ALICE, PASSWORD_ALICE).await;

    let mut resp = session.request(Method::GET, "/index")
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.body().await.unwrap(), r#"<html lang="en">
<head><title>Tusk Index</title></head>
<body>Welcome back, Alice!</body>
</html>"#);
}

#[actix_web::test]
async fn root_is_index() {
    await_tusk();

    let session = Session::new_authenticated(&USER_ALICE, PASSWORD_ALICE).await;

    // For some reason, it is not possible to ask the client to not follow the requests at this point.
    // Hence, we test against the contents of the login page, rather than the index page.
    let mut resp = session.request(Method::GET, "/")
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.body().await.unwrap(), r#"<html lang="en">
<head><title>Tusk Index</title></head>
<body>Welcome back, Alice!</body>
</html>"#);
}

#[actix_web::test]
async fn get_password_reset_request() {
    await_tusk();

    let session = Session::new();

    let mut resp = session.request(Method::GET, "/password_reset/request")
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.body().await.unwrap(), r#"<html lang="en">
<head><title>Tusk Index</title></head>
<body><form><input name="email" type="email" /></form></body>
</html>"#);
}

#[actix_web::test]
async fn get_password_reset_verify() {
    await_tusk();

    let session = Session::new();

    // Create a dummy request.
    let mut db = TUSK.db().unwrap();
    let req = PasswordResetRequest::create(&mut db, USER_ALICE.id()).unwrap();
    let token = req.token();

    let mut resp = session.request(Method::GET, format!("/password_reset/verify?token={token}"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.body().await.unwrap(), format!(r#"<html lang="en">
<head><title>Tusk Index</title></head>
<body><form><input name="token" type="hidden" value="{token}" /><input name="email" type="email" /></form></body>
</html>"#));
}

#[actix_web::test]
async fn avoid_bad_strings() {
    await_tusk();

    let session = Session::new();

    // Create a dummy request.
    let mut db = TUSK.db().unwrap();
    let req = PasswordResetRequest::create(&mut db, USER_ALICE.id()).unwrap();
    let token = req.token();

    let resp = session.request(Method::GET, format!("/password_reset/verify?token={token}%22"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}