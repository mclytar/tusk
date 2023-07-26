use actix_session::Session;
use actix_web::{HttpResponse, Responder};
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

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(SessionResource);
    // TODO...
}