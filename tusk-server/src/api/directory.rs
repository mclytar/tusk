//! Contains the CRUD structures relative to the `/directory` REST resource.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use actix_multipart::form::json::Json;
use actix_multipart::form::MultipartForm;
use actix_multipart::form::tempfile::TempFile;
use serde::{Deserialize, Serializer};
use serde::ser::SerializeMap;
use crate::error::{HttpError, HttpIfError, HttpOkOr};

/// A path in the server folder.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct DirectoryPath {
    root: PathBuf,
    path: PathBuf
}
impl DirectoryPath {
    /// Creates a new `DirectoryPath` from a root, given in the `tusk.toml` configuration file,
    /// and a given path.
    pub fn with_root<R: AsRef<Path>, P: AsRef<Path>>(root: R, path: P) -> Result<Self, HttpError> {
        let root = root.as_ref().canonicalize()
            .or_internal_server_error()
            .with_log_error()?;
        let query = path.as_ref().to_path_buf();
        let mut path = root.clone();
        path.push(query);

        log::info!("Queried path {}", path.display());

        path = path.canonicalize()
            .or_not_found()?;

        if !path.starts_with(&root) {
            return Err(HttpError::not_found());
        }

        let path = DirectoryPath { root, path };

        Ok(path)
    }

    /// Returns `true` if this path belongs to the user named `username` and `false` otherwise.
    pub fn belongs_to<S: AsRef<str>>(&self, username: S) -> bool {
        let username = username.as_ref();
        let mut public_path = self.root.clone();
        public_path.push(".public");
        let mut user_path = self.root.clone();
        user_path.push(username);

        self.path.starts_with(public_path) || self.path.starts_with(user_path)
    }

    /// Returns `Ok(())` if the user is authorized to access the path and an error
    /// with status code 403 `FORBIDDEN` otherwise.
    pub fn authorize_for<S: AsRef<str>>(&self, username: S) -> Result<(), HttpError> {
        if !self.belongs_to(&username) {
            log::warn!("User {} tried to access path {}", username.as_ref(), self.path.display());
            Err(HttpError::forbidden())
        } else {
            Ok(())
        }
    }

    /// Adds a component to the path.
    pub fn push<P: AsRef<Path>>(&mut self, path: P) {
        self.path.push(path);
    }

    /// Creates the path if it does not exist.
    ///
    /// # Errors
    /// If the parent of this path does not exist, this function returns an HTTP error
    /// 404 `NOT FOUND`.
    ///
    /// If the folder already exists, this function returns an HTTP error 409 `CONFLICT`.
    pub fn create(&self) -> Result<(), HttpError> {
        match std::fs::create_dir(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => Err(HttpError::conflict()),
            Err(e) if e.kind() == ErrorKind::NotFound => Err(HttpError::not_found()),
            Err(_) => Err(HttpError::internal_server_error())
        }
    }

    /// Deletes the item at this path.
    ///
    /// # Errors
    /// If the path does not exist, this function returns an HTTP error 404 `NOT FOUND`.
    pub fn delete(&self) -> Result<(), HttpError> {
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

    /// Returns a vector of attributes relative to all the files in the directory specified by
    /// this path.
    ///
    /// # Errors
    /// If the path does not exist or is not a folder, this function returns an HTTP error
    /// 404 `NOT FOUND`.
    pub fn list_children(&self) -> Result<Vec<DirectoryItemRead>, HttpError> {
        if !self.path.is_dir() { return Err(HttpError::conflict()); }

        let result = std::fs::read_dir(&self.path)
            .or_not_found()?
            .filter_map(|dir| dir.ok())
            .map(|dir| DirectoryItemRead::from_path(dir.path()))
            .collect::<Result<Vec<DirectoryItemRead>, _>>()
            .or_not_found()?;

        Ok(result)
    }
}
impl AsRef<Path> for DirectoryPath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

/// Describes the newly created item as a file or a folder.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Deserialize)]
pub enum DirectoryItemCreateKind {
    /// The newly created item is a file.
    File,
    /// The newly created item is a folder.
    Folder
}
/// Contains the metadata relative to an uploaded file or folder.
#[derive(Debug, Deserialize)]
pub struct DirectoryItemCreateMetadata {
    kind: DirectoryItemCreateKind,
    name: String,
    created: Option<i64>,
    last_access: Option<i64>,
    last_modified: Option<i64>
}
impl DirectoryItemCreateMetadata {
    /// Returns the creation date and time of the file, if present in the request.
    pub fn created(&self) -> Option<SystemTime> {
        let created = self.created?;

        let date = if created > 0 {
            SystemTime::UNIX_EPOCH + Duration::new(created as u64, 0)
        } else {
            SystemTime::UNIX_EPOCH - Duration::new(-created as u64, 0)
        };

        Some(date)
    }
    /// Returns the last access date and time of the file, if present in the request.
    pub fn last_access(&self) -> Option<SystemTime> {
        let last_access = self.last_access?;

        let date = if last_access > 0 {
            SystemTime::UNIX_EPOCH + Duration::new(last_access as u64, 0)
        } else {
            SystemTime::UNIX_EPOCH - Duration::new(-last_access as u64, 0)
        };

        Some(date)
    }
    /// Returns the last modification date and time of the file, if present in the request.
    pub fn last_modified(&self) -> Option<SystemTime> {
        let last_modified = self.last_modified?;

        let date = if last_modified > 0 {
            SystemTime::UNIX_EPOCH + Duration::new(last_modified as u64, 0)
        } else {
            SystemTime::UNIX_EPOCH - Duration::new(-last_modified as u64, 0)
        };

        Some(date)
    }
}
/// Represents the CRUD **Create** structure relative to the `/directory` REST resource.
///
/// This can be specialized into a [`FileCreate`] or a [`FolderCreate`] structure depending
/// on the type of uploaded resource.
#[derive(Debug, MultipartForm)]
pub struct DirectoryItemCreate {
    metadata: Option<Json<DirectoryItemCreateMetadata>>,
    payload: Option<TempFile>
}
impl DirectoryItemCreate {
    /// Returns `true` if the uploaded resource is a folder and `false` otherwise.
    pub fn is_folder(&self) -> bool {
        if self.payload.is_some() { return false; }
        let metadata = match &self.metadata {
            Some(metadata) => metadata,
            None => return false
        };
        metadata.kind == DirectoryItemCreateKind::Folder
    }
    /// Returns `true if the uploaded resource is a file and `false` otherwise.
    pub fn is_file(&self) -> bool {
        if self.payload.is_none() { return false; }
        let metadata = match &self.metadata {
            Some(metadata) => metadata,
            None => return false
        };
        metadata.kind == DirectoryItemCreateKind::File
    }
}
/// Represents the CRUD **Create** structure relative to the `/directory` REST resource.
///
/// This structure contains the necessary information to create a file.
#[derive(Debug)]
pub struct FileCreate {
    payload: TempFile
}
impl FileCreate {
    /// Returns a reference to the temporary file created by the upload request.
    pub fn payload(&self) -> &TempFile {
        &self.payload
    }
    /// Returns the temporary file created by the upload request.
    pub fn into_payload(self) -> TempFile {
        self.payload
    }
}
impl TryFrom<DirectoryItemCreate> for FileCreate {
    type Error = HttpError;

    fn try_from(value: DirectoryItemCreate) -> Result<Self, Self::Error> {
        if value.payload.is_none() { return Err(HttpError::bad_request()); }
        let metadata = match value.metadata {
            Some(metadata) => metadata,
            None => return Err(HttpError::bad_request())
        };
        if metadata.kind != DirectoryItemCreateKind::File {
            return Err(HttpError::bad_request());
        }
        match value.payload {
            Some(payload) => Ok(FileCreate { payload }),
            None => Err(HttpError::bad_request())
        }
    }
}
/// Represents the CRUD **Create** structure relative to the `/directory` REST resource.
///
/// This structure contains the necessary information to create a folder.
#[derive(Debug)]
pub struct FolderCreate {
    name: String
}
impl FolderCreate {
    /// Returns the name of the folder to be created.
    pub fn name(&self) -> &str {
        &self.name
    }
}
impl TryFrom<DirectoryItemCreate> for FolderCreate {
    type Error = HttpError;

    fn try_from(value: DirectoryItemCreate) -> Result<Self, Self::Error> {
        if value.payload.is_some() { return Err(HttpError::bad_request()); }
        let Json(directory_item_create) = match value.metadata {
            Some(metadata) => metadata,
            None => return Err(HttpError::bad_request())
        };
        if directory_item_create.kind != DirectoryItemCreateKind::Folder {
            Err(HttpError::bad_request())
        } else {
            Ok(FolderCreate { name: directory_item_create.name })
        }
    }
}

/// Type of directory item.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum DirectoryItemReadKind {
    /// Item of type file.
    File {
        /// Size, in bytes, of the file.
        size: u64
    },
    /// Item of type directory.
    Folder {
        /// Number of folders inside the current folder.
        children: u64
    },
    /// Unknown or unsupported type.
    None
}
/// Represents the CRUD **Read** structure relative to the `/directory` REST resource.
pub struct DirectoryItemRead {
    filename: String,
    kind: DirectoryItemReadKind,
    created: i64,
    last_access: i64,
    last_modified: i64
}
impl DirectoryItemRead {
    /// Creates a new `DirectoryRead` item by loading the metadata relative to the given `path`.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<DirectoryItemRead, HttpError> {
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

            DirectoryItemReadKind::Folder { children }
        } else if attr.is_file() {
            let size = attr.len();

            DirectoryItemReadKind::File { size }
        } else {
            DirectoryItemReadKind::None
        };

        Ok(DirectoryItemRead {
            filename: path.file_name().or_not_found()?.to_string_lossy().into_owned(),
            kind,
            created: into_lossy_secs(attr.created()),
            last_access: into_lossy_secs(attr.accessed()),
            last_modified: into_lossy_secs(attr.modified())
        })
    }
}
impl serde::Serialize for DirectoryItemRead {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let (add_len, kind, size, children) = match self.kind {
            DirectoryItemReadKind::File { size } => (1, "file", Some(size), None),
            DirectoryItemReadKind::Folder { children } => (1, "directory", None, Some(children)),
            DirectoryItemReadKind::None => (0, "none", None, None)
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
impl TryFrom<DirectoryPath> for DirectoryItemRead {
    type Error = HttpError;

    fn try_from(value: DirectoryPath) -> Result<Self, Self::Error> {
        DirectoryItemRead::from_path(value.path)
    }
}

#[cfg(test)]
mod test {
    use actix_web::http::StatusCode;
    use actix_web::ResponseError;
    use crate::api::directory::DirectoryPath;
    use crate::test::init;

    #[test]
    fn test_directory_path_permission() {
        init();

        let public_path = DirectoryPath::with_root("../srv/directory/", ".public/some_folder/some_file.txt").unwrap();
        let user_path = DirectoryPath::with_root("../srv/directory/", "test/some_folder/some_file.txt").unwrap();

        assert!(public_path.authorize_for("test").is_ok());
        assert!(public_path.authorize_for("dummy").is_ok());
        assert!(user_path.authorize_for("test").is_ok());
        assert_eq!(user_path.authorize_for("dummy").unwrap_err().status_code(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_directory_path_create() {
        init();

        let mut user_path = DirectoryPath::with_root("../srv/directory/", "test/").unwrap();
        user_path.push("my_folder");
        let mut user_path_folder = user_path.clone();
        user_path_folder.push("my_other_folder");

        // Delete the file in case it already exists because of a previous failed test.
        let mut path = user_path.path.clone();
        path.push("my_folder");
        let _ = std::fs::remove_dir_all(path);

        // Start the actual test.

        // At the beginning, nothing exists.
        assert!(!user_path.is_directory());
        assert!(!user_path_folder.is_directory());

        // If the parent folder does not exist, return NOT_FOUND.
        assert_eq!(user_path_folder.create().unwrap_err().status_code(), StatusCode::NOT_FOUND);

        user_path.create().unwrap();
        assert!(user_path.is_directory());
        assert!(!user_path_folder.is_directory());

        // If the directory already exists, return CONFLICT.
        assert_eq!(user_path.create().unwrap_err().status_code(), StatusCode::CONFLICT);

        user_path_folder.create().unwrap();
        assert!(user_path.is_directory());
        assert!(user_path_folder.is_directory());

        // Delete the path.
        user_path.delete().unwrap();
        assert!(!user_path.is_directory());
        assert!(!user_path_folder.is_directory());

        // If the folder does not exist, return NOT_FOUND.
        assert_eq!(user_path.delete().unwrap_err().status_code(), StatusCode::NOT_FOUND);
    }

    // TODO: Add test for DirectoryPath::list_children.
    // TODO: Add tests for DirectoryItemCreate, DirectoryItemRead.
}