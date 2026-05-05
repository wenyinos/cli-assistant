//! Data structures for D-Bus communication.
//!
//! Every type implements `Clone` and `Debug` for general use. Dictionary-shaped
//! structs derive `Type`, `SerializeDict`, and `DeserializeDict` from `zvariant`
//! so they map naturally to the D-Bus `a{sv}` (dict of variant) wire format that
//! Python's `dbus-python` produces.

use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

// ===========================================================================
// Chat structures
// ===========================================================================

/// A single chat session record.
///
/// Mirrors the Python `ChatEntry` dataclass.
/// Serialized as a D-Bus dict (`a{sv}`).
#[derive(Debug, Clone, Type, SerializeDict, DeserializeDict)]
#[zvariant(signature = "dict")]
pub struct ChatEntry {
    /// Unique chat identifier (UUID string).
    pub id: String,
    /// Human-readable chat name.
    pub name: String,
    /// Description of the chat session.
    pub description: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 last-update timestamp.
    pub updated_at: String,
    /// ISO 8601 deletion timestamp (`None` if the chat is active).
    pub deleted_at: Option<String>,
}

/// A collection of chat entries returned by list operations.
///
/// Mirrors the Python `ChatList` dataclass.
/// Serialized as a D-Bus dict (`a{sv}`).
#[derive(Debug, Clone, Type, SerializeDict, DeserializeDict)]
#[zvariant(signature = "dict")]
pub struct ChatList {
    /// The chat entries.
    pub chats: Vec<ChatEntry>,
}

// ===========================================================================
// Question / input structures
// ===========================================================================

/// Attachment payload attached to a question.
///
/// Mirrors the Python `AttachmentInput` dataclass.
/// Serialized as a D-Bus dict (`a{sv}`).
#[derive(Debug, Clone, Type, SerializeDict, DeserializeDict)]
#[zvariant(signature = "dict")]
pub struct AttachmentInput {
    /// Base64-encoded (or raw) attachment contents.
    pub contents: String,
    /// MIME type of the attachment (e.g. `text/plain`).
    pub mimetype: String,
}

/// Data piped through stdin when posing a question.
///
/// Mirrors the Python `StdinInput` dataclass.
/// Serialized as a D-Bus dict (`a{sv}`).
#[derive(Debug, Clone, Type, SerializeDict, DeserializeDict)]
#[zvariant(signature = "dict")]
pub struct StdinInput {
    /// Raw stdin content.
    pub stdin: String,
}

/// Captured terminal output included with a question.
///
/// Mirrors the Python `TerminalInput` dataclass.
/// Serialized as a D-Bus dict (`a{sv}`).
#[derive(Debug, Clone, Type, SerializeDict, DeserializeDict)]
#[zvariant(signature = "dict")]
pub struct TerminalInput {
    /// Terminal output content.
    pub output: String,
}

/// Host system metadata forwarded with a question.
///
/// Mirrors the Python `SystemInfo` dataclass.
/// Serialized as a D-Bus dict (`a{sv}`).
#[derive(Debug, Clone, Type, SerializeDict, DeserializeDict)]
#[zvariant(signature = "dict")]
pub struct SystemInfo {
    /// Operating system name (e.g. `Fedora`).
    pub os: String,
    /// OS version string.
    pub version: String,
    /// CPU architecture (e.g. `x86_64`).
    pub arch: String,
    /// Platform identifier.
    pub id: String,
}

/// A question payload sent to the assistant over D-Bus.
///
/// Mirrors the Python `Question` dataclass. All fields except `message` are
/// optional and represent different input modalities.
/// Serialized as a D-Bus dict (`a{sv}`).
#[derive(Debug, Clone, Type, SerializeDict, DeserializeDict)]
#[zvariant(signature = "dict")]
pub struct Question {
    /// The user's text message.
    pub message: String,
    /// Optional stdin input.
    pub stdin: Option<StdinInput>,
    /// Optional file attachment.
    pub attachment: Option<AttachmentInput>,
    /// Optional terminal output capture.
    pub terminal: Option<TerminalInput>,
    /// Optional host system information.
    pub systeminfo: Option<SystemInfo>,
}

/// A response payload returned by the assistant over D-Bus.
///
/// Mirrors the Python `Response` dataclass.
/// Serialized as a D-Bus dict (`a{sv}`).
#[derive(Debug, Clone, Type, SerializeDict, DeserializeDict)]
#[zvariant(signature = "dict")]
pub struct Response {
    /// The assistant's reply text.
    pub message: String,
}

// ===========================================================================
// History structures
// ===========================================================================

/// A single conversation turn stored in history.
///
/// Mirrors the Python `HistoryEntry` dataclass.
/// Serialized as a D-Bus dict (`a{sv}`).
#[derive(Debug, Clone, Type, SerializeDict, DeserializeDict)]
#[zvariant(signature = "dict")]
pub struct HistoryEntry {
    /// Unique entry identifier (UUID string).
    pub id: String,
    /// Chat ID this entry belongs to.
    pub chat_id: String,
    /// The question that was asked.
    pub question: String,
    /// The response that was received.
    pub response: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
}

/// A collection of history entries.
///
/// Mirrors the Python `HistoryList` dataclass.
/// Serialized as a D-Bus dict (`a{sv}`).
#[derive(Debug, Clone, Type, SerializeDict, DeserializeDict)]
#[zvariant(signature = "dict")]
pub struct HistoryList {
    /// The history entries.
    pub histories: Vec<HistoryEntry>,
}
