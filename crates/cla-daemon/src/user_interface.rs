//! UserInterface D-Bus implementation.
//!
//! Implements the `com.redhat.lightspeed.user` D-Bus interface.

use tracing::info;
use zbus::fdo;
use zbus::message::Header;
use zbus::Connection;

use cla_common::UserSessionManager;

use crate::authorization;

// ---------------------------------------------------------------------------
// D-Bus interface
// ---------------------------------------------------------------------------

/// Stateful handle behind the `com.redhat.lightspeed.user` D-Bus interface.
pub struct UserInterface {
    session_manager: UserSessionManager,
}

impl UserInterface {
    pub fn new() -> anyhow::Result<Self> {
        let session_manager = UserSessionManager::new()
            .map_err(|e| anyhow::anyhow!("failed to initialize session manager: {}", e))?;
        Ok(Self { session_manager })
    }
}

#[zbus::interface(name = "com.redhat.lightspeed.user")]
impl UserInterface {
    /// Return a deterministic user UUID derived from the machine-id and the
    /// given effective UID.
    async fn get_user_id(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        effective_user_id: u32,
    ) -> fdo::Result<String> {
        authorization::authorize_unix_user(conn, &header, effective_user_id).await?;
        info!("GetUserId for euid={}", effective_user_id);
        Ok(self.session_manager.get_user_id(effective_user_id))
    }
}
