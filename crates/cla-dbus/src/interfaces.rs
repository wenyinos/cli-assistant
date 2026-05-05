//! Trait-based D-Bus interface definitions.
//!
//! Each trait corresponds to a D-Bus interface under the `com.redhat.lightspeed`
//! namespace. The daemon crate implements these traits on concrete service structs
//! and exposes them via `#[zbus::interface]`.
//!
//! # Method naming
//!
//! Trait methods use `snake_case` (idiomatic Rust). When mapped to D-Bus via
//! `#[zbus::interface]`, zbus automatically converts them to `PascalCase` to
//! match the Python method names (e.g. `get_all_chat_from_user` →
//! `GetAllChatFromUser`).
//!
//! # Example (daemon side)
//!
//! ```rust,ignore
//! use cla_dbus::interfaces::Chat;
//! use cla_dbus::structures::*;
//! use cla_dbus::ClaDbusError;
//!
//! struct ChatService { /* ... */ }
//!
//! impl Chat for ChatService {
//!     async fn get_all_chat_from_user(&self, user_id: &str) -> Result<ChatList, ClaDbusError> {
//!         // implementation
//! #       todo!()
//!     }
//!     // ...
//! }
//!
//! #[zbus::interface(name = "com.redhat.lightspeed.chat")]
//! impl ChatService {
//!     // Delegate to the trait methods or implement directly.
//! }
//! ```

use crate::exceptions::ClaDbusError;
use crate::structures::{
    ChatList, HistoryList, Question, Response,
};

// ===========================================================================
// Chat interface  —  com.redhat.lightspeed.chat
// ===========================================================================

/// D-Bus interface for chat session management.
///
/// Provides CRUD operations on chat sessions and the primary `ask_question`
/// entry-point for interacting with the assistant.
pub trait Chat {
    /// List every chat that belongs to `user_id`.
    fn get_all_chat_from_user(
        &self,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<ChatList, ClaDbusError>> + Send;

    /// Delete **all** chats for `user_id`.
    fn delete_all_chat_for_user(
        &self,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<(), ClaDbusError>> + Send;

    /// Delete a single chat identified by `name` for `user_id`.
    fn delete_chat_for_user(
        &self,
        user_id: &str,
        name: &str,
    ) -> impl std::future::Future<Output = Result<(), ClaDbusError>> + Send;

    /// Return the name of the most recently created chat for `user_id`.
    fn get_latest_chat_from_user(
        &self,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<String, ClaDbusError>> + Send;

    /// Check whether a chat named `name` exists for `user_id`.
    fn is_chat_available(
        &self,
        user_id: &str,
        name: &str,
    ) -> impl std::future::Future<Output = Result<bool, ClaDbusError>> + Send;

    /// Return the UUID of the chat named `name` for `user_id`.
    fn get_chat_id(
        &self,
        user_id: &str,
        name: &str,
    ) -> impl std::future::Future<Output = Result<String, ClaDbusError>> + Send;

    /// Create a new chat and return its UUID.
    fn create_chat(
        &self,
        user_id: &str,
        name: &str,
        description: &str,
    ) -> impl std::future::Future<Output = Result<String, ClaDbusError>> + Send;

    /// Pose a question to the assistant and return the response.
    fn ask_question(
        &self,
        user_id: &str,
        message_input: Question,
    ) -> impl std::future::Future<Output = Result<Response, ClaDbusError>> + Send;
}

// ===========================================================================
// History interface  —  com.redhat.lightspeed.history
// ===========================================================================

/// D-Bus interface for conversation history operations.
///
/// Provides read, filter, and write access to the history of questions and
/// responses stored per user / chat.
pub trait History {
    /// Retrieve the full conversation history for `user_id`.
    fn get_history(
        &self,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<HistoryList, ClaDbusError>> + Send;

    /// Retrieve the first conversation in a given chat.
    fn get_first_conversation(
        &self,
        user_id: &str,
        from_chat: &str,
    ) -> impl std::future::Future<Output = Result<HistoryList, ClaDbusError>> + Send;

    /// Retrieve the most recent conversation in a given chat.
    fn get_last_conversation(
        &self,
        user_id: &str,
        from_chat: &str,
    ) -> impl std::future::Future<Output = Result<HistoryList, ClaDbusError>> + Send;

    /// Retrieve conversations matching `filter` within a given chat.
    fn get_filtered_conversation(
        &self,
        user_id: &str,
        filter: &str,
        from_chat: &str,
    ) -> impl std::future::Future<Output = Result<HistoryList, ClaDbusError>> + Send;

    /// Erase **all** history for `user_id`.
    fn clear_all_history(
        &self,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<(), ClaDbusError>> + Send;

    /// Erase history for a specific chat.
    fn clear_history(
        &self,
        user_id: &str,
        from_chat: &str,
    ) -> impl std::future::Future<Output = Result<(), ClaDbusError>> + Send;

    /// Persist a new history entry (question + response) for a chat.
    fn write_history(
        &self,
        chat_id: &str,
        user_id: &str,
        question: &str,
        response: &str,
    ) -> impl std::future::Future<Output = Result<(), ClaDbusError>> + Send;
}

// ===========================================================================
// User interface  —  com.redhat.lightspeed.user
// ===========================================================================

/// D-Bus interface for user identity operations.
pub trait User {
    /// Map an effective OS user id to the internal user identifier used by the
    /// assistant.
    fn get_user_id(
        &self,
        effective_user_id: u32,
    ) -> impl std::future::Future<Output = Result<String, ClaDbusError>> + Send;
}
