//! HistoryInterface D-Bus implementation.
//!
//! Implements the `com.redhat.lightspeed.history` D-Bus interface.

use std::sync::Arc;

use tracing::info;
use zbus::fdo;

use cla_common::Config;
use cla_dbus::structures::{HistoryList, HistoryEntry};

use crate::history::manager::HistoryManager;

/// Stateful handle behind the `com.redhat.lightspeed.history` D-Bus interface.
pub struct HistoryInterface {
    history_manager: HistoryManager,
}

impl HistoryInterface {
    pub async fn new(config: Arc<Config>) -> anyhow::Result<Self> {
        let history_manager = HistoryManager::new(config).await?;
        Ok(Self { history_manager })
    }
}

#[zbus::interface(name = "com.redhat.lightspeed.history")]
impl HistoryInterface {
    /// Return the full conversation history for a user.
    async fn get_history(&self, user_id: &str) -> fdo::Result<HistoryList> {
        info!("GetHistory for user={}", user_id);
        let entries = self
            .history_manager
            .read(user_id)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        Ok(HistoryList { histories: entries })
    }

    /// Return the first conversation in a given chat.
    async fn get_first_conversation(
        &self,
        user_id: &str,
        from_chat: &str,
    ) -> fdo::Result<HistoryList> {
        info!(
            "GetFirstConversation for user={}, chat={}",
            user_id, from_chat
        );
        let entry = self
            .history_manager
            .read_from_chat(user_id, from_chat)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        Ok(HistoryList {
            histories: entry.into_iter().collect(),
        })
    }

    /// Return the most recent conversation in a given chat.
    async fn get_last_conversation(
        &self,
        user_id: &str,
        from_chat: &str,
    ) -> fdo::Result<HistoryList> {
        info!(
            "GetLastConversation for user={}, chat={}",
            user_id, from_chat
        );
        // For now, same as get_first_conversation (single-entry history).
        let entry = self
            .history_manager
            .read_from_chat(user_id, from_chat)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        Ok(HistoryList {
            histories: entry.into_iter().collect(),
        })
    }

    /// Return conversations matching a filter within a given chat.
    async fn get_filtered_conversation(
        &self,
        user_id: &str,
        filter: &str,
        from_chat: &str,
    ) -> fdo::Result<HistoryList> {
        info!(
            "GetFilteredConversation for user={}, chat={}, filter={}",
            user_id, from_chat, filter
        );
        // Delegate to read_from_chat; filtering is a future enhancement.
        let entry = self
            .history_manager
            .read_from_chat(user_id, from_chat)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        let histories: Vec<HistoryEntry> = entry
            .into_iter()
            .filter(|e| {
                e.question.contains(filter) || e.response.contains(filter)
            })
            .collect();

        Ok(HistoryList { histories })
    }

    /// Erase all history for a user.
    async fn clear_all_history(&self, user_id: &str) -> fdo::Result<()> {
        info!("ClearAllHistory for user={}", user_id);
        self.history_manager
            .clear(user_id)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    /// Erase history for a specific chat.
    async fn clear_history(&self, user_id: &str, from_chat: &str) -> fdo::Result<()> {
        info!("ClearHistory for user={}, chat={}", user_id, from_chat);
        self.history_manager
            .clear_from_chat(user_id, from_chat)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    /// Persist a new history entry for a chat.
    async fn write_history(
        &self,
        chat_id: &str,
        user_id: &str,
        question: &str,
        response: &str,
    ) -> fdo::Result<()> {
        info!("WriteHistory for user={}, chat={}", user_id, chat_id);
        self.history_manager
            .write(chat_id, user_id, question, response)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }
}
