use std::fs::DirEntry;
use std::path::PathBuf;
use std::time::SystemTime;
use actix_session::Session;
use actix_web::{HttpRequest, HttpResponse, Responder};
use actix_web::web::{self, ServiceConfig};
use log::{error, info, warn};
use secrecy::Secret;
use serde::{Serialize, Deserialize};
use tusk_backend::config::TuskData;
use tusk_backend::error::Error;
use tusk_backend::resources;
use tusk_derive::rest_resource;

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
        let username = match session.get::<String>("username") {
            Ok(Some(username)) => username,
            Ok(None) => return HttpResponse::Unauthorized().finish(),
            Err(e) => {
                error!("{e}");
                return HttpResponse::InternalServerError().finish()
            }
        };

        HttpResponse::Ok().json(SessionJsonData { username })
    }

    async fn post(tusk: TuskData, session: Session, form: web::Form<SessionFormData>) -> impl Responder {
        let form = form.into_inner();

        let mut db_connection = match tusk.database_connect() {
            Ok(db_conn) => db_conn,
            Err(e) => {
                error!("{e}");
                return HttpResponse::InternalServerError().finish();
            }
        };

        let username = form.username;
        let password = form.password;

        let user = match resources::User::read_by_username(&mut db_connection, &username) {
            Ok(user) => user,
            Err(Error::DatabaseQueryError(tusk_backend::error::DieselQueryError::NotFound)) => {
                // TODO: implement fake password verification.
                warn!("Failed login attempt for user `{username}`");
                return HttpResponse::Unauthorized().finish();
            },
            Err(e) => {
                error!("{e}");
                return HttpResponse::InternalServerError().finish();
            }
        };

        if !user.verify_password(&Secret::new(password)) {
            warn!("Failed login attempt for user `{username}`");
            return HttpResponse::Unauthorized().finish();
        }

        session.renew();
        if let Err(e) = session.insert("username", username.clone()) {
            error!("{e}");
            return HttpResponse::InternalServerError().finish();
        }

        info!("User {username} logged in");

        HttpResponse::Created().finish()
    }

    async fn delete(session: Session) -> impl Responder {
        session.clear();
        session.purge();

        HttpResponse::Ok().finish()
    }
}

#[derive(Serialize)]
pub enum FileType {
    File { size: u64 },
    Directory { children: u64 },
    None
}
#[derive(Serialize)]
pub struct DirectoryData {
    filename: String,
    file_type: FileType,
    created: u64,
    last_access: u64,
    last_modified: u64
}
impl DirectoryData {
    pub fn from_dir_entry(entry: DirEntry) -> DirectoryData {
        let filename = entry.file_name().to_string_lossy().into_owned();
        let attr = entry.metadata()
            .unwrap();

        let file_type = if attr.is_dir() {
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
            file_type,
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

    fn path_canonicalize(req: &HttpRequest) -> std::io::Result<Option<PathBuf>> {
        let path: PathBuf = req.match_info().query("filename").parse()
            .unwrap();
        let mut file_path = PathBuf::from(Self::ROOT);
        file_path.extend(path.iter());

        let path = file_path.canonicalize()?;

        info!("Queried path {} (exists: {}, safe: {})", path.display(), path.exists(), path.starts_with(Self::ROOT));

        if path.exists() && path.starts_with(Self::ROOT) {
            Ok(Some(path))
        } else {
            Ok(None)
        }
    }

    async fn get(session: Session, req: HttpRequest) -> impl Responder {
        let username = match session.get::<String>("username") {
            Ok(Some(username)) => username,
            Ok(None) => return HttpResponse::Unauthorized().finish(),
            Err(e) => {
                error!("{e}");
                return HttpResponse::InternalServerError().finish()
            }
        };

        let path = match Self::path_canonicalize(&req) {
            Ok(Some(path)) => path,
            _ => return HttpResponse::NotFound().finish()
        };

        if !path.starts_with(Self::ROOT.to_owned() + ".public") && !path.starts_with(Self::ROOT.to_owned() + &username) {
            return HttpResponse::Forbidden().finish();
        }

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
}

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(SessionResource)
        .service(DirectoryResource);
    // TODO...
}