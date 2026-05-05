//! clad – D-Bus daemon for the Command Line Assistant.

mod chat_interface;
mod database;
mod dbus_server;
mod history;
mod history_interface;
mod http;
mod user_interface;

use std::sync::Arc;

use tracing::info;
use tracing_subscriber::EnvFilter;

use cla_common::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise structured logging from RUST_LOG (default: info).
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("clad {} starting", cla_common::VERSION);

    let config = Config::load().map_err(|e| {
        tracing::error!("Failed to load configuration: {}", e);
        e
    })?;
    info!("Configuration loaded successfully");

    let config = Arc::new(config);

    // Enter the D-Bus event loop (blocks until the bus goes away).
    dbus_server::serve(config).await
}
