use actix_session::Session;
use actix_web::{HttpResponse, Responder};
use actix_web::web::{self, ServiceConfig};
use log::{error, info};
use serde::{Serialize, Deserialize};
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

    async fn post(session: Session, form: web::Form<SessionFormData>) -> impl Responder {
        let form = form.into_inner();
        if &form.username.to_lowercase() != "dummy" || &form.password != "dummy" {
            info!("Failed login attempty for user `dummy`");
            return HttpResponse::Unauthorized().finish();
        }

        session.renew();
        if let Err(e) = session.insert("username", "dummy") {
            error!("{e}");
            return HttpResponse::InternalServerError().finish();
        }

        log::info!("User {} logged in", form.username);

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