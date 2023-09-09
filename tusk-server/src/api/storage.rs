//! Contains the CRUD structures relative to the `/storage` REST resource.
//!
//! # Security
//! ## Access
//! Any user can access any file or storage, under the following conditions:
//! - the user must be logged in;
//! - the path should be a valid children of `srv/storage` (i.e. no traversal allowed);
//! - the path of the resource is either in the public root `/.public/` or in the user's root
//!   `/<user>/`, where `<user>` is the username of the user;
//! - the path of the resource exists.
//!
//! If any of these conditions fail, the response will be `UNAUTHORIZED`, if the user is not
//! authenticated,  `FORBIDDEN`, if the user tried to access another user's storage, or
//! `NOT FOUND`, if the resource the user is trying to access does not exist.
//!
//! ## Creation
//! A subdirectory is created by `POST`ing the relative metadata to the storage in which the
//! subdirectory should be created.
//! Similarly, a file is created by `POST`ing the metadata and the contents of the file.
//!
//! The same rules as in the Access section apply, with the additional rule that:
//! - no item should exist in the storage with the same name.
//!
//! Response upon failure is, again, the same as in the Access section, with the additional
//! response `CONFLICT` in case the item that the user is trying to create already exists.
//!
//! ## Deletion
//! A file or subdirectory is deleted by `DELETE`ing the corresponding REST resource.
//!
//! The same rules as in the Access section apply.

use std::future::Future;
use std::io::{ErrorKind};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::{Duration, SystemTime};
use actix_files::NamedFile;
use actix_multipart::form::json::Json;
use actix_multipart::form::MultipartForm;
use actix_multipart::form::tempfile::TempFile;
use actix_session::Session;
use actix_web::{FromRequest, HttpRequest, HttpResponse};
use actix_web::dev::Payload;
use actix_web::http::header;
use serde::{Deserialize, Serializer};
use serde::ser::SerializeMap;
use tusk_core::config::{TuskData};
use tusk_derive::rest_resource;
use crate::error::{HttpError, HttpIfError, HttpOkOr, HttpResult, WrapResult};
use crate::api::session::{SessionRead};

/// Interprets the specified integer into a signed distance, in seconds, from
/// [`SystemTime::UNIX_EPOCH`], and converts it into a [`SystemTime`].
pub fn system_type_from_epoch_delta(delta: i64) -> SystemTime {
    if delta > 0 {
        SystemTime::UNIX_EPOCH + Duration::new(delta as u64, 0)
    } else {
        SystemTime::UNIX_EPOCH - Duration::new(-delta as u64, 0)
    }
}

/// Resource extractor for the requested path.
///
/// Performs the necessary checks and then outputs a valid, authorized path to an existing resource.
#[derive(Clone, Debug)]
pub struct PathInfo {
    depth: usize,
    req: HttpRequest,
    root: PathBuf,
    path: PathBuf
}
impl PathInfo {
    /// Creates a directory in the path.
    ///
    /// # Errors
    /// If the directory to be created contains the symbols `\`, `/` or is `.` or `..`, then
    /// the name of the file is not valid and this function returns an HTTP error 400 `BAD REQUEST`.
    ///
    /// If the parent of this directory does not exist, this function returns an HTTP error
    /// 404 `NOT FOUND`.
    ///
    /// If the storage already exists, this function returns an HTTP error 409 `CONFLICT`.
    ///
    /// Finally, for any other error, the function returns 500 `INTERNAL SERVER ERROR`.
    pub fn create_dir(&self, data: CreateDirectoryData) -> Result<Self, HttpError> {
        let name = data.name();
        if name.contains(|c| c == '/' || c == '\\') || name == "." || name == "..." {
            return Err(HttpError::bad_request());
        }

        let mut path = self.path.clone();
        path.push(name);

        match std::fs::create_dir(&path) {
            Ok(()) => Ok({
                let mut child = self.clone();
                child.path = path;
                child.depth += 1;
                child
            }),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => Err(HttpError::conflict()),
            Err(e) if e.kind() == ErrorKind::NotFound => Err(HttpError::not_found()),
            Err(_) => Err(HttpError::internal_server_error())
        }
    }

    /// Creates a file in the path.
    ///
    /// # Errors
    /// If the directory to be created contains the symbols `\`, `/` or is `.` or `..`, then
    /// the name of the file is not valid and this function returns an HTTP error 400 `BAD REQUEST`.
    ///
    /// If the parent of this directory does not exist, this function returns an HTTP error
    /// 404 `NOT FOUND`.
    ///
    /// If the storage already exists, this function returns an HTTP error 409 `CONFLICT`.
    ///
    /// Finally, for any other error, the function returns 500 `INTERNAL SERVER ERROR`.
    pub fn create_file(&self, data: CreateFileData) -> Result<Self, HttpError> {
        let payload = data.into_payload();
        let name = payload.file_name
            .or_bad_request()?;
        if name.contains(|c| c == '/' || c == '\\') || name == "." || name == "..." {
            return Err(HttpError::bad_request());
        }

        let mut path = self.path.clone();
        path.push(name);

        payload.file.persist(&path)
            .or_internal_server_error()
            .with_log_error()?;

        Ok({
            let mut child = self.clone();
            child.path = path;
            child.depth += 1;
            child
        })
    }

    /// Returns the information relative to the path.
    ///
    /// See [`StoragePathRead::from_path`] for more information.
    pub fn info(&self) -> Result<StoragePathRead, HttpError> {
        StoragePathRead::from_path(&self.path)
    }

    /// Deletes the item at this path.
    ///
    /// # Errors
    /// If the path does not exist, this function returns an HTTP error 404 `NOT FOUND`.
    pub fn delete(self) -> Result<(), HttpError> {
        if self.depth == 0 { return Err(HttpError::not_found()); }
        if self.is_directory() {
            std::fs::remove_dir_all(&self.path)
                .or_not_found()?;
        } else {
            std::fs::remove_file(&self.path)
                .or_not_found()?;
        }
        Ok(())
    }

    /// Returns `true` if this path points to a directory and `false` otherwise.
    pub fn is_directory(&self) -> bool {
        self.path.is_dir()
    }
    /// Returns the internal [`HttpRequest`].
    pub fn request(&self) -> &HttpRequest {
        &self.req
    }
    /// Returns a request path relative to this path.
    pub fn request_path(&self) -> String {
        let result: Vec<std::borrow::Cow<str>> = self.path.iter()
            .skip(self.root.iter().count())
            .map(|s| s.to_string_lossy())
            .collect();
        result.join("/")
    }

    /// Returns a vector of attributes relative to all the files in the storage specified by
    /// this path.
    ///
    /// # Errors
    /// If the path does not exist or is not a storage, this function returns an HTTP error
    /// 404 `NOT FOUND`.
    ///
    /// If the path points to something that is not a directory, this function returns an HTTP
    /// error 409 `CONFLICT`.
    pub fn list_children(&self) -> Result<Vec<StoragePathRead>, HttpError> {
        if !self.path.is_dir() { return Err(HttpError::conflict()); }

        let result = std::fs::read_dir(&self.path)
            .or_not_found()?
            .filter_map(|dir| dir.ok())
            .map(|dir| StoragePathRead::from_path(dir.path()))
            .collect::<Result<Vec<StoragePathRead>, _>>()
            .or_not_found()?;

        Ok(result)
    }
}
impl AsRef<Path> for PathInfo {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
impl FromRequest for PathInfo {
    type Error = HttpError;
    type Future = Pin<Box<dyn Future<Output=Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.to_owned();
        let tusk = TuskData::from_request(&req, payload);
        let session = Session::from_request(&req, payload);
        let queried_path = req.match_info()
            .query("filename")
            .to_owned();

        Box::pin(async move {
            let tusk = tusk.await
                .or_internal_server_error()?;
            let session: SessionRead = session.await
                .or_internal_server_error()?
                .try_into()?;
            let root = tusk.user_directories()
                .canonicalize()
                .or_internal_server_error()
                .with_log_error()?;
            let initiator = session.username().to_owned();

            // Return early if the user is not authorized.
            if !queried_path.starts_with(".public/") && !queried_path.starts_with(&format!("{initiator}/")) {
                log::info!("User `{initiator}` tried to access forbidden path `{queried_path}`");
                return Err(HttpError::forbidden());
            }

            // Construct physical path.
            let mut path = root.clone();
            path.push(queried_path);
            path = path.canonicalize()
                .or_not_found()?;

            // Block any attempt of a path traversal attack.
            if !path.starts_with(&root) {
                return Err(HttpError::not_found());
            }

            // Get the depth to the path, relative to the user root.
            let mut depth = path.ancestors().count() - root.ancestors().count();
            if depth == 0 {
                return Err(HttpError::forbidden());
            }
            depth -= 1;

            Ok(PathInfo {
                depth,
                req,
                root,
                path
            })
        })
    }
}

/// Describes the newly created item as a file or a storage.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Deserialize)]
pub enum CreatePathKind {
    /// The newly created item is a file.
    File,
    /// The newly created item is a storage.
    Directory
}
/// Contains the metadata relative to an uploaded file or storage.
#[derive(Debug, Deserialize)]
pub struct CreatePathAttributes {
    kind: CreatePathKind,
    name: String,
    created: Option<i64>,
    last_access: Option<i64>,
    last_modified: Option<i64>
}
impl CreatePathAttributes {
    /// Returns the creation date and time of the file, if present in the request.
    pub fn created(&self) -> Option<SystemTime> {
        Some(system_type_from_epoch_delta(self.created?))
    }
    /// Returns the last access date and time of the file, if present in the request.
    pub fn last_access(&self) -> Option<SystemTime> {
        Some(system_type_from_epoch_delta(self.last_access?))
    }
    /// Returns the last modification date and time of the file, if present in the request.
    pub fn last_modified(&self) -> Option<SystemTime> {
        Some(system_type_from_epoch_delta(self.last_modified?))
    }
}
/// Represents the CRUD **Create** structure relative to the `/storage` REST resource.
///
/// This can be specialized into a [`CreateFileData`] or a [`CreateDirectoryData`] structure depending
/// on the type of uploaded resource.
#[derive(Debug, MultipartForm)]
struct CreatePathData {
    metadata: Option<Json<CreatePathAttributes>>,
    payload: Option<TempFile>
}
impl CreatePathData {
    /// Creates a new `StoragePathCreate` of type `Directory` with the given name.
    #[cfg(test)]
    pub fn new_directory<S: AsRef<str>>(name: S) -> CreatePathData {
        CreatePathData {
            metadata: Some(Json(CreatePathAttributes {
                kind: CreatePathKind::Directory,
                name: name.as_ref().to_string(),
                created: None,
                last_access: None,
                last_modified: None
            })),
            payload: None
        }
    }
    /// Creates a new `StoragePathCreate` of type `File` with the given name.
    #[cfg(test)]
    pub fn new_file<S: AsRef<str>, C: Into<&'static [u8]>>(name: S, contents: C) -> CreatePathData {
        use std::io::Write;

        let mut file = tempfile::NamedTempFile::new().expect("a named temporary file");
        file.write(contents.into()).expect("file to be written");
        CreatePathData {
            metadata: Some(Json(CreatePathAttributes {
                kind: CreatePathKind::File,
                name: name.as_ref().to_string(),
                created: None,
                last_access: None,
                last_modified: None
            })),
            payload: Some(TempFile {
                file,
                content_type: None,
                file_name: Some(name.as_ref().to_string()),
                size: name.as_ref().len()
            })
        }
    }
    /// Returns `true` if the uploaded resource is a storage and `false` otherwise.
    pub fn is_directory(&self) -> bool {
        if self.payload.is_some() { return false; }
        let metadata = match &self.metadata {
            Some(metadata) => metadata,
            None => return false
        };
        metadata.kind == CreatePathKind::Directory
    }
    /// Returns `true if the uploaded resource is a file and `false` otherwise.
    pub fn is_file(&self) -> bool {
        if self.payload.is_none() { return false; }
        let metadata = match &self.metadata {
            Some(metadata) => metadata,
            None => return false
        };
        metadata.kind == CreatePathKind::File
    }
}
/// Represents the CRUD **Create** structure relative to the `/storage` REST resource.
///
/// This structure contains the necessary information to create a file.
#[derive(Debug)]
pub struct CreateFileData {
    payload: TempFile
}
impl CreateFileData {
    /// Returns the temporary file created by the upload request.
    pub fn into_payload(self) -> TempFile {
        self.payload
    }
}
impl TryFrom<CreatePathData> for CreateFileData {
    type Error = HttpError;

    fn try_from(value: CreatePathData) -> Result<Self, Self::Error> {
        if value.payload.is_none() { return Err(HttpError::bad_request()); }
        let metadata = match value.metadata {
            Some(metadata) => metadata,
            None => return Err(HttpError::bad_request())
        };
        if metadata.kind != CreatePathKind::File {
            return Err(HttpError::bad_request());
        }
        match value.payload {
            Some(payload) => Ok(CreateFileData { payload }),
            None => Err(HttpError::bad_request())
        }
    }
}
/// Represents the CRUD **Create** structure relative to the `/storage` REST resource.
///
/// This structure contains the necessary information to create a storage.
#[derive(Debug)]
pub struct CreateDirectoryData {
    name: String
}
impl CreateDirectoryData {
    /// Returns the name of the storage to be created.
    pub fn name(&self) -> &str {
        &self.name
    }
}
impl TryFrom<CreatePathData> for CreateDirectoryData {
    type Error = HttpError;

    fn try_from(value: CreatePathData) -> Result<Self, Self::Error> {
        if value.payload.is_some() { return Err(HttpError::bad_request()); }
        let Json(directory_item_create) = match value.metadata {
            Some(metadata) => metadata,
            None => return Err(HttpError::bad_request())
        };
        if directory_item_create.kind != CreatePathKind::Directory {
            Err(HttpError::bad_request())
        } else {
            Ok(CreateDirectoryData { name: directory_item_create.name })
        }
    }
}

/// Type of storage item.
#[derive(Clone, Eq, PartialEq, Debug)]
enum StoragePathReadKind {
    /// Item of type file.
    File {
        /// Size, in bytes, of the file.
        size: u64
    },
    /// Item of type storage.
    Directory {
        /// Number of directories inside the current storage.
        children: u64
    },
    /// Unknown or unsupported type.
    None
}
/// Represents the CRUD **Read** structure relative to the `/storage` REST resource.
pub struct StoragePathRead {
    filename: String,
    kind: StoragePathReadKind,
    created: i64,
    last_access: i64,
    last_modified: i64
}
impl StoragePathRead {
    /// Creates a new `DirectoryRead` item by loading the metadata relative to the given `path`.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<StoragePathRead, HttpError> {
        let path = path.as_ref();
        let attr = path.metadata()
            .or_not_found()?;
        let into_lossy_secs = |time_result: std::io::Result<SystemTime>| {
            match time_result {
                Ok(time) => match time.duration_since(SystemTime::UNIX_EPOCH) {
                    Ok(duration) => duration.as_secs() as i64,
                    Err(e) => -(e.duration().as_secs() as i64)
                },
                Err(_) => 0
            }
        };

        let kind = if attr.is_dir() {
            let children = std::fs::read_dir(&path)
                .or_not_found()?
                .filter_map(|dir| dir.ok())
                .filter_map(|dir| dir.metadata().ok())
                .filter(|attr| attr.is_dir())
                .count() as u64;

            StoragePathReadKind::Directory { children }
        } else if attr.is_file() {
            let size = attr.len();

            StoragePathReadKind::File { size }
        } else {
            StoragePathReadKind::None
        };

        Ok(StoragePathRead {
            filename: path.file_name().or_not_found()?.to_string_lossy().into_owned(),
            kind,
            created: into_lossy_secs(attr.created()),
            last_access: into_lossy_secs(attr.accessed()),
            last_modified: into_lossy_secs(attr.modified())
        })
    }
}
impl serde::Serialize for StoragePathRead {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let (add_len, kind, size, children) = match self.kind {
            StoragePathReadKind::File { size } => (1, "file", Some(size), None),
            StoragePathReadKind::Directory { children } => (1, "directory", None, Some(children)),
            StoragePathReadKind::None => (0, "none", None, None)
        };

        let mut map = serializer.serialize_map(Some(5 + add_len))?;
        map.serialize_entry("filename", &self.filename)?;
        map.serialize_entry("kind", kind)?;
        if let Some(size) = size { map.serialize_entry("size", &size)?; }
        if let Some(children) = children { map.serialize_entry("children", &children)?; }
        map.serialize_entry("created", &self.created)?;
        map.serialize_entry("last_access", &self.last_access)?;
        map.serialize_entry("last_modified", &self.last_modified)?;
        map.end()
    }
}
impl TryFrom<&Path> for StoragePathRead {
    type Error = HttpError;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        StoragePathRead::from_path(value)
    }
}

/// Represents the `/storage` REST resource.
///
/// The `/storage` resource is responsible for creating, downloading, uploading or deleting
/// files or directories in the user's storage.
pub struct StorageResource;
#[rest_resource("/storage/{filename:.*}")]
impl StorageResource {
    async fn get(path: PathInfo) -> HttpResult {
        if path.is_directory() {
            let children = path.list_children()?;

            HttpResponse::Ok()
                .json(children)
                .wrap_ok()
        } else {
            NamedFile::open(&path)
                .or_internal_server_error()
                .with_log_error()?
                .into_response(path.request())
                .wrap_ok()
        }
    }

    async fn delete(path: PathInfo) -> HttpResult {
        path.delete()?;

        HttpResponse::Ok()
            .finish()
            .wrap_ok()
    }

    async fn post(path: PathInfo, MultipartForm(data): MultipartForm<CreatePathData>) -> HttpResult {
        if data.is_directory() {
            let directory_data: CreateDirectoryData = data.try_into()?;
            let child = path.create_dir(directory_data)?;
            let attr = child.info()?;
            HttpResponse::Created()
                .insert_header((header::LOCATION, format!("/v1/storage/{}/", child.request_path())))
                .json(attr)
                .wrap_ok()
        } else if data.is_file() {
            let file_data: CreateFileData = data.try_into()?;
            let child = path.create_file(file_data)?;
            let attr = child.info()?;
            HttpResponse::Created()
                .insert_header((header::LOCATION, format!("/v1/storage/{}", child.request_path())))
                .json(attr)
                .wrap_ok()
        } else {
            HttpResponse::BadRequest()
                .finish()
                .wrap_ok()
        }
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;
    use actix_multipart::form::MultipartForm;
    use actix_session::SessionExt;
    use actix_web::{FromRequest, HttpRequest, ResponseError};
    use actix_web::dev::{Payload, ServiceResponse};
    use actix_web::http::{Method, StatusCode};
    use actix_web::test::TestRequest;
    use serde::Deserialize;
    use tusk_core::config::TEST_CONFIGURATION;
    use crate::api::{StorageResource};
    use crate::api::storage::{PathInfo, CreatePathData};

    pub async fn create_request(method: Method, path: &str, user: Option<&str>) -> HttpRequest {
        let tusk = TEST_CONFIGURATION.to_data();
        let req = TestRequest::with_uri(&format!("/v1/storage/{path}"))
            .method(method)
            .app_data(tusk)
            .param("filename", path.to_owned())
            .to_http_request();
        if let Some(user) = user {
            req.get_session().insert("username", user)
                .expect("Cookie set");
        }
        req
    }

    #[actix_web::test]
    async fn read_file_from_user_directory() {
        let req = create_request(Method::GET, "user/user.txt", Some("user")).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect("Valid path");

        let resp = StorageResource::get(path).await
            .expect("response");
        let status = resp.status();
        let body = actix_web::test::read_body(ServiceResponse::new(req, resp)).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body, include_str!("../../srv/storage/user/user.txt"));
    }

    #[actix_web::test]
    async fn read_file_from_public_directory() {
        let req = create_request(Method::GET, ".public/public.txt", Some("user")).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect("Valid path");

        let resp = StorageResource::get(path).await
            .expect("response");
        let status = resp.status();
        let body = actix_web::test::read_body(ServiceResponse::new(req, resp)).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body, include_str!("../../srv/storage/.public/public.txt"));
    }

    #[actix_web::test]
    async fn cannot_access_other_user_directory() {
        let req = create_request(Method::GET, "admin/admin.txt", Some("user")).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect_err("Forbidden path");

        assert_eq!(path.status_code(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn cannot_access_if_not_authenticated() {
        let req = create_request(Method::GET, "user/user.txt", None).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect_err("Unauthorized path");

        assert_eq!(path.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn cannot_access_if_does_not_exist() {
        let req = create_request(Method::GET, "user/does_not_exist.txt", Some("user")).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect_err("Non existent path");

        assert_eq!(path.status_code(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn does_not_leak_existence_if_without_permissions() {
        let req = create_request(Method::GET, "user/does_not_exist.txt", None).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect_err("Non existent path");

        assert_eq!(path.status_code(), StatusCode::UNAUTHORIZED);

        let req = create_request(Method::GET, "user/does_not_exist.txt", Some("admin")).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect_err("Non existent path");

        assert_eq!(path.status_code(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn read_directory_from_user_directory() {
        let req = create_request(Method::GET, "user/", Some("user")).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect("Valid path");

        #[derive(Clone, Debug, Deserialize)]
        pub struct DirectoryItem {
            filename: String,
            kind: String,
            size: Option<usize>,
            children: Option<usize>,
            #[allow(unused)] created: i64,
            #[allow(unused)] last_access: i64,
            #[allow(unused)] last_modified: i64
        }

        let resp = StorageResource::get(path).await
            .expect("response");
        let status = resp.status();
        let body: Vec<DirectoryItem> = actix_web::test::read_body_json(ServiceResponse::new(req, resp)).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.len(), 3);
        assert_eq!(&body[0].filename, "folder_1");
        assert_eq!(&body[0].kind, "directory");
        assert!(&body[0].size.is_none());
        assert_eq!(&body[0].children.unwrap(), &1);
        assert_eq!(&body[1].filename, "folder_2");
        assert_eq!(&body[1].kind, "directory");
        assert!(&body[1].size.is_none());
        assert_eq!(&body[1].children.unwrap(), &0);
        assert_eq!(&body[2].filename, "user.txt");
        assert_eq!(&body[2].kind, "file");
        assert!(&body[2].size.is_some());
        assert!(&body[2].children.is_none());
    }

    #[actix_web::test]
    async fn create_and_delete_file() {
        let req = create_request(Method::GET, "test/", Some("test")).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect("Valid path");

        // Delete the old file in case it exists.
        let _ = std::fs::remove_file("srv/storage/test/test_2.txt");
        assert!(!Path::new("srv/storage/test/test_2.txt").exists());

        // Send request to create a file.
        let item = CreatePathData::new_file("test_2.txt", "Hi, I am a text.".as_bytes());
        let resp = StorageResource::post(path, MultipartForm(item)).await
            .expect("CREATED");

        // Verify.
        assert_eq!(resp.status(), StatusCode::CREATED);
        assert_eq!(resp.headers().get("Location").unwrap(), "/v1/storage/test/test_2.txt");
        assert_eq!(std::fs::read_to_string("srv/storage/test/test_2.txt").expect("a file"), "Hi, I am a text.");

        // Send request to delete the file.
        let req = create_request(Method::GET, "test/test_2.txt", Some("test")).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect("Valid path");
        let resp = StorageResource::delete(path).await
            .unwrap();

        // Verify.
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(!Path::new("srv/storage/test/test_2.txt").exists());
    }

    #[actix_web::test]
    async fn create_and_delete_directory() {
        let req = create_request(Method::GET, "test/", Some("test")).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect("Valid path");

        // Delete the old folder in case it exists.
        let _ = std::fs::remove_dir("srv/storage/test/test_dir/");
        assert!(!Path::new("srv/storage/test/test_dir/").exists());

        // Send request to create a folder.
        let item = CreatePathData::new_directory("test_dir");
        let resp = StorageResource::post(path, MultipartForm(item)).await
            .expect("CREATED");

        // Verify.
        assert_eq!(resp.status(), StatusCode::CREATED);
        assert_eq!(resp.headers().get("Location").unwrap(), "/v1/storage/test/test_dir/");
        assert!(Path::new("srv/storage/test/test_dir/").is_dir());

        // Send request to delete the folder.
        let req = create_request(Method::GET, "test/test_dir/", Some("test")).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect("Valid path");
        let resp = StorageResource::delete(path).await
            .unwrap();

        // Verify.
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(!Path::new("srv/storage/test/test_dir/").exists());
    }

    #[actix_web::test]
    async fn cannot_create_with_same_name() {
        let req = create_request(Method::GET, "test/", Some("test")).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect("Valid path");

        // Send request to create a folder.
        let item = CreatePathData::new_directory("folder_2");
        let resp = StorageResource::post(path, MultipartForm(item)).await
            .expect_err("CONFLICT");

        assert_eq!(resp.status_code(), StatusCode::CONFLICT);
    }

    #[actix_web::test]
    async fn cannot_delete_user_root() {
        let req = create_request(Method::GET, "admin/", Some("admin")).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect("Valid path");

        // Send request to delete a directory.
        let resp = StorageResource::delete(path).await
            .unwrap_err();

        assert_eq!(resp.status_code(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn cannot_access_root() {
        let req = create_request(Method::GET, "", Some("admin")).await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect_err("Invalid path");

        assert_eq!(path.status_code(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn cannot_perform_path_traversal() {
        let req = create_request(Method::GET, "user/../../other_file.txt", Some("user"))
            .await;
        let path = PathInfo::from_request(&req, &mut Payload::None).await
            .expect_err("Invalid path");

        assert_eq!(path.status_code(), StatusCode::NOT_FOUND);
    }
}