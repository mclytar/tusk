use std::fs::File;
use std::io::BufReader;
use serde::Deserialize;
use crate::error::{TuskError, TuskResult};

/// Represents the `ssl` section of the `tusk.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct Ssl {
    cert_file: String,
    key_file: String
}
impl Ssl {
    /// Converts this section into a TLS server configuration.
    pub fn into_server_configuration(self) -> TuskResult<rustls::ServerConfig> {
        log::debug!("Loading Ssl section");

        let file = File::open(&self.cert_file)?;
        let mut reader = BufReader::new(file);
        let certs: Vec<_> = rustls_pemfile::certs(&mut reader)?
            .into_iter()
            .map(rustls::Certificate)
            .collect();

        if certs.len() == 0 {
            log::error!("No certificate found.");
            return Err(TuskError::CertificatesNotFound);
        }
        log::info!("Found {} certificates.", certs.len());

        let file = File::open(&self.key_file)?;
        let mut reader = BufReader::new(file);
        let keys: Vec<_> = rustls_pemfile::pkcs8_private_keys(&mut reader)?
            .into_iter()
            .map(rustls::PrivateKey)
            .collect();

        if keys.len() > 0 { log::info!("Found {} keys, using the first one available.", keys.len()) };

        let key = keys.into_iter()
            .next()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, format!("No key in file '{}'.", self.key_file)))?;

        log::info!("Key file loaded");

        let config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        Ok(config)
    }
}