use std::collections::HashMap;
use std::io::ErrorKind;
use std::str::FromStr;
use std::time::{Duration, Instant};
use actix_web::http::{Method, StatusCode};
use serde::Serialize;
use uuid::Uuid;
use tusk_server::api::account::AccountProofType;
use crate::{await_tusk, PASSWORD_ALICE, PASSWORD_BOB_1, PASSWORD_BOB_2, PASSWORD_CHARLIE_1, PASSWORD_CHARLIE_2, PASSWORD_EVE, Session, USER_ALICE, USER_BOB, USER_CHARLIE, USER_EVE};

const RESET_ADDRESS: &'static str = "https://localhost/password_reset/verify?token=";

#[derive(Clone, Debug, Serialize)]
pub struct AccountPasswordPutData<'a> {
    email: &'a str,
    password: Option<&'a str>,
    proof: Option<&'a str>,
    #[serde(default)]
    proof_type: AccountProofType
}

async fn read_mail<S: AsRef<str>>(address: S) -> String {
    let address = address.as_ref();
    let start_time = Instant::now();
    loop {
        if Instant::now() - start_time > Duration::from_millis(5_000) {
            panic!("Mail is taking too long!");
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
        match std::fs::read_to_string(&format!("test_srv/mail/{address}.mail")) {
            Ok(s) => break s,
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => panic!("{e}")
        }
    }
}

#[actix_web::test]
async fn password_reset_request() {
    await_tusk();

    let session = Session::new();

    // Bob has the first password, but does not remember it.
    assert!(session.verify_password(USER_BOB.email(), PASSWORD_BOB_1).await);
    assert!(!session.verify_password(USER_BOB.email(), PASSWORD_BOB_2).await);

    // Send the password reset request.
    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_BOB.email(), password: None, proof: None, proof_type: AccountProofType::None })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    // Bob's first password is still valid.
    assert!(session.verify_password(USER_BOB.email(), PASSWORD_BOB_1).await);
    assert!(!session.verify_password(USER_BOB.email(), PASSWORD_BOB_2).await);

    // Bob receives the email.
    let mail = read_mail(USER_BOB.email()).await;
    let token_pos = mail.find(RESET_ADDRESS)
        .expect("Token") + RESET_ADDRESS.len();
    let token = &mail[token_pos..(token_pos + 36)];
    let token = Uuid::from_str(token)
        .expect("A valid token");

    // Bob sends the request.
    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_BOB.email(), password: Some(PASSWORD_BOB_2), proof: Some(&token.to_string()), proof_type: AccountProofType::Token })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Finally, Bob has managed to change password..
    assert!(!session.verify_password(USER_BOB.email(), PASSWORD_BOB_1).await);
    assert!(session.verify_password(USER_BOB.email(), PASSWORD_BOB_2).await);
}

#[actix_web::test]
async fn password_reset_for_non_existing_users() {
    await_tusk();

    let session = Session::new();

    // Send the password reset request.
    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: "not-bob@example.org", password: None, proof: None, proof_type: AccountProofType::None })
        .await.unwrap();

    // Status code is the same as before.
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[actix_web::test]
async fn correct_password_is_needed () {
    await_tusk();

    let session = Session::new_authenticated(&USER_EVE, PASSWORD_EVE).await;

    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_EVE.email(), password: Some("f248h2fiu2b34978rtu"), proof: Some("1dhg1290r8ug3r982"), proof_type: AccountProofType::Password })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn unauthenticated_bad_requests() {
    await_tusk();

    let session = Session::new();

    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_BOB.email(), password: Some("f248h2fiu2b34978rtu"), proof: Some("1dhg1290r8ug3r982"), proof_type: AccountProofType::None })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_BOB.email(), password: Some("f248h2fiu2b34978rtu"), proof: None, proof_type: AccountProofType::Password })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_BOB.email(), password: Some("f248h2fiu2b34978rtu"), proof: None, proof_type: AccountProofType::Token })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_ALICE.email(), password: None, proof: Some(PASSWORD_ALICE), proof_type: AccountProofType::Password })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn existence_of_user_does_not_leak_to_authenticated_user() {
    await_tusk();

    let session = Session::new_authenticated(&USER_EVE, PASSWORD_EVE).await;

    let mut resp_user_exists = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_ALICE.email(), password: Some("f248h2fiu2b34978rtu"), proof: Some(PASSWORD_ALICE), proof_type: AccountProofType::Password })
        .await.unwrap();
    assert_eq!(resp_user_exists.status(), StatusCode::FORBIDDEN);

    let mut resp_user_does_not_exist = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: "no-user@localhost", password: Some("f248h2fiu2b34978rtu"), proof: Some(PASSWORD_ALICE), proof_type: AccountProofType::Password })
        .await.unwrap();
    assert_eq!(resp_user_does_not_exist.status(), StatusCode::FORBIDDEN);

    // Check that headers and body are the same.
    let headers_user_exists: HashMap<String, String> = resp_user_exists.headers()
        .iter()
        .filter(|(k, _)| k.as_str() != "set-cookie" && k.as_str() != "date")
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_owned()))
        .collect();
    let headers_user_does_not_exist: HashMap<String, String> = resp_user_does_not_exist.headers()
        .iter()
        .filter(|(k, _)| k.as_str() != "set-cookie" && k.as_str() != "date")
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_owned()))
        .collect();
    assert_eq!(headers_user_exists, headers_user_does_not_exist);
    assert_eq!(resp_user_exists.body().await.unwrap(), resp_user_does_not_exist.body().await.unwrap());
}

#[actix_web::test]
async fn existence_of_user_does_not_leak_to_unauthenticated_user() {
    await_tusk();

    let session = Session::new();

    let mut resp_user_exists = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_ALICE.email(), password: Some("f248h2fiu2b34978rtu"), proof: Some(PASSWORD_ALICE), proof_type: AccountProofType::Password })
        .await.unwrap();
    assert_eq!(resp_user_exists.status(), StatusCode::UNAUTHORIZED);

    let mut resp_user_does_not_exist = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: "no-user@localhost", password: Some("f248h2fiu2b34978rtu"), proof: Some(PASSWORD_ALICE), proof_type: AccountProofType::Password })
        .await.unwrap();
    assert_eq!(resp_user_does_not_exist.status(), StatusCode::UNAUTHORIZED);

    // Check that headers and body are the same.
    let headers_user_exists: HashMap<String, String> = resp_user_exists.headers()
        .iter()
        .filter(|(k, _)| k.as_str() != "set-cookie" && k.as_str() != "date")
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_owned()))
        .collect();
    let headers_user_does_not_exist: HashMap<String, String> = resp_user_does_not_exist.headers()
        .iter()
        .filter(|(k, _)| k.as_str() != "set-cookie" && k.as_str() != "date")
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_owned()))
        .collect();
    assert_eq!(headers_user_exists, headers_user_does_not_exist);
    assert_eq!(resp_user_exists.body().await.unwrap(), resp_user_does_not_exist.body().await.unwrap());
}

#[actix_web::test]
async fn authenticated_bad_requests() {
    await_tusk();

    let session = Session::new_authenticated(&USER_EVE, PASSWORD_EVE).await;

    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_BOB.email(), password: Some("f248h2fiu2b34978rtu"), proof: Some("1dhg1290r8ug3r982"), proof_type: AccountProofType::None })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_BOB.email(), password: Some("f248h2fiu2b34978rtu"), proof: None, proof_type: AccountProofType::Password })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_BOB.email(), password: Some("f248h2fiu2b34978rtu"), proof: None, proof_type: AccountProofType::Token })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_ALICE.email(), password: None, proof: Some(PASSWORD_ALICE), proof_type: AccountProofType::Password })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn password_update_request() {
    await_tusk();

    let mut session = Session::new();

    // Charlie has the first password.
    assert!(session.verify_password(USER_CHARLIE.email(), PASSWORD_CHARLIE_1).await);
    assert!(!session.verify_password(USER_CHARLIE.email(), PASSWORD_CHARLIE_2).await);

    // User must be logged in.
    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_CHARLIE.email(), password: Some(PASSWORD_CHARLIE_2), proof: Some(PASSWORD_CHARLIE_1), proof_type: AccountProofType::Password })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Log Charlie in.
    session.authenticate(&USER_CHARLIE, &PASSWORD_CHARLIE_1).await;

    // Now Charlie can change password.
    let resp = session.request(Method::PUT, "/v1/account/password")
        .send_json(&AccountPasswordPutData { email: USER_CHARLIE.email(), password: Some(PASSWORD_CHARLIE_2), proof: Some(PASSWORD_CHARLIE_1), proof_type: AccountProofType::Password })
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Receive the password changed email.
    let _email = read_mail(USER_CHARLIE.email()).await;

    // Finally, Charlie has managed to change password..
    assert!(!session.verify_password(USER_CHARLIE.email(), PASSWORD_CHARLIE_1).await);
    assert!(session.verify_password(USER_CHARLIE.email(), PASSWORD_CHARLIE_2).await);

    // After changing the password, the user is automatically logged out.
    let resp = session.request(Method::GET, "/v1/session")
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}