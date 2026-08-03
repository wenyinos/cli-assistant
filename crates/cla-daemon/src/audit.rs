//! Structured audit event helpers.
//!
//! Audit events are emitted with `target: "audit"` so journald or a JSON
//! tracing layer can route them independently from normal application logs.

/// Emit an audit event when audit logging is enabled.
pub fn event(enabled: bool, action: &str, user_id: &str, chat_id: Option<&str>) {
    if !enabled {
        return;
    }

    tracing::info!(
        target: "audit",
        audit = true,
        action = action,
        user_id = user_id,
        chat_id = ?chat_id,
    );
}
