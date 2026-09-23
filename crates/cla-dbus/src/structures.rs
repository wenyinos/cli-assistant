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
    /// Chat session (by name) whose recent history should be attached as
    /// conversation context. `None` keeps the question single-turn.
    pub context_chat: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::{
        serialized::{Context, Format},
        to_bytes_for_signature, Dict, Endian, Signature, Value,
    };

    /// Encode a `Question`-shaped `a{sv}` dict containing only the given keys,
    /// as an older client (or a single-turn call) would send it.
    fn encode_dict(entries: &[(&str, &str)]) -> zbus::zvariant::serialized::Data<'static, 'static> {
        let mut dict = Dict::new(
            Signature::from_static_str("s").unwrap(),
            Signature::from_static_str("v").unwrap(),
        );
        for (key, value) in entries {
            // `a{sv}` values are variants, so wrap the string in `Value::Value`.
            dict.append(
                Value::from(*key),
                Value::Value(Box::new(Value::from(*value))),
            )
            .expect("append");
        }
        let ctxt = Context::new(Format::DBus, Endian::Little, 0);
        to_bytes_for_signature(ctxt, "a{sv}", &dict).expect("encode")
    }

    #[test]
    fn question_without_context_chat_deserializes_to_none() {
        // Older clients omit the key entirely; the daemon must still accept it.
        let encoded = encode_dict(&[("message", "hello")]);
        let (question, _): (Question, usize) = encoded.deserialize().expect("deserialize");
        assert_eq!(question.message, "hello");
        assert!(question.context_chat.is_none());
    }

    #[test]
    fn question_with_context_chat_deserializes() {
        let encoded = encode_dict(&[("message", "hello"), ("context_chat", "default")]);
        let (question, _): (Question, usize) = encoded.deserialize().expect("deserialize");
        assert_eq!(question.context_chat.as_deref(), Some("default"));
    }

    #[test]
    fn question_roundtrips_with_context_chat() {
        let question = Question {
            message: "hi".to_string(),
            stdin: None,
            attachment: None,
            terminal: None,
            systeminfo: None,
            context_chat: Some("work".to_string()),
        };
        let ctxt = Context::new(Format::DBus, Endian::Little, 0);
        let encoded = to_bytes_for_signature(ctxt, "a{sv}", &question).expect("encode");
        let (decoded, _): (Question, usize) = encoded.deserialize().expect("deserialize");
        assert_eq!(decoded.message, "hi");
        assert_eq!(decoded.context_chat.as_deref(), Some("work"));
    }
}
