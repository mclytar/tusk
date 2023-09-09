//! Contains the CRUD structures relative to the `/users` REST resource.

use actix_session::Session;
use actix_web::{web, HttpResponse};
use secrecy::Secret;
use serde::Deserialize;
use tusk_core::config::{TuskData};
use tusk_core::resources::User;
use tusk_derive::rest_resource;
use crate::error::{HttpError, HttpIfError, HttpOkOr, HttpResult, WrapResult};
use crate::api::session::{SessionRead};

/// Represents the CRUD (partial) **Update** structure relative to the `/users/{username}` REST resource.
#[derive(Clone, Debug, Deserialize)]
pub struct UserPatchData {
    username: Option<String>,
    password: Option<Secret<String>>,
    proof: Option<Secret<String>>
}
impl UserPatchData {
    /// Returns true if only the user itself is allowed to perform the requested changes.
    pub fn only_owner(&self) -> bool {
        self.username.is_some() || self.password.is_some()
    }
    /// Returns the new username of the user, if any.
    pub fn username(&self) -> Option<&str> {
        self.username.as_ref()
            .map(|s| s.as_str())
    }
    /// Returns the new password of the user, if any.
    pub fn password(&self) -> Option<&Secret<String>> {
        self.password.as_ref()
    }
    /// Returns a proof that the original user is requesting the changes.
    ///
    /// The proof consists of the current password of the user.
    pub fn proof(&self) -> Option<&Secret<String>> {
        self.proof.as_ref()
    }
    /// Applies the changes, or reports the error if any.
    pub fn apply<I: AsRef<str>, U: AsRef<str>>(&self, db_connection: &mut tusk_core::PgConnection, initiator: I, user: U) -> HttpResult {
        let initiator = initiator.as_ref();
        let user = user.as_ref();

        let mut user = User::read_by_username(db_connection, user)
            .map_err(|e| HttpError::from(e))
            .with_authentication_failure(user, "fake password")?;

        if self.only_owner() {
            if initiator != user.username() {
                return HttpError::forbidden()
                    .wrap_err();
            }

            if let Some(proof) = &self.proof {
                if !user.verify_password(proof) {
                    log::warn!("Failed authentication attempt for user `{}`", user.username());
                    return Err(HttpError::unauthorized());
                }
            } else {
                return HttpError::unauthorized()
                    .wrap_err();
            }
        }

        if let Some(username) = &self.username {
            user = user.update_username(db_connection, username)?;
        }

        if let Some(password) = &self.password {
            user.update_password(db_connection, password)?;
        }

        HttpResponse::Ok()
            .finish()
            .wrap_ok()
    }
}

/// Represents the `/users` REST resource.
///
/// The `/users` resource is used for user management.
pub struct UserCollectionResource;
impl UserCollectionResource {

}

/// Represents the `/users/{username}` REST resource.
///
/// The `/users/{username}` resource is used for management of a specific user.
pub struct UserResource;
#[rest_resource("/users/{username}")]
impl UserResource {
    async fn patch(tusk: TuskData, session: Session, web::Json(data): web::Json<UserPatchData>, resource_path: web::Path<String>) -> HttpResult {
        let session_data: SessionRead = session.clone().try_into()?;
        let username = resource_path.into_inner();

        let mut db_connection = tusk.database_connect()
            .or_internal_server_error()?;

        let initiator_roles = tusk_core::resources::Role::read_by_user_username(&mut db_connection, session_data.username())
            .or_internal_server_error()
            .with_log_error()?;

        if session_data.username() != username && !initiator_roles.iter().any(|r| r.name() == "admin") {
            return HttpError::forbidden()
                .wrap_err();
        }

        let response = data.apply(&mut db_connection, session_data.username(), username)?;

        if data.only_owner() {
            session.clear();
            session.purge();
        }

        Ok(response)
    }
}

#[cfg(test)]
mod test {
    use actix_web::{FromRequest, ResponseError, web};
    use actix_web::http::{StatusCode};
    use actix_web::test::TestRequest;
    use secrecy::Secret;
    use tusk_core::config::TEST_CONFIGURATION;
    use crate::api::session::SessionRead;
    use crate::api::session::test::create_session_for_user;
    use crate::api::users::{UserPatchData, UserResource};

    #[actix_web::test]
    async fn changing_username_or_password_requires_proof() {
        let tusk = TEST_CONFIGURATION.to_data();
        let session = create_session_for_user("dummy").await;
        let req = TestRequest::patch()
            .param("username", "dummy")
            .to_http_request();

        let data = web::Json(UserPatchData {
            username: Some(String::from("not_dummy")),
            password: None,
            proof: None
        });

        let resp = UserResource::patch(tusk.clone(), session.clone(), data, web::Path::extract(&req).await.unwrap()).await
            .expect_err("UNAUTHORIZED");
        assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);

        let data = web::Json(UserPatchData {
            username: None,
            password: Some(Secret::from("abc".to_owned())),
            proof: None
        });

        let resp = UserResource::patch(tusk, session, data, web::Path::extract(&req).await.unwrap()).await
            .expect_err("UNAUTHORIZED");
        assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn changing_username_or_password_requires_valid_proof() {
        let tusk = TEST_CONFIGURATION.to_data();
        let session = create_session_for_user("dummy").await;
        let req = TestRequest::patch()
            .param("username", "dummy")
            .to_http_request();

        let data = web::Json(UserPatchData {
            username: Some(String::from("not_dummy")),
            password: None,
            proof: Some(Secret::from("not_the_real_password".to_owned()))
        });

        let resp = UserResource::patch(tusk.clone(), session.clone(), data, web::Path::extract(&req).await.unwrap()).await
            .expect_err("UNAUTHORIZED");
        assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);

        let data = web::Json(UserPatchData {
            username: None,
            password: Some(Secret::from("abc".to_owned())),
            proof: Some(Secret::from("not_the_real_password".to_owned()))
        });

        let resp = UserResource::patch(tusk, session, data, web::Path::extract(&req).await.unwrap()).await
            .expect_err("UNAUTHORIZED");
        assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn cannot_change_other_user_username_or_password() {
        let tusk = TEST_CONFIGURATION.to_data();
        let session = create_session_for_user("admin").await;
        let req = TestRequest::patch()
            .param("username", "dummy")
            .to_http_request();

        let data = web::Json(UserPatchData {
            username: Some(String::from("not_dummy")),
            password: None,
            proof: Some(Secret::from("not_the_real_password".to_owned()))
        });

        let resp = UserResource::patch(tusk.clone(), session.clone(), data, web::Path::extract(&req).await.unwrap()).await
            .expect_err("FORBIDDEN");
        assert_eq!(resp.status_code(), StatusCode::FORBIDDEN);

        let data = web::Json(UserPatchData {
            username: None,
            password: Some(Secret::from("abc".to_owned())),
            proof: Some(Secret::from("not_the_real_password".to_owned()))
        });

        let resp = UserResource::patch(tusk, session, data, web::Path::extract(&req).await.unwrap()).await
            .expect_err("FORBIDDEN");
        assert_eq!(resp.status_code(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn can_change_username_or_password() {
        let tusk = TEST_CONFIGURATION.to_data();
        let session = create_session_for_user("dummy").await;
        let req = TestRequest::patch()
            .param("username", "dummy")
            .to_http_request();

        // Change username.
        let data = web::Json(UserPatchData {
            username: Some(String::from("dummy_2")),
            password: None,
            proof: Some(Secret::from("dummy".to_owned()))
        });

        let resp = UserResource::patch(tusk.clone(), session.clone(), data, web::Path::extract(&req).await.unwrap()).await
            .expect("OK");
        assert_eq!(resp.status(), StatusCode::OK);

        // Changing username logs the user out.
        SessionRead::try_from(session)
            .expect_err("User should be logged out");

        // Change password.
        let session = create_session_for_user("dummy_2").await;
        let req = TestRequest::patch()
            .param("username", "dummy_2")
            .to_http_request();
        let data = web::Json(UserPatchData {
            username: None,
            password: Some(Secret::from("password".to_owned())),
            proof: Some(Secret::from("dummy".to_owned()))
        });

        let resp = UserResource::patch(tusk.clone(), session.clone(), data, web::Path::extract(&req).await.unwrap()).await
            .expect("OK");
        assert_eq!(resp.status(), StatusCode::OK);

        // Changing password logs the user out.
        SessionRead::try_from(session)
            .expect_err("User should be logged out");

        // Restore old username and password.
        let mut db_connection = tusk.database_connect()
            .expect("Database connection");
        let resp = UserPatchData {
            username: Some(String::from("dummy")),
            password: Some(Secret::from("dummy".to_owned())),
            proof: Some(Secret::from("password".to_owned()))
        }.apply(&mut db_connection, "dummy_2", "dummy_2")
            .expect("User's username back to `dummy`.");
        assert_eq!(resp.status(), StatusCode::OK);
    }
}