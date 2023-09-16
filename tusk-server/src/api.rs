//! Resources in the REST API.
//!
//! Contains the structures relative to the REST resources handled by the API.
//! Every structure is relative to a single resource and contains the HTTP methods to handle it.
//! Helper serializable/deserializable structures are contained in the respective modules, and they
//! address the relative CRUD methods.

pub mod session;
pub mod storage;
pub mod account;

use actix_web::web::ServiceConfig;
use crate::api::account::AccountPasswordResource;
use crate::api::storage::StorageResource;
use crate::api::session::SessionResource;

/// Configures the server by adding the corresponding API resources.
pub fn configure(cfg: &mut ServiceConfig) {
    cfg
        .service(AccountPasswordResource)
        .service(SessionResource)
        .service(StorageResource)
    ;
}