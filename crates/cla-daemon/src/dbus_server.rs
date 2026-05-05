//! D-Bus server setup – publishes the three daemon interfaces on the system bus.

use std::sync::Arc;

use tracing::info;
use zbus::connection::Builder;

use cla_common::Config;
use cla_dbus::constants::{
    CHAT_IDENTIFIER, CHAT_OBJECT_PATH, HISTORY_OBJECT_PATH, USER_OBJECT_PATH,
};

use crate::chat_interface::ChatInterface;
use crate::history_interface::HistoryInterface;
use crate::user_interface::UserInterface;

/// Set up and serve the three daemon interfaces on the system bus.
///
/// This function **blocks** (via `std::future::pending`) until the owning
/// connection is torn down by the bus.
pub async fn serve(config: Arc<Config>) -> anyhow::Result<()> {
    info!("Publishing D-Bus interfaces on the system bus");

    let chat = ChatInterface::new(config.clone()).await?;
    let history = HistoryInterface::new(config.clone()).await?;
    let user = UserInterface::new();

    let _conn = Builder::system()?
        .name(CHAT_IDENTIFIER)?
        .serve_at(CHAT_OBJECT_PATH, chat)?
        .serve_at(HISTORY_OBJECT_PATH, history)?
        .serve_at(USER_OBJECT_PATH, user)?
        .build()
        .await?;

    info!("D-Bus server running – waiting for requests");

    // Park the task forever; the D-Bus connection owns the interfaces.
    std::future::pending::<()>().await;

    #[allow(unreachable_code)]
    Ok(())
}
