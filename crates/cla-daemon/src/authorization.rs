//! D-Bus caller authorization helpers.
//!
//! Every interface method must verify that the caller can only access the
//! user identity it is requesting. If caller information cannot be resolved,
//! the request is denied (fail closed).

use cla_common::UserSessionManager;
use zbus::fdo;
use zbus::message::Header;
use zbus::names::BusName;
use zbus::Connection;

/// Return the Unix user ID for the D-Bus sender on the given connection.
pub async fn caller_unix_user_id(conn: &Connection, header: &Header<'_>) -> fdo::Result<u32> {
    let sender = header
        .sender()
        .ok_or_else(|| fdo::Error::AccessDenied("missing D-Bus sender".to_string()))?;
    let bus_name = BusName::from(sender.clone());

    let dbus = zbus::fdo::DBusProxy::new(conn)
        .await
        .map_err(|e| fdo::Error::AccessDenied(format!("failed to query D-Bus: {e}")))?;
    dbus.get_connection_unix_user(bus_name)
        .await
        .map_err(|e| fdo::Error::AccessDenied(format!("failed to query caller user: {e}")))
}

/// Verify that the caller's Unix user ID matches `requested_euid`.
pub async fn authorize_unix_user(
    conn: &Connection,
    header: &Header<'_>,
    requested_euid: u32,
) -> fdo::Result<()> {
    let caller = caller_unix_user_id(conn, header).await?;
    verify_unix_user(caller, requested_euid)
}

/// Verify that the caller's Unix user ID maps to `requested_user_id`.
pub async fn authorize_internal_user(
    conn: &Connection,
    header: &Header<'_>,
    requested_user_id: &str,
    session_manager: &UserSessionManager,
) -> fdo::Result<()> {
    let caller = caller_unix_user_id(conn, header).await?;
    verify_internal_user(caller, requested_user_id, session_manager)
}

/// Pure Unix user ID equality check, kept separate for unit testing.
pub fn verify_unix_user(caller_unix_id: u32, requested_unix_id: u32) -> fdo::Result<()> {
    if caller_unix_id == requested_unix_id {
        Ok(())
    } else {
        Err(fdo::Error::AccessDenied(
            "Unix user ID mismatch: access denied".to_string(),
        ))
    }
}

/// Pure internal user ID check, kept separate for unit testing.
pub fn verify_internal_user(
    caller_unix_id: u32,
    requested_user_id: &str,
    session_manager: &UserSessionManager,
) -> fdo::Result<()> {
    let caller_internal_id = session_manager.get_user_id(caller_unix_id);
    if caller_internal_id == requested_user_id {
        Ok(())
    } else {
        Err(fdo::Error::AccessDenied(
            "User ID mismatch: access denied".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_manager() -> UserSessionManager {
        UserSessionManager::from_raw_machine_id("09e28913cb074ed995a239c93b07fd8a")
            .expect("valid machine id")
    }

    #[test]
    fn unix_user_authorization_matches() {
        assert!(verify_unix_user(1000, 1000).is_ok());
        assert!(verify_unix_user(1000, 1001).is_err());
    }

    #[test]
    fn internal_user_authorization_matches() {
        let session = session_manager();
        let user_id = session.get_user_id(1000);

        assert!(verify_internal_user(1000, &user_id, &session).is_ok());
        assert!(verify_internal_user(1001, &user_id, &session).is_err());
    }

    #[test]
    fn internal_user_authorization_rejects_wrong_id() {
        let session = session_manager();
        assert!(verify_internal_user(1000, "not-the-user", &session).is_err());
    }
}
