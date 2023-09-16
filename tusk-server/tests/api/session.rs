use std::collections::HashMap;
use actix_web::http::{Method, StatusCode};
use actix_web::http::header::ContentType;
use crate::{await_tusk, PASSWORD_ALICE, Session, SessionPostData, USER_ALICE};

#[actix_web::test]
async fn authorization_success() {
    await_tusk();

    let session = Session::new();

    // At the beginning, the user is unauthorized.
    let resp = session.request(Method::GET, "/v1/session")
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Then, the user logs in and gets a token.
    let resp = session.request(Method::POST, "/v1/session")
        .send_json(&SessionPostData { email: USER_ALICE.email(), password: PASSWORD_ALICE })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let auth_cookie = resp.cookie("id")
        .unwrap();

    // Now, the user is authorized.
    let resp = session.request(Method::GET, "/v1/session")
        .cookie(auth_cookie.clone())
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Then, the user logs out.
    let resp = session.request(Method::DELETE, "/v1/session")
        .cookie(auth_cookie.clone())
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Finally, the user is unauthorized again.
    let resp = session.request(Method::GET, "/v1/session")
        .cookie(auth_cookie.clone())
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn authorization_failure_does_not_leak_anything() {
    await_tusk();

    let session = Session::new();

    let mut resp_wrong_email = session.request(Method::POST, "/v1/session")
        .send_json(&SessionPostData { email: "not-alice@localhost", password: PASSWORD_ALICE })
        .await.unwrap();
    assert_eq!(resp_wrong_email.status(), StatusCode::UNAUTHORIZED);

    let mut resp_wrong_password = session.request(Method::POST, "/v1/session")
        .send_json(&SessionPostData { email: USER_ALICE.email(), password: "not-alice's-password" })
        .await.unwrap();
    assert_eq!(resp_wrong_password.status(), StatusCode::UNAUTHORIZED);

    // Check that headers and body are the same.
    let headers_email: HashMap<String, String> = resp_wrong_email.headers()
        .iter()
        .filter(|(k, _)| &k.to_string() != "date")
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_owned()))
        .collect();
    let headers_pwd: HashMap<String, String> = resp_wrong_password.headers()
        .iter()
        .filter(|(k, _)| &k.to_string() != "date")
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_owned()))
        .collect();
    assert_eq!(headers_email, headers_pwd);
    assert_eq!(resp_wrong_email.body().await.unwrap(), resp_wrong_password.body().await.unwrap());
}

#[actix_web::test]
async fn bad_requests() {
    await_tusk();

    let session = Session::new();

    // Not JSON.
    let resp = session.request(Method::POST, "/v1/session")
        .send_form(&SessionPostData { email: USER_ALICE.email(), password: PASSWORD_ALICE })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Missing field.
    let resp = session.request(Method::POST, "/v1/session")
        .insert_header(ContentType::json())
        .send_body(format!(r#"{{ "email": "{}" }}"#, USER_ALICE.email()))
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}