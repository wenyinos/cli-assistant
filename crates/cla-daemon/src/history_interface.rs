//! HistoryInterface D-Bus implementation.
//!
//! Implements the `com.redhat.lightspeed.history` D-Bus interface.

use std::sync::Arc;

use tracing::info;
use zbus::fdo;
use zbus::message::Header;
use zbus::Connection;

use cla_common::{Config, UserSessionManager};
use cla_dbus::structures::{HistoryEntry, HistoryList};
use cla_dbus::ClaDbusError;

use crate::authorization;
use crate::history::manager::HistoryManager;

/// Stateful handle behind the `com.redhat.lightspeed.history` D-Bus interface.
pub struct HistoryInterface {
    history_manager: HistoryManager,
    history_enabled: bool,
    audit_enabled: bool,
    session_manager: UserSessionManager,
}

impl HistoryInterface {
    pub async fn new(config: Arc<Config>) -> anyhow::Result<Self> {
        let history_enabled = config.history.enabled;
        let audit_enabled = config.logging.audit.enabled;
        let history_manager = HistoryManager::new(config).await?;
        let session_manager = UserSessionManager::new()
            .map_err(|e| anyhow::anyhow!("failed to initialize session manager: {}", e))?;
        Ok(Self {
            history_manager,
            history_enabled,
            audit_enabled,
            session_manager,
        })
    }

    fn ensure_history_enabled(&self) -> fdo::Result<()> {
        if self.history_enabled {
            return Ok(());
        }

        tracing::warn!("History is disabled in the configuration; refusing history operation");
        Err(ClaDbusError::HistoryNotEnabled.into())
    }
}

#[zbus::interface(name = "com.redhat.lightspeed.history")]
impl HistoryInterface {
    /// Return the full conversation history for a user.
    async fn get_history(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
    ) -> fdo::Result<HistoryList> {
        self.ensure_history_enabled()?;
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
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
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
        from_chat: &str,
    ) -> fdo::Result<HistoryList> {
        self.ensure_history_enabled()?;
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
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
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
        from_chat: &str,
    ) -> fdo::Result<HistoryList> {
        self.ensure_history_enabled()?;
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
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
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
        filter: &str,
        from_chat: &str,
    ) -> fdo::Result<HistoryList> {
        self.ensure_history_enabled()?;
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
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
            .filter(|e| e.question.contains(filter) || e.response.contains(filter))
            .collect();

        Ok(HistoryList { histories })
    }

    /// Erase all history for a user.
    async fn clear_all_history(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
    ) -> fdo::Result<()> {
        self.ensure_history_enabled()?;
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
        info!("ClearAllHistory for user={}", user_id);
        crate::audit::event(self.audit_enabled, "ClearAllHistory", user_id, None);
        self.history_manager
            .clear(user_id)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    /// Erase history for a specific chat.
    async fn clear_history(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
        from_chat: &str,
    ) -> fdo::Result<()> {
        self.ensure_history_enabled()?;
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
        info!("ClearHistory for user={}, chat={}", user_id, from_chat);
        crate::audit::event(self.audit_enabled, "ClearHistory", user_id, Some(from_chat));
        self.history_manager
            .clear_from_chat(user_id, from_chat)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    /// Persist a new history entry for a chat.
    async fn write_history(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        chat_id: &str,
        user_id: &str,
        question: &str,
        response: &str,
    ) -> fdo::Result<()> {
        self.ensure_history_enabled()?;
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
        info!("WriteHistory for user={}, chat={}", user_id, chat_id);
        crate::audit::event(self.audit_enabled, "WriteHistory", user_id, Some(chat_id));
        self.history_manager
            .write(chat_id, user_id, question, response)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cla_common::Config;

    use super::*;

    fn temp_config(history_enabled: bool) -> Config {
        let mut config = Config::default();
        config.history.enabled = history_enabled;
        config.database.path =
            std::env::temp_dir().join(format!("cla-history-{}.db", uuid::Uuid::new_v4()));
        config
    }

    #[tokio::test]
    async fn disabled_history_returns_dedicated_error() {
        let config = Arc::new(temp_config(false));
        let interface = HistoryInterface::new(config).await.expect("interface");

        let err = interface.ensure_history_enabled().unwrap_err();
        assert!(err.to_string().contains("history not enabled"));
    }

    #[tokio::test]
    async fn enabled_history_passes_guard() {
        let config = Arc::new(temp_config(true));
        let interface = HistoryInterface::new(config).await.expect("interface");

        assert!(interface.ensure_history_enabled().is_ok());
    }
}
