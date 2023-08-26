//! Resources in the REST API.
//!
//! Contains the structures relative to the REST resources handled by the API.
//! Every structure is relative to a single resource and contains the HTTP methods to handle it.
//! Helper serializable/deserializable structures are contained in the respective modules, and they
//! address the relative CRUD methods.

pub mod session;
pub mod directory;

use actix_files::NamedFile;
use actix_multipart::form::MultipartForm;
use actix_session::Session;
use actix_web::{HttpRequest, HttpResponse, Responder};
use actix_web::http::header;
use actix_web::web::{self, ServiceConfig};
use tusk_core::config::{TuskConfiguration, TuskData};
use tusk_core::error::Error as TuskError;
use tusk_core::resources::User;
use tusk_derive::rest_resource;
use crate::error::{HttpError, HttpIfError, HttpOkOr, HttpResult, WrapResult};
use crate::api::directory::{DirectoryPath, DirectoryItemRead, DirectoryItemCreate, FileCreate, FolderCreate};
use crate::api::session::{SessionCreate, SessionRead};

/// Represents the `/session` REST resource.
///
/// The `/session` resource is responsible for authenticating users and keeping user sessions.
pub struct SessionResource;
#[rest_resource("/session")]
impl SessionResource {
    async fn get(session: Session) -> HttpResult {
        let session: SessionRead = session.try_into()
            .or_unauthorized()?;

        HttpResponse::Ok()
            .json(session)
            .wrap_ok()
    }

    async fn post(tusk: TuskData, session: Session, web::Form(session_create): web::Form<SessionCreate>) -> HttpResult {
        let mut db_connection = tusk.database_connect()
            .or_internal_server_error()?;

        let user = User::read_by_username(&mut db_connection, session_create.username())
            .map_err(|e| match e {
                TuskError::DatabaseQueryError(tusk_core::error::DieselQueryError::NotFound) => {
                    // TODO: fake attempt login
                    log::warn!("Failed login attempt for user `{}`", session_create.username());
                    HttpError::unauthorized()
                },
                e => HttpError::internal_server_error().error(e)
            })?;

        if !user.verify_password(session_create.password()) {
            log::warn!("Failed login attempt for user `{}`", session_create.username());
            return Err(HttpError::unauthorized());
        }

        session.renew();
        session.insert("username", session_create.username())
            .or_internal_server_error()
            .with_log_error()?;
        log::info!("User {} logged in", session_create.username());

        HttpResponse::Created()
            .finish()
            .wrap_ok()
    }

    async fn delete(session: Session) -> impl Responder {
        session.clear();
        session.purge();

        HttpResponse::Ok().finish()
    }
}

/// Represents the `/directory` REST resource.
///
/// The `/directory` resource is responsible for creating, downloading, uploading or deleting
/// files or folders in the user's directory.
pub struct DirectoryResource;
#[rest_resource("/directory/{filename:.*}")]
impl DirectoryResource {
    async fn get(tusk: TuskData, session: Session, req: HttpRequest) -> HttpResult {
        let session: SessionRead = session.try_into()?;
        let path = req.match_info()
            .query("filename");
        let path = DirectoryPath::with_root(tusk.user_directories(), path)?;
        path.authorize_for(session.username())?;

        if path.is_directory() {
            let children = path.list_children()?;

            HttpResponse::Ok()
                .json(children)
                .wrap_ok()
        } else {
            NamedFile::open(path)
                .or_internal_server_error()?
                .into_response(&req)
                .wrap_ok()
        }
    }

    async fn delete(tusk: TuskData, session: Session, req: HttpRequest) -> HttpResult {
        let session: SessionRead = session.try_into()?;
        let path = req.match_info()
            .query("filename");
        let path = DirectoryPath::with_root(tusk.user_directories(), path)?;
        path.authorize_for(session.username())?;

        path.delete()?;

        HttpResponse::Ok()
            .finish()
            .wrap_ok()
    }

    async fn post(tusk: TuskData, session: Session, data: MultipartForm<DirectoryItemCreate>, req: HttpRequest) -> HttpResult {
        let session: SessionRead = session.try_into()?;
        let path = req.match_info()
            .query("filename");
        let mut path = DirectoryPath::with_root(tusk.user_directories(), path)?;
        path.authorize_for(session.username())?;

        let data = data.into_inner();

        if data.is_folder() {
            let folder_data: FolderCreate = data.try_into()?;

            if folder_data.name().contains(|c| c == '\\' || c == '/') { return Err(HttpError::bad_request()); }
            path.push(&folder_data.name());

            path.create()?;

            let attr: DirectoryItemRead = path.try_into()?;
            HttpResponse::Created()
                .insert_header((header::LOCATION, ""))
                .json(attr)
                .wrap_ok()
        } else if data.is_file() {
            let file_data: FileCreate = data.try_into()?;

            {
                let filename = match &file_data.payload().file_name {
                    Some(name) if !name.contains(|c| c == '\\' || c == '/') => { name },
                    _ => return Err(HttpError::bad_request())
                };
                path.push(filename);
            }

            file_data.into_payload().file.persist(&path)
                .or_internal_server_error()
                .with_log_error()?;


            let attr: DirectoryItemRead = path.try_into()?;
            HttpResponse::Created()
                .insert_header((header::LOCATION, ""))
                .json(attr)
                .wrap_ok()
        } else {
            HttpResponse::BadRequest()
                .finish()
                .wrap_ok()
        }
    }
}

/// Configures the server by adding the corresponding API resources.
pub fn configure(cfg: &mut ServiceConfig, _tusk: &TuskConfiguration) {
    cfg.service(SessionResource)
        .service(DirectoryResource);
    // TODO...
}

#[cfg(test)]
mod test {
    // TODO: Add tests for SessionResource.
    // TODO: Add tests for DirectoryResource.
}