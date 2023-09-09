//! Resources in the REST API.
//!
//! Contains the structures relative to the REST resources handled by the API.
//! Every structure is relative to a single resource and contains the HTTP methods to handle it.
//! Helper serializable/deserializable structures are contained in the respective modules, and they
//! address the relative CRUD methods.

pub mod session;
pub mod storage;
pub mod users;

use actix_web::web::{ServiceConfig};
use tusk_core::config::{TuskConfiguration};
use crate::api::storage::StorageResource;
use crate::api::session::{SessionResource};
use crate::api::users::UserResource;

/// Configures the server by adding the corresponding API resources.
pub fn configure(cfg: &mut ServiceConfig, _tusk: &TuskConfiguration) {
    cfg.service(StorageResource)
        .service(SessionResource)
        .service(UserResource);
    // TODO...
}