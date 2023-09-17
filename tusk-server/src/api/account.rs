//! Contains the CRUD structures relative to the `/users` REST resource.

use std::str::FromStr;
use actix_web::{HttpResponse, ResponseError};
use actix_web::http::StatusCode;
use actix_web::web::Json;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tusk_core::config::Tusk;
use tusk_core::{Connection, Message};
use tusk_core::error::{HttpOkOr, TuskError, TuskHttpResult};
use tusk_core::resources::{PasswordResetRequest, User};
use tusk_derive::rest_resource;

/// Returns a result that is `Ok` if the given password is strong enough, and `Err` otherwise.
pub fn verify_password_strength(password: &Secret<String>, user_inputs: &Vec<&str>) -> Result<(), TuskError> {
    if password.expose_secret().len() < 8 {
        return TuskError::unprocessable_entity()
            .with_text("The new password is too small.")
            .bail();
    }
    if password.expose_secret().len() > 70 {
        return TuskError::unprocessable_entity()
            .with_text("The new password is too large.")
            .bail();
    }
    if zxcvbn::zxcvbn(password.expose_secret(), &user_inputs)
        .or_internal_server_error()
        .log_error()?
        .score() < 3 {
        return TuskError::unprocessable_entity()
            .with_text("The new password is too weak.")
            .bail();
    }

    Ok(())
}

/// Defines the types of proof with which an user can prove that the patch request is legitimate.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountProofType {
    /// No proof given.
    ///
    /// This is useful when the user is applying minor changes.
    #[default] None,
    /// The proof is the (old) password of the user.
    Password,
    /// The proof is a password reset token.
    Token
}

/// Represents the (JSON) data that is sent to the server with a `PUT` request to `/account/password`.
#[derive(Clone, Debug, Deserialize)]
pub struct AccountPasswordPutData {
    email: String,
    password: Option<Secret<String>>,
    proof: Option<Secret<String>>,
    #[serde(default)]
    proof_type: AccountProofType
}
impl AccountPasswordPutData {
    /// Returns the email of the user.
    pub fn email(&self) -> &str {
        &self.email
    }
    /// Returns the new password of the user, if any.
    pub fn password(&self) -> Option<&Secret<String>> {
        self.password.as_ref()
    }
    /// Returns a proof that the original user is requesting the changes.
    ///
    /// The content of the proof depends on the specified proof type.
    pub fn proof(&self) -> Option<&Secret<String>> {
        self.proof.as_ref()
    }
    /// Returns the type of proof that the user is providing to prove that the account belongs
    /// to them.
    pub fn proof_type(&self) -> AccountProofType {
        self.proof_type
    }
}

/// Represents the `/account/password` REST resource.
///
/// The `/account/password` resource is used for requesting a password reset for
/// the specified user.
pub struct AccountPasswordResource;
#[rest_resource("/account/password")]
impl AccountPasswordResource {
    async fn put(tusk: Tusk, Json(data): Json<AccountPasswordPutData>) -> TuskHttpResult {
        let mut db = tusk.db()?;

        db.transaction(|db| {
            let initiator = match tusk.authenticate() {
                Ok(auth_session) => Some(auth_session.user(db)?),
                Err(e) if e.status_code() == StatusCode::UNAUTHORIZED => None,
                Err(e) => Err(e)?
            };
            let target = User::from_email(db, data.email())?;

            let contact_noreply = &tusk.config().email_contacts().noreply;
            let contact_support = &tusk.config().email_contacts().support;
            let server_address = tusk.config().www_domain();

            let result = match (initiator, target, data.proof_type(), data.proof(), data.password()) {
                (Some(initiator), Some(mut target), AccountProofType::Password, Some(proof), Some(password)) if initiator.id() == target.id() => {
                    // Initiator and target are the same user and the given proof is the user's password.
                    // This means that this is a legitimate and well-formed password update request.
                    if target.verify_password(proof) {
                        let user_inputs = vec![target.email(), target.display(), "Tusk"];
                        verify_password_strength(password, &user_inputs)?;
                        target.update_password(db, password)?;
                        // Send password update confirmation email.
                        let message = Message::builder()
                            .from(format!("Tusk Server <{contact_noreply}>").parse().unwrap())
                            .to(target.mailbox()?)
                            .subject("Password change")
                            .body(format!(r#"Hello, {target}!

Your password has been updated.
If you request the change, no further action is required.
If you did NOT request the change, please write immediately to {contact_support} explaining the situation.

Best,
Tusk"#))
                            .or_internal_server_error()?;
                        tusk.send_email(&message)
                            .log_error()?;
                        // Done!
                        HttpResponse::NoContent().finish()
                    } else {
                        return TuskError::unauthorized().bail();
                    }
                },
                (Some(initiator), target, AccountProofType::Password, Some(_), Some(_)) => {
                    // Although the syntax of the request is correct, the initiator and the target are NOT the same user.
                    // This means that this operation is forbidden for the initiator.
                    // Additionally, if the user exists, a forbidden password change attempt should be logged.
                    if let Some(target) = target {
                        log::warn!("User `{initiator}` tried to change password of user `{target}`");
                    }
                    return TuskError::forbidden().bail();
                },
                (None, target, AccountProofType::Password, Some(_), Some(_)) => {
                    // Although the syntax of the request is correct, the initiator is not logged in and, hence, unauthorized.
                    // Additionally, if the user exists, an unauthorized password change attempt should be logged.
                    if let Some(target) = target {
                        log::warn!("Unauthorized user tried to change password of user `{target}`");
                    }
                    return TuskError::unauthorized().bail();
                },
                (None, Some(target), AccountProofType::None, None, None) => {
                    // There is no information except for the existence of a target user, and there is no proof given.
                    // This means that this is a legitimate and well-formed password RECOVERY request.
                    let request = target.request_password_reset(db)?;
                    let token = request.token();
                    let message = Message::builder()
                        .from(format!("Tusk Server <{contact_noreply}>").parse().unwrap())
                        .to(target.mailbox()?)
                        .subject("Password reset")
                        .body(format!(r#"Hello, {target}!
A password reset request has been sent for your account.
If you requested the reset, you can set up a new password by visiting https://{server_address}/password_reset/verify?token={token} and following the steps.
If this is not the case, you can simply ignore this email.

Note: the above link expires after 24 hours. In this case, you can request a new link by visiting https://{server_address}/password_reset/request and following the steps.

Best,
Tusk"#))
                        .or_internal_server_error()?;
                    tusk.send_email(&message)
                        .log_error()?;
                    HttpResponse::Accepted().finish()
                },
                (None, None, AccountProofType::None, None, None) => {
                    // There is no information at all.
                    // This means that this may be an user that gave the wrong email address, or
                    // an user discovery attack.
                    // In any case, we do nothing, but we answer exactly as in the previous case.
                    HttpResponse::Accepted().finish()
                },
                (None, Some(mut target), AccountProofType::Token, Some(token), Some(password)) => {
                    // There is no initiator, but all the other details are there.
                    // This means that this is a legitimate and well-formed password RESET request.
                    let token = Uuid::from_str(token.expose_secret())
                        .or_bad_request()?;
                    let request = PasswordResetRequest::from_token(db, token)?;
                    let user_inputs = vec![target.email(), target.display(), "Tusk"];
                    verify_password_strength(password, &user_inputs)?;
                    target.update_password(db, password)?;
                    // Remove the reset token from the database.
                    request.delete(db)?;
                    // Send password update confirmation email.
                    let message = Message::builder()
                        .from(format!("Tusk Server <{contact_noreply}>").parse().unwrap())
                        .to(target.mailbox()?)
                        .subject("Password change")
                        .body(format!(r#"Hello, {target}!

Your password has been updated.
If you requested the change, no further action is required.
If you did NOT request the change, please write immediately to {contact_support} explaining the situation.

Best,
Tusk"#))
                        .or_internal_server_error()?;
                    tusk.send_email(&message)
                        .log_error()?;

                    HttpResponse::NoContent().finish()
                },
                _ => {
                    // Everything else can be just regarded as a `400 - BAD REQUEST` response.
                    return TuskError::bad_request().bail();
                }
            };

            // In any successful case, make the user log in again.
            tusk.log_out();

            Ok(result)
        })
    }
}

#[cfg(test)]
mod tests {
    use actix_web::http::StatusCode;
    use actix_web::ResponseError;
    use secrecy::Secret;
    use crate::api::account::verify_password_strength;

    macro_rules! secret { ($str:expr) => { Secret::from($str.to_owned()) }}

    #[test]
    fn password_verification() {
        // Too small password.
        let err = verify_password_strength(&secret!("k!7=$@"), &vec![])
            .expect_err("Password is too small");
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);

        // Too big password.
        let big_pwd = "k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?k!7=$@p?";
        let err = verify_password_strength(&secret!(big_pwd), &vec![])
            .expect_err("Password is too big");
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);

        // Too weak password.
        let err = verify_password_strength(&secret!("mario rossi"), &vec!["Mario Rossi"])
            .expect_err("Password is too weak");
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);

        // This is a quite strong password.
        verify_password_strength(&secret!("K7hQqX@#39brS"), &vec!["Mario", "Rossi", "1982", "Italy", "Tusk", "Server", "mario.rossi@example.com"])
            .expect("Valid password");
        // Relevant XKCD: https://xkcd.com/936/.
        verify_password_strength(&secret!("correct horse battery staple"), &vec!["Mario", "Rossi", "1982", "Italy", "Tusk", "Server", "mario.rossi@example.com"])
            .expect("Valid password");
        // Similar to the above, but not the same.
        verify_password_strength(&secret!("doctor parsley room tank beauty"), &vec!["Mario", "Rossi", "1982", "Italy", "Tusk", "Server", "mario.rossi@example.com"])
            .expect("Valid password");
        // From OWASP, https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html:
        // Allow usage of all characters including unicode and whitespace.
        // There should be no password composition rules limiting the type of characters permitted.
        verify_password_strength(&secret!(r#"`~^*()_+-;':",./<
>?👻(╯°□°）╯︵ ┻━┻®™¬⁆"#), &vec!["Mario", "Rossi", "1982", "Italy", "Tusk", "Server", "mario.rossi@example.com"])
            .expect("Valid password");
    }
}