//! User session management.
//!
//! Maps to Python `daemon/session.py`. Derives a deterministic user ID from the
//! system machine-id UUID and the effective Unix user ID using UUID v5.

use std::path::Path;

use uuid::Uuid;

use crate::constants::MACHINE_ID_PATH;
use crate::errors::{ClaError, Result};

/// Manages user session identity.
///
/// Each session is uniquely identified by a UUID v5 derived from:
/// - The system machine-id (`/etc/machine-id`) parsed as a UUID namespace
/// - The effective Unix user ID
#[derive(Debug, Clone)]
pub struct UserSessionManager {
    machine_id: Uuid,
}

impl UserSessionManager {
    /// Creates a new session manager by reading the system machine-id.
    ///
    /// The machine-id file (`/etc/machine-id`) contains a 32-character hex
    /// string that is parsed directly as a UUID.
    pub fn new() -> Result<Self> {
        let machine_id = Self::read_machine_id()?;
        Ok(Self { machine_id })
    }

    /// Creates a session manager with an explicit machine-id UUID.
    ///
    /// The value is the machine-id UUID namespace used by the upstream
    /// Python implementation.
    pub fn with_machine_id(machine_id: Uuid) -> Self {
        Self { machine_id }
    }

    /// Creates a session manager from the raw machine-id string.
    ///
    /// This mirrors the upstream Python implementation: the raw machine-id
    /// string is parsed as a UUID and used as the namespace for per-user IDs.
    pub fn from_raw_machine_id(raw_machine_id: &str) -> Result<Self> {
        let machine_id = Self::parse_machine_id(raw_machine_id)?;
        Ok(Self { machine_id })
    }

    /// Returns the system machine-id as a UUID.
    pub fn machine_id(&self) -> Uuid {
        self.machine_id
    }

    /// Generates a deterministic user ID from the effective Unix user ID.
    ///
    /// Uses UUID v5 with the machine-id namespace and the euid as name.
    /// The same `(machine_id, euid)` pair always produces the same UUID.
    pub fn get_user_id(&self, effective_user_id: u32) -> String {
        Uuid::new_v5(&self.machine_id, effective_user_id.to_string().as_bytes()).to_string()
    }

    /// Reads and parses the machine-id from the system file.
    fn read_machine_id() -> Result<Uuid> {
        let path = Path::new(MACHINE_ID_PATH);
        let content = std::fs::read_to_string(path).map_err(|e| {
            ClaError::session_with_source(
                format!("failed to read machine-id from {}", MACHINE_ID_PATH),
                e,
            )
        })?;

        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(ClaError::session("machine-id file is empty"));
        }

        Self::parse_machine_id(trimmed)
    }

    /// Parse a machine-id string into a UUID.
    ///
    /// `/etc/machine-id` is normally a 32-char hex string without dashes, so
    /// normalize it to the standard UUID representation before parsing.
    fn parse_machine_id(trimmed: &str) -> Result<Uuid> {
        let uuid_str = if trimmed.len() == 32 && !trimmed.contains('-') {
            format!(
                "{}-{}-{}-{}-{}",
                &trimmed[0..8],
                &trimmed[8..12],
                &trimmed[12..16],
                &trimmed[16..20],
                &trimmed[20..32],
            )
        } else {
            trimmed.to_string()
        };

        Uuid::parse_str(&uuid_str).map_err(|e| {
            ClaError::session_with_source(format!("invalid machine-id format: {}", trimmed), e)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_user_id_is_deterministic() {
        let mid = Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap();
        let mgr = UserSessionManager::with_machine_id(mid);

        let id1 = mgr.get_user_id(1000);
        let id2 = mgr.get_user_id(1000);
        assert_eq!(id1, id2);
    }

    #[test]
    fn different_euids_produce_different_ids() {
        let mid = Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap();
        let mgr = UserSessionManager::with_machine_id(mid);

        let id_root = mgr.get_user_id(0);
        let id_user = mgr.get_user_id(1000);
        assert_ne!(id_root, id_user);
    }

    #[test]
    fn different_machine_ids_produce_different_user_ids() {
        let mid1 = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let mid2 = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

        let mgr1 = UserSessionManager::with_machine_id(mid1);
        let mgr2 = UserSessionManager::with_machine_id(mid2);

        assert_ne!(mgr1.get_user_id(1000), mgr2.get_user_id(1000));
    }

    #[test]
    fn user_id_is_valid_uuid() {
        let mid = Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap();
        let mgr = UserSessionManager::with_machine_id(mid);
        let uid = mgr.get_user_id(1000);
        assert!(Uuid::parse_str(&uid).is_ok());
    }

    #[test]
    fn user_id_matches_upstream_implementation() {
        let mgr = UserSessionManager::from_raw_machine_id("09e28913cb074ed995a239c93b07fd8a")
            .expect("valid machine id");
        assert_eq!(
            mgr.get_user_id(1000),
            "4d465f1c-0507-5dfa-9ea0-e2de1a9e90a5"
        );
    }

    #[test]
    fn machine_id_accessor() {
        let mid = Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap();
        let mgr = UserSessionManager::with_machine_id(mid);
        assert_eq!(mgr.machine_id(), mid);
    }
}
