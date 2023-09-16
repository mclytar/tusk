use std::collections::HashMap;
use std::path::PathBuf;
use actix_web::http::{header, Method, StatusCode};
use actix_web::http::header::ContentType;
use serde::Deserialize;
use crate::{await_tusk, PASSWORD_ALICE, PASSWORD_DANIEL, PASSWORD_EVE, Session, USER_ALICE, USER_DANIEL, USER_EVE};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StoragePathReadKind {
    File,
    Directory,
    None
}
#[derive(Clone, Eq, PartialEq, Debug, Deserialize)]
pub struct StoragePathRead {
    filename: String,
    kind: StoragePathReadKind,
    size: Option<usize>,
    children: Option<usize>,
    created: i64,
    last_access: i64,
    last_modified: i64
}

#[actix_web::test]
async fn create_directory() {
    await_tusk();
    let user_id = USER_DANIEL.id();

    let session = Session::new_authenticated(&USER_DANIEL, PASSWORD_DANIEL).await;
    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/Documents/University"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"directory\", \"name\": \"Projects\" }\r\n\
        --0x0xboundary--").await.unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    assert_eq!(resp.headers().get("location").expect("Header").to_str().unwrap(), &format!("/v1/storage/{user_id}/Documents/University/Projects/"));
    assert!(PathBuf::from(format!("test_srv/storage/{user_id}/Documents/University/Projects")).is_dir());
}

#[actix_web::test]
async fn create_file() {
    await_tusk();
    let user_id = USER_DANIEL.id();

    let session = Session::new_authenticated(&USER_DANIEL, PASSWORD_DANIEL).await;
    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/Documents/Apartment"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"file\", \"name\": \"Electricity Bill.txt\" }\r\n\
        --0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"payload\"; filename=\"Electricity Bill.txt\"\r\n\
        \r\n\
        Amount: 20.25$\nDue before: October, 31st\nProduced as: 78% nuclear, 22% renewable, 0% fossil\r\n\
        --0x0xboundary--").await.unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    assert_eq!(resp.headers().get("location").expect("Header").to_str().unwrap(), &format!("/v1/storage/{user_id}/Documents/Apartment/Electricity Bill.txt"));
    assert_eq!(std::fs::read_to_string(format!("test_srv/storage/{user_id}/Documents/Apartment/Electricity Bill.txt")).expect("File"), "Amount: 20.25$\nDue before: October, 31st\nProduced as: 78% nuclear, 22% renewable, 0% fossil");
}

#[actix_web::test]
async fn file_must_have_payload() {
    await_tusk();
    let user_id = USER_DANIEL.id();

    let session = Session::new_authenticated(&USER_DANIEL, PASSWORD_DANIEL).await;
    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/Documents/Apartment"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"file\", \"name\": \"Water Bill.txt\" }\r\n\
        --0x0xboundary--").await.unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn read_directory() {
    await_tusk();

    let session = Session::new_authenticated(&USER_DANIEL, PASSWORD_DANIEL).await;

    let user_id = USER_DANIEL.id();
    let mut resp = session.request(Method::GET, &format!("/v1/storage/{user_id}/"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let directory: Vec<StoragePathRead> = resp.json().await.unwrap();

    assert_eq!(directory.len(), 4);
    assert_eq!(&directory[0].filename, "Documents");
    assert_eq!(directory[0].kind, StoragePathReadKind::Directory);
    assert!(directory[0].size.is_none());
    assert_eq!(directory[0].children, Some(4));
    assert_eq!(&directory[1].filename, "Multimedia");
    assert_eq!(directory[1].kind, StoragePathReadKind::Directory);
    assert!(directory[1].size.is_none());
    assert_eq!(directory[1].children, Some(0));
    assert_eq!(&directory[2].filename, "Quick Notes");
    assert_eq!(directory[2].kind, StoragePathReadKind::Directory);
    assert!(directory[2].size.is_none());
    assert!(directory[2].children.is_some()); // This may vary during the tests.
    assert_eq!(&directory[3].filename, "README.txt");
    assert_eq!(directory[3].kind, StoragePathReadKind::File);
    assert!(directory[3].size.is_some());
    assert_eq!(directory[3].children, None);
}

#[actix_web::test]
async fn read_file() {
    await_tusk();

    let session = Session::new_authenticated(&USER_DANIEL, PASSWORD_DANIEL).await;

    let user_id = USER_DANIEL.id();
    let mut resp = session.request(Method::GET, &format!("/v1/storage/{user_id}/Quick%20Notes/Shopping%20List.txt"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get(header::CONTENT_TYPE).expect("Header").to_str().unwrap(), ContentType::plaintext().to_string());
    let contents = resp.body().await.unwrap();
    let contents = String::from_utf8_lossy(&contents.as_ref());
    assert_eq!(contents, std::fs::read_to_string(format!("test_srv/storage/{user_id}/Quick Notes/Shopping List.txt")).unwrap());
}

#[actix_web::test]
async fn read_not_found() {
    await_tusk();

    let session = Session::new_authenticated(&USER_DANIEL, PASSWORD_DANIEL).await;

    let user_id = USER_DANIEL.id();
    let resp = session.request(Method::GET, &format!("/v1/storage/{user_id}/Not%20Existent%20Things/"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let resp = session.request(Method::GET, &format!("/v1/storage/{user_id}/not-a-file.txt"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn delete_file() {
    await_tusk();

    let session = Session::new_authenticated(&USER_DANIEL, PASSWORD_DANIEL).await;

    let user_id = USER_DANIEL.id();
    let resp = session.request(Method::DELETE, &format!("/v1/storage/{user_id}/Quick%20Notes/scrap.txt"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[actix_web::test]
async fn delete_directory() {
    await_tusk();

    let session = Session::new_authenticated(&USER_DANIEL, PASSWORD_DANIEL).await;

    let user_id = USER_DANIEL.id();
    let resp = session.request(Method::DELETE, &format!("/v1/storage/{user_id}/Quick%20Notes/Temp"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[actix_web::test]
async fn read_and_write_in_public() {
    await_tusk();

    let session = Session::new_authenticated(&USER_DANIEL, PASSWORD_DANIEL).await;

    let resp = session.request(Method::POST, &format!("/v1/storage/.public/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"file\", \"name\": \"Alice.txt\" }\r\n\
        --0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"payload\"; filename=\"Alice.txt\"\r\n\
        \r\n\
        Hello Alice! How are you?\r\n\
        --0x0xboundary--").await.unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    assert_eq!(resp.headers().get("location").expect("Header").to_str().unwrap(), &format!("/v1/storage/.public/Alice.txt"));
    assert_eq!(std::fs::read_to_string(format!("test_srv/storage/.public/Alice.txt")).expect("File"), "Hello Alice! How are you?");

    let mut resp = session.request(Method::GET, &format!("/v1/storage/.public/Alice.txt"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get(header::CONTENT_TYPE).expect("Header").to_str().unwrap(), ContentType::plaintext().to_string());
    let contents = resp.body().await.unwrap();
    let contents = String::from_utf8_lossy(&contents.as_ref());
    assert_eq!(contents, std::fs::read_to_string(format!("test_srv/storage/.public/Alice.txt")).unwrap());
}

#[actix_web::test]
async fn alice_cannot_use_storage() {
    await_tusk();

    let session = Session::new_authenticated(&USER_ALICE, PASSWORD_ALICE).await;

    let resp = session.request(Method::POST, &format!("/v1/storage/.public/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"file\", \"name\": \"Bob.txt\" }\r\n\
        --0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"payload\"; filename=\"Bob.txt\"\r\n\
        \r\n\
        Hello Bob! How are you?\r\n\
        --0x0xboundary--").await.unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn unauthenticated_cannot_use_storage() {
    await_tusk();

    let session = Session::new();

    let resp = session.request(Method::POST, &format!("/v1/storage/.public/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"file\", \"name\": \"Bob.txt\" }\r\n\
        --0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"payload\"; filename=\"Bob.txt\"\r\n\
        \r\n\
        Hello Bob! How are you?\r\n\
        --0x0xboundary--").await.unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn cannot_create_if_directory_already_exists() {
    await_tusk();

    let session = Session::new_authenticated(&USER_DANIEL, PASSWORD_DANIEL).await;

    let user_id = USER_DANIEL.id();
    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/Documents/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"directory\", \"name\": \"University\" }\r\n\
        --0x0xboundary--").await.unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_web::test]
async fn cannot_create_if_file_already_exists() {
    await_tusk();

    let session = Session::new_authenticated(&USER_DANIEL, PASSWORD_DANIEL).await;

    let user_id = USER_DANIEL.id();
    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/Quick%20Notes/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"file\", \"name\": \"Shopping List.txt\" }\r\n\
        --0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"payload\"; filename=\"Shopping List.txt\"\r\n\
        \r\n\
        - Banana\r\n\
        --0x0xboundary--").await.unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_web::test]
async fn cannot_create_file_inside_file() {
    await_tusk();

    let session = Session::new_authenticated(&USER_DANIEL, PASSWORD_DANIEL).await;

    let user_id = USER_DANIEL.id();
    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/Quick%20Notes/Shopping%20List.txt"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"file\", \"name\": \"Other.txt\" }\r\n\
        --0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"payload\"; filename=\"Other.txt\"\r\n\
        \r\n\
        Some data\r\n\
        --0x0xboundary--").await.unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn unauthorized_read() {
    await_tusk();

    let session = Session::new_authenticated(&USER_EVE, PASSWORD_EVE).await;

    let user_id = USER_DANIEL.id();
    let mut resp_exists = session.request(Method::GET, &format!("/v1/storage/{user_id}/Documents/README.txt"))
        .send().await.unwrap();
    assert_eq!(resp_exists.status(), StatusCode::FORBIDDEN);
    let mut resp_does_not_exist = session.request(Method::GET, &format!("/v1/storage/{user_id}/not-a-file.txt"))
        .send().await.unwrap();
    assert_eq!(resp_does_not_exist.status(), StatusCode::FORBIDDEN);

    // Check that headers and body are the same.
    let headers_user_exists: HashMap<String, String> = resp_exists.headers()
        .iter()
        .filter(|(k, _)| k.as_str() != "set-cookie" && k.as_str() != "date")
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_owned()))
        .collect();
    let headers_user_does_not_exist: HashMap<String, String> = resp_does_not_exist.headers()
        .iter()
        .filter(|(k, _)| k.as_str() != "set-cookie" && k.as_str() != "date")
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_owned()))
        .collect();
    assert_eq!(headers_user_exists, headers_user_does_not_exist);
    assert_eq!(resp_exists.body().await.unwrap(), resp_does_not_exist.body().await.unwrap());
}

#[actix_web::test]
async fn unauthenticated_read() {
    await_tusk();

    let session = Session::new();

    let user_id = USER_DANIEL.id();
    let mut resp_exists = session.request(Method::GET, &format!("/v1/storage/{user_id}/Documents/README.txt"))
        .send().await.unwrap();
    assert_eq!(resp_exists.status(), StatusCode::UNAUTHORIZED);
    let mut resp_does_not_exist = session.request(Method::GET, &format!("/v1/storage/{user_id}/not-a-file.txt"))
        .send().await.unwrap();
    assert_eq!(resp_does_not_exist.status(), StatusCode::UNAUTHORIZED);

    // Check that headers and body are the same.
    let headers_user_exists: HashMap<String, String> = resp_exists.headers()
        .iter()
        .filter(|(k, _)| k.as_str() != "set-cookie" && k.as_str() != "date")
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_owned()))
        .collect();
    let headers_user_does_not_exist: HashMap<String, String> = resp_does_not_exist.headers()
        .iter()
        .filter(|(k, _)| k.as_str() != "set-cookie" && k.as_str() != "date")
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_owned()))
        .collect();
    assert_eq!(headers_user_exists, headers_user_does_not_exist);
    assert_eq!(resp_exists.body().await.unwrap(), resp_does_not_exist.body().await.unwrap());
}

#[actix_web::test]
async fn cannot_delete_user_root() {
    await_tusk();

    let session = Session::new_authenticated(&USER_EVE, PASSWORD_EVE).await;

    let user_id = USER_EVE.id();
    let resp = session.request(Method::DELETE, &format!("/v1/storage/{user_id}/"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn cannot_delete_root() {
    await_tusk();

    let session = Session::new_authenticated(&USER_EVE, PASSWORD_EVE).await;

    let resp = session.request(Method::DELETE, &format!("/v1/storage/"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn cannot_read_root() {
    await_tusk();

    let session = Session::new_authenticated(&USER_EVE, PASSWORD_EVE).await;

    let resp = session.request(Method::GET, &format!("/v1/storage/"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn avoid_path_traversal_attack() {
    await_tusk();

    let session = Session::new_authenticated(&USER_EVE, PASSWORD_EVE).await;

    let eve_id = USER_EVE.id();
    let daniel_id = USER_DANIEL.id();
    let resp = session.request(Method::GET, &format!("/v1/storage/{eve_id}/../{daniel_id}/"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let resp = session.request(Method::GET, &format!("/v1/storage/{eve_id}/../../tusk-test.toml"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn cannot_create_bad_directory() {
    await_tusk();

    let session = Session::new_authenticated(&USER_EVE, PASSWORD_EVE).await;

    let user_id = USER_EVE.id();

    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"directory\", \"name\": \"a/dir\" }\r\n\
        --0x0xboundary--").await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"directory\", \"name\": \"a\\dir\" }\r\n\
        --0x0xboundary--").await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"directory\", \"name\": \".\" }\r\n\
        --0x0xboundary--").await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"directory\", \"name\": \"..\" }\r\n\
        --0x0xboundary--").await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn cannot_create_bad_file() {
    await_tusk();

    let session = Session::new_authenticated(&USER_EVE, PASSWORD_EVE).await;
    let user_id = USER_EVE.id();

    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"file\", \"name\": \"a/dir\" }\r\n\
        \r\n\
        Hello Alice! How are you?\r\n\
        --0x0xboundary--").await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"file\", \"name\": \"a\\dir\" }\r\n\
        \r\n\
        Hello Alice! How are you?\r\n\
        --0x0xboundary--").await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"file\", \"name\": \".\" }\r\n\
        \r\n\
        Hello Alice! How are you?\r\n\
        --0x0xboundary--").await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"file\", \"name\": \"..\" }\r\n\
        \r\n\
        Hello Alice! How are you?\r\n\
        --0x0xboundary--").await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn cannot_create_if_parent_does_not_exist() {
    await_tusk();

    let session = Session::new_authenticated(&USER_EVE, PASSWORD_EVE).await;
    let user_id = USER_EVE.id();

    let resp = session.request(Method::POST, &format!("/v1/storage/{user_id}/does_not_exist/"))
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=\"0x0xboundary\""))
        .send_body("--0x0xboundary\r\n\
        Content-Disposition: form-data; name=\"metadata\"\r\n\
        Content-Type: application/json\r\n\
        \r\n\
        { \"kind\": \"file\", \"name\": \"alice.txt\" }\r\n\
        \r\n\
        Hello Alice! How are you?\r\n\
        --0x0xboundary--").await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}