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

use std::io::{ErrorKind};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use actix_files::NamedFile;
use actix_multipart::form::json::Json;
use actix_multipart::form::MultipartForm;
use actix_multipart::form::tempfile::TempFile;
use actix_web::{FromRequest, HttpRequest, HttpResponse};
use actix_web::dev::Payload;
use actix_web::http::header;
use path_clean::clean;
use serde::{Deserialize, Serialize, Serializer};
use serde::ser::SerializeMap;
use tusk_core::config::{BoxedAsyncBlock, Tusk};
use tusk_core::error::{HttpOkOr, TuskError, TuskHttpResult, TuskResult};
use tusk_derive::rest_resource;

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
    pub fn create_dir(&self, data: CreateDirectoryData) -> TuskResult<Self> {
        let name = data.name();
        if name.contains(|c| c == '/' || c == '\\') || name == "." || name == ".." {
            return TuskError::bad_request().bail();
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
            Err(e) if e.kind() == ErrorKind::AlreadyExists => TuskError::conflict().bail(),
            Err(e) if e.kind() == ErrorKind::NotFound => TuskError::not_found().bail(),
            Err(e) if e.kind() == ErrorKind::PermissionDenied => TuskError::forbidden().bail(),
            Err(_) => TuskError::internal_server_error().bail()
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
    pub fn create_file(&self, data: CreateFileData) -> TuskResult<Self> {
        let payload = data.into_payload();
        let name = payload.file_name
            .or_bad_request()?;
        if name.contains(|c| c == '/' || c == '\\') || name == "." || name == ".." {
            return TuskError::bad_request().bail();
        }

        let mut path = self.path.clone();
        path.push(name);

        match payload.file.persist_noclobber(&path) {
            Ok(_) => Ok({
                let mut child = self.clone();
                child.path = path;
                child.depth += 1;
                child
            }),
            Err(e) if e.error.kind() == ErrorKind::AlreadyExists => TuskError::conflict().bail(),
            Err(e) if e.error.kind() == ErrorKind::NotFound => TuskError::not_found().bail(),
            Err(e) if e.error.kind() == ErrorKind::PermissionDenied => TuskError::forbidden().bail(),
            Err(e) => TuskError::internal_server_error().with_error(e).log_error().bail()
        }
    }

    /// Returns the information relative to the path.
    ///
    /// See [`StoragePathRead::from_path`] for more information.
    pub fn info(&self) -> TuskResult<StoragePathRead> {
        StoragePathRead::from_path(&self.path)
    }

    /// Deletes the item at this path.
    ///
    /// # Errors
    /// If the path does not exist, this function returns an HTTP error 404 `NOT FOUND`.
    pub fn delete(self) -> TuskResult<()> {
        if self.depth == 0 { return TuskError::forbidden().bail(); }
        if self.is_directory() {
            std::fs::remove_dir_all(&self.path)?;
        } else {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }

    /// Returns `true` if this path points to a directory and `false` otherwise.
    pub fn is_directory(&self) -> bool { self.path.is_dir() }
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
    pub fn list_children(&self) -> TuskResult<Vec<StoragePathRead>> {
        if !self.path.is_dir() { return TuskError::conflict().bail(); }

        let result = std::fs::read_dir(&self.path)?
            .filter_map(|dir| dir.ok())
            .map(|dir| StoragePathRead::from_path(dir.path()))
            .collect::<Result<Vec<StoragePathRead>, _>>()?;

        Ok(result)
    }
}
impl AsRef<Path> for PathInfo {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
impl FromRequest for PathInfo {
    type Error = TuskError;
    type Future = BoxedAsyncBlock<Self>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let tusk_future = Tusk::extract(req);
        let queried_path:PathBuf = req.match_info()
            .query("filename")
            .into();

        Box::pin(async move {
            let tusk = tusk_future.await?;
            let mut db = tusk.db()?;
            let root = tusk.config()
                .user_directories()
                .canonicalize()?;
            let queried_path = clean(queried_path);
            let initiator = tusk.authenticate()?
                .user(&mut db)?;
            let mut path = root.clone();

            if !initiator.roles(&mut db)?
                .iter()
                .any(|r| r.name() == "directory") {
                return TuskError::forbidden().bail();
            }

            // Return early if the user is not authorized;
            // construct physical path otherwise.
            let user_root = format!("{}/", initiator.id());
            if queried_path.starts_with(".public/") {
                path.push(&queried_path);
            } else if queried_path.starts_with(&user_root) {
                path.push(&queried_path);
            } else {
                log::info!("User `{initiator}` tried to access forbidden path `{}`", queried_path.display());
                return TuskError::forbidden().bail();
            };

            // Get the depth to the path, relative to the user root.
            let mut depth = queried_path.iter().count();
            if depth == 0 {
                return TuskError::forbidden().bail();
            }
            depth -= 1;

            Ok(PathInfo {
                depth,
                root,
                path
            })
        })
    }
}

/// Describes the newly created item as a file or a storage.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathKind {
    /// The newly created item is a file.
    File,
    /// The newly created item is a storage.
    Directory
}
/// Contains the metadata relative to an uploaded file or storage.
#[derive(Debug, Deserialize)]
pub struct CreatePathAttributes {
    kind: PathKind,
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
pub struct CreatePathData {
    metadata: Option<Json<CreatePathAttributes>>,
    payload: Option<TempFile>
}
impl CreatePathData {
    /// Creates a new `StoragePathCreate` of type `Directory` with the given name.
    #[cfg(feature = "test_utils")]
    pub fn new_directory<S: AsRef<str>>(name: S) -> CreatePathData {
        CreatePathData {
            metadata: Some(Json(CreatePathAttributes {
                kind: PathKind::Directory,
                name: name.as_ref().to_string(),
                created: None,
                last_access: None,
                last_modified: None
            })),
            payload: None
        }
    }
    /// Creates a new `StoragePathCreate` of type `File` with the given name.
    #[cfg(feature = "test_utils")]
    pub fn new_file<S: AsRef<str>, C: Into<&'static [u8]>>(name: S, contents: C) -> CreatePathData {
        use std::io::Write;

        let mut file = tempfile::NamedTempFile::new().expect("a named temporary file");
        file.write(contents.into()).expect("file to be written");
        CreatePathData {
            metadata: Some(Json(CreatePathAttributes {
                kind: PathKind::File,
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
        metadata.kind == PathKind::Directory
    }
    /// Returns `true if the uploaded resource is a file and `false` otherwise.
    pub fn is_file(&self) -> bool {
        if self.payload.is_none() { return false; }
        let metadata = match &self.metadata {
            Some(metadata) => metadata,
            None => return false
        };
        metadata.kind == PathKind::File
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
    type Error = TuskError;

    fn try_from(value: CreatePathData) -> Result<Self, Self::Error> {
        if value.payload.is_none() { return TuskError::bad_request().bail(); }
        let metadata = match value.metadata {
            Some(metadata) => metadata,
            None => return TuskError::bad_request().bail()
        };
        if metadata.kind != PathKind::File {
            return TuskError::bad_request().bail();
        }
        match value.payload {
            Some(payload) => Ok(CreateFileData { payload }),
            None => TuskError::bad_request().bail()
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
    type Error = TuskError;

    fn try_from(value: CreatePathData) -> Result<Self, Self::Error> {
        if value.payload.is_some() { return TuskError::bad_request().bail(); }
        let Json(directory_item_create) = match value.metadata {
            Some(metadata) => metadata,
            None => return TuskError::bad_request().bail()
        };
        if directory_item_create.kind != PathKind::Directory {
            TuskError::bad_request().bail()
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
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct StoragePathRead {
    filename: String,
    kind: StoragePathReadKind,
    created: i64,
    last_access: i64,
    last_modified: i64
}
impl StoragePathRead {
    /// Creates a new `DirectoryRead` item by loading the metadata relative to the given `path`.
    pub fn from_path<P: AsRef<Path>>(path: P) -> TuskResult<StoragePathRead> {
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
    type Error = TuskError;

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
    async fn get(path: PathInfo, req: HttpRequest) -> TuskHttpResult {
        if path.is_directory() {
            let children = path.list_children()?;

            Ok(HttpResponse::Ok().json(children))
        } else {
            Ok(NamedFile::open(&path)?.into_response(&req))
        }
    }

    async fn delete(path: PathInfo) -> TuskHttpResult {
        path.delete()?;

        Ok(HttpResponse::NoContent().finish())
    }

    async fn post(path: PathInfo, MultipartForm(data): MultipartForm<CreatePathData>) -> TuskHttpResult {
        let response = if data.is_directory() {
            let directory_data: CreateDirectoryData = data.try_into()?;
            let child = path.create_dir(directory_data)?;
            let attr = child.info()?;
            HttpResponse::Created()
                .insert_header((header::LOCATION, format!("/v1/storage/{}/", child.request_path())))
                .json(attr)
        } else if data.is_file() {
            let file_data: CreateFileData = data.try_into()?;
            let child = path.create_file(file_data)?;
            let attr = child.info()?;
            HttpResponse::Created()
                .insert_header((header::LOCATION, format!("/v1/storage/{}", child.request_path())))
                .json(attr)
        } else {
            HttpResponse::BadRequest()
                .finish()
        };
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};
    use crate::api::storage::system_type_from_epoch_delta;

    #[test]
    fn test_system_type_from_epoch_delta() {
        let time = system_type_from_epoch_delta(0);
        assert_eq!(time, SystemTime::UNIX_EPOCH);

        let time = system_type_from_epoch_delta(5);
        assert_eq!(time, SystemTime::UNIX_EPOCH + Duration::from_secs(5));

        let time = system_type_from_epoch_delta(-60);
        assert_eq!(time, SystemTime::UNIX_EPOCH - Duration::from_secs(60));
    }


}