use std::fs::DirEntry;
use std::path::PathBuf;
use std::time::SystemTime;
use actix_session::Session;
use actix_web::{HttpRequest, HttpResponse, Responder};
use actix_web::http::header;
use actix_web::web::{self, ServiceConfig};
use log::{error, info, warn};
use secrecy::Secret;
use serde::{Serialize, Deserialize, Serializer};
use serde::ser::SerializeMap;
use tusk_backend::config::TuskData;
use tusk_backend::error::Error;
use tusk_backend::resources;
use tusk_derive::rest_resource;

macro_rules! maybe_error {
    ($expr:expr) => {{
        match $expr {
            Ok(val) => val,
            Err(_) => {
                return HttpResponse::InternalServerError().finish()
            }
        }
    }};
    ($expr:expr, log) => {{
        match $expr {
            Ok(val) => val,
            Err(e) => {
                error!("{e}");
                return HttpResponse::InternalServerError().finish()
            }
        }
    }};
    ($expr:expr, $($err_type:pat => $block:block),*) => {{
        match $expr {
            Ok(val) => val,
            $($err_type => $block),*
            Err(_) => {
                return HttpResponse::InternalServerError().finish()
            }
        }
    }};
    ($expr:expr, log, $($err_type:pat => $block:block),*) => {{
        match $expr {
            Ok(val) => val,
            $($err_type => $block),*
            Err(e) => {
                error!("{e}");
                return HttpResponse::InternalServerError().finish()
            }
        }
    }};
}

macro_rules! extract_username {
    ($session:expr) => {{
        match $session.get::<String>("username") {
            Ok(Some(username)) => username,
            Ok(None) => return HttpResponse::Unauthorized().finish(),
            Err(e) => {
                error!("{e}");
                return HttpResponse::InternalServerError().finish()
            }
        }
    }}
}

#[derive(Deserialize)]
struct SessionFormData {
    username: String,
    password: String
}

#[derive(Serialize)]
struct SessionJsonData {
    username: String
}

pub struct SessionResource;

#[rest_resource("/session")]
impl SessionResource {
    async fn get(session: Session) -> impl Responder {
        let username = extract_username!(session);

        HttpResponse::Ok().json(SessionJsonData { username })
    }

    async fn post(tusk: TuskData, session: Session, form: web::Form<SessionFormData>) -> impl Responder {
        let form = form.into_inner();

        let mut db_connection = maybe_error![tusk.database_connect(), log];

        let username = form.username;
        let password = form.password;

        let user = maybe_error!{resources::User::read_by_username(&mut db_connection, &username), log,
            Err(Error::DatabaseQueryError(tusk_backend::error::DieselQueryError::NotFound)) => {
                // TODO: implement fake password verification.
                warn!("Failed login attempt for user `{username}`");
                return HttpResponse::Unauthorized().finish();
            }};

        if !user.verify_password(&Secret::new(password)) {
            warn!("Failed login attempt for user `{username}`");
            return HttpResponse::Unauthorized().finish();
        }

        session.renew();
        maybe_error![session.insert("username", username.clone())];

        info!("User {username} logged in");

        HttpResponse::Created().finish()
    }

    async fn delete(session: Session) -> impl Responder {
        session.clear();
        session.purge();

        HttpResponse::Ok().finish()
    }
}

#[derive(Deserialize)]
pub struct DirectoryCreate {
    filename: String
}

pub enum FileType {
    File { size: u64 },
    Directory { children: u64 },
    None
}

pub struct DirectoryData {
    filename: String,
    kind: FileType,
    created: u64,
    last_access: u64,
    last_modified: u64
}
impl serde::Serialize for DirectoryData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let (add_len, kind, size, children) = match self.kind {
            FileType::File { size } => (1, "file", Some(size), None),
            FileType::Directory { children } => (1, "directory", None, Some(children)),
            FileType::None => (0, "none", None, None)
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
impl DirectoryData {
    pub fn from_dir_metadata(filename: String, attr: std::fs::Metadata) -> DirectoryData {
        let kind = FileType::Directory { children: 0 };
        let created = attr.created().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let last_access = attr.accessed().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let last_modified = attr.modified().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

        DirectoryData {
            filename,
            kind,
            created,
            last_access,
            last_modified
        }
    }

    pub fn from_dir_entry(entry: DirEntry) -> DirectoryData {
        let filename = entry.file_name().to_string_lossy().into_owned();
        let attr = entry.metadata()
            .unwrap();

        let kind = if attr.is_dir() {
            let children = match std::fs::read_dir(&entry.path()) {
                Ok(sub_dirs) => sub_dirs.into_iter()
                    .filter_map(std::io::Result::ok)
                    .map(|dir| dir.metadata())
                    .filter_map(std::io::Result::ok)
                    .filter(|attr| attr.is_dir())
                    .count() as u64,
                Err(_) => 0
            };
            FileType::Directory { children }
        } else if attr.is_file() {
            let size = attr.len();
            FileType::File { size }
        } else {
            FileType::None
        };
        let created = attr.created().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let last_access = attr.accessed().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let last_modified = attr.modified().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

        DirectoryData {
            filename,
            kind,
            created,
            last_access,
            last_modified
        }
    }
}

pub struct DirectoryResource;

#[rest_resource("/directory/{filename:.*}")]
impl DirectoryResource {
    #[cfg(unix)]
    const ROOT: &'static str = "/srv/cloud/";
    #[cfg(windows)]
    const ROOT: &'static str = "\\\\?\\C:\\srv\\cloud\\";

    fn path_canonicalize(req: &HttpRequest, username: &str) -> Result<PathBuf, HttpResponse> {
        let path: PathBuf = req.match_info().query("filename").parse()
            .unwrap();
        let mut file_path = PathBuf::from(Self::ROOT);
        file_path.extend(path.iter());

        let path = file_path.canonicalize()
            .map_err(|e| { error!("{e}"); HttpResponse::NotFound().finish() })?;

        info!("Queried path {} (exists: {}, safe: {})", path.display(), path.exists(), path.starts_with(Self::ROOT));

        if !path.exists() && !path.starts_with(Self::ROOT) {
            return Err(HttpResponse::NotFound().finish());
        }

        if !path.starts_with(Self::ROOT.to_owned() + ".public") && !path.starts_with(Self::ROOT.to_owned() + username) {
            return Err(HttpResponse::Forbidden().finish());
        }

        Ok(path)
    }

    async fn get(session: Session, req: HttpRequest) -> impl Responder {
        let username = extract_username!(session);

        let path = match Self::path_canonicalize(&req, &username) {
            Ok(path) => path,
            Err(e) => return e
        };

        let attr = std::fs::metadata(&path)
            .unwrap();

        if attr.is_dir() {
            let sub_directories: Vec<_> = match std::fs::read_dir(&path) {
                Ok(sub_dirs) => sub_dirs.into_iter()
                    .filter_map(std::io::Result::ok)
                    .map(DirectoryData::from_dir_entry)
                    .collect(),
                Err(e) => return HttpResponse::InternalServerError().body(format!("{e}"))
            };

            HttpResponse::Ok()
                .json(sub_directories)
        } else if attr.is_file() {
            match actix_files::NamedFile::open(path) {
                Ok(file) => file.into_response(&req),
                Err(e) => HttpResponse::InternalServerError().body(format!("{e}"))
            }
        } else {
            // TODO: return directory hierarchy
            HttpResponse::NotImplemented().finish()
        }
    }

    async fn post(session: Session, directory: web::Form<DirectoryCreate>, req: HttpRequest) -> impl Responder {
        let username = extract_username!(session);
        let directory = directory.into_inner();

        let mut path = match Self::path_canonicalize(&req, &username) {
            Ok(path) => path,
            Err(e) => return e
        };

        if directory.filename.contains(|c| c == '\\' || c == '/') { return HttpResponse::BadRequest().finish(); }
        path.push(&directory.filename);

        maybe_error![std::fs::create_dir(&path), log];
        let attr = maybe_error![std::fs::metadata(&path)];

        // TODO: add location header with the new resource location.
        return HttpResponse::Created()
            //.insert_header((header::LOCATION, ))
            .json(DirectoryData::from_dir_metadata(directory.filename, attr))
    }
}

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(SessionResource)
        .service(DirectoryResource);
    // TODO...
}