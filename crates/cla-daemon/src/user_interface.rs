//! UserInterface D-Bus implementation.
//!
//! Implements the `com.redhat.lightspeed.user` D-Bus interface.

use uuid::Uuid;
use zbus::fdo;
use tracing::info;

use cla_common::system;

// ---------------------------------------------------------------------------
// UserSessionManager – shared helper used by ChatInterface too
// ---------------------------------------------------------------------------

/// Derives deterministic user identifiers from the machine-id and the
/// caller's effective UID.
pub struct UserSessionManager {
    machine_id: Uuid,
}

impl UserSessionManager {
    pub fn new() -> Self {
        let raw = system::read_machine_id();
        let machine_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, raw.as_bytes());
        Self { machine_id }
    }

    /// The UUID derived from `/etc/machine-id`.
    #[allow(dead_code)]
    pub fn machine_id(&self) -> Uuid {
        self.machine_id
    }

    /// Derive a deterministic user UUID from the machine-id and `euid`.
    pub fn get_user_id(&self, euid: u32) -> String {
        Uuid::new_v5(&self.machine_id, euid.to_string().as_bytes()).to_string()
    }
}

// ---------------------------------------------------------------------------
// D-Bus interface
// ---------------------------------------------------------------------------

/// Stateful handle behind the `com.redhat.lightspeed.user` D-Bus interface.
pub struct UserInterface {
    session_manager: UserSessionManager,
}

impl UserInterface {
    pub fn new() -> Self {
        Self {
            session_manager: UserSessionManager::new(),
        }
    }
}

#[zbus::interface(name = "com.redhat.lightspeed.user")]
impl UserInterface {
    /// Return a deterministic user UUID derived from the machine-id and the
    /// given effective UID.
    async fn get_user_id(&self, effective_user_id: u32) -> fdo::Result<String> {
        info!("GetUserId for euid={}", effective_user_id);
        Ok(self.session_manager.get_user_id(effective_user_id))
    }
}
