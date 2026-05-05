//! Reqwest-based HTTP client with mTLS and proxy support.

use std::time::Duration;

use reqwest::Client;
use tracing::debug;

use cla_common::{ClaError, Config, VERSION};

/// Build a [`reqwest::Client`] from the daemon configuration.
///
/// Configures:
/// - request timeout from `config.backend.timeout`
/// - `User-Agent: clad/<version>` header
/// - optional mTLS via `cert_file` + `key_file`
/// - optional HTTP proxies
/// - rustls TLS backend (via Cargo feature)
pub fn create_client(config: &Config) -> Result<Client, ClaError> {
    let timeout = Duration::from_secs(config.backend.timeout);

    let mut builder = Client::builder()
        .timeout(timeout)
        .user_agent(format!("clad/{}", VERSION));

    // mTLS client certificate
    let cert_path = &config.backend.auth.cert_file;
    let key_path = &config.backend.auth.key_file;

    if cert_path.exists() && key_path.exists() {
        let cert_pem = std::fs::read(cert_path)
            .map_err(|e| ClaError::config_with_source("failed to read cert file", e))?;
        let key_pem = std::fs::read(key_path)
            .map_err(|e| ClaError::config_with_source("failed to read key file", e))?;

        // reqwest expects a concatenated PEM (cert + key).
        let mut combined = cert_pem;
        combined.extend_from_slice(&key_pem);

        let identity = reqwest::Identity::from_pem(&combined)
            .map_err(|e| ClaError::config_with_source("failed to build TLS identity", e))?;
        builder = builder.identity(identity);
    }

    // HTTP proxies
    for (proto, proxy_url) in &config.backend.proxies {
        let proxy = reqwest::Proxy::all(proxy_url)
            .map_err(|e| ClaError::config_with_source(format!("invalid proxy URL '{}'", proto), e))?;
        builder = builder.proxy(proxy);
    }

    debug!("Creating HTTP client (timeout={}s)", config.backend.timeout);

    builder
        .build()
        .map_err(|e| ClaError::config_with_source("failed to create HTTP client", e))
}
