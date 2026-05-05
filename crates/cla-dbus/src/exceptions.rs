//! D-Bus specific error types for the Command Line Assistant.
//!
//! Each variant maps to a distinct D-Bus error name under the
//! `com.redhat.lightspeed` namespace, mirroring the Python exception hierarchy.

use thiserror::Error;
use zbus::fdo::Error as FdoError;

// ---------------------------------------------------------------------------
// Error enum
// ---------------------------------------------------------------------------

/// Unified error type for all `cla-dbus` interface operations.
///
/// Conversions to [`zbus::fdo::Error`] are provided so that trait
/// implementations can return `Result<T, ClaDbusError>` directly from
/// `#[zbus::interface]` methods.
#[derive(Debug, Error)]
pub enum ClaDbusError {
    /// A generic request failure.
    #[error("request failed: {0}")]
    RequestFailed(String),

    /// The on-disk history file is corrupt or unreadable.
    #[error("corrupted history: {0}")]
    CorruptedHistory(String),

    /// The expected history file does not exist.
    #[error("missing history file: {0}")]
    MissingHistoryFile(String),

    /// History data is not available for the requested context.
    #[error("history not available: {0}")]
    HistoryNotAvailable(String),

    /// History recording is disabled in the current configuration.
    #[error("history not enabled")]
    HistoryNotEnabled,

    /// The requested chat session was not found.
    #[error("chat not found: {0}")]
    ChatNotFound(String),
}

// ---------------------------------------------------------------------------
// D-Bus error name mapping
// ---------------------------------------------------------------------------

impl ClaDbusError {
    /// Return the fully-qualified D-Bus error name for this variant.
    ///
    /// These names follow the `com.redhat.lightspeed.*` convention used by
    /// the Python implementation.
    pub fn error_name(&self) -> &'static str {
        match self {
            Self::RequestFailed(_) => "com.redhat.lightspeed.RequestFailed",
            Self::CorruptedHistory(_) => "com.redhat.lightspeed.CorruptedHistory",
            Self::MissingHistoryFile(_) => "com.redhat.lightspeed.MissingHistoryFile",
            Self::HistoryNotAvailable(_) => "com.redhat.lightspeed.HistoryNotAvailable",
            Self::HistoryNotEnabled => "com.redhat.lightspeed.HistoryNotEnabled",
            Self::ChatNotFound(_) => "com.redhat.lightspeed.ChatNotFound",
        }
    }
}

// ---------------------------------------------------------------------------
// Conversion → zbus::fdo::Error
// ---------------------------------------------------------------------------

impl From<ClaDbusError> for FdoError {
    fn from(err: ClaDbusError) -> Self {
        // Map to the standard D-Bus `Failed` error with a descriptive message.
        // Callers that need the exact error name can use [`ClaDbusError::error_name`].
        FdoError::Failed(format!("[{}] {}", err.error_name(), err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_name_round_trip() {
        let cases: Vec<(ClaDbusError, &str)> = vec![
            (
                ClaDbusError::RequestFailed("oops".into()),
                "com.redhat.lightspeed.RequestFailed",
            ),
            (
                ClaDbusError::CorruptedHistory("bad file".into()),
                "com.redhat.lightspeed.CorruptedHistory",
            ),
            (
                ClaDbusError::MissingHistoryFile("/tmp/h.json".into()),
                "com.redhat.lightspeed.MissingHistoryFile",
            ),
            (
                ClaDbusError::HistoryNotAvailable("no data".into()),
                "com.redhat.lightspeed.HistoryNotAvailable",
            ),
            (
                ClaDbusError::HistoryNotEnabled,
                "com.redhat.lightspeed.HistoryNotEnabled",
            ),
            (
                ClaDbusError::ChatNotFound("abc-123".into()),
                "com.redhat.lightspeed.ChatNotFound",
            ),
        ];

        for (err, expected_name) in cases {
            assert_eq!(err.error_name(), expected_name);
            // Ensure conversion to fdo::Error does not panic.
            let _fdo: FdoError = err.into();
        }
    }
}
