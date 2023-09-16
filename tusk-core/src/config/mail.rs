use lettre::SmtpTransport;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use crate::error::TuskResult;

/// Represents the `mail` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct Mail {
    hostname: String,
    port: u16,
    auth_user: String,
    auth_password: Secret<String>
}
impl Mail {
    pub fn mailer(&self) -> TuskResult<SmtpTransport> {
        log::debug!("Loading Mailer section");

        let mailer = SmtpTransport::starttls_relay(&self.hostname)?
            .port(self.port)
            .credentials(Credentials::new(self.auth_user.clone(), self.auth_password.expose_secret().to_owned()))
            .authentication(vec![Mechanism::Plain])
            .build();
        Ok(mailer)
    }
}