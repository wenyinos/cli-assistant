//! HistoryManager – delegates to a pluggable backend via enum dispatch.

use std::sync::Arc;

use cla_common::Config;
use cla_dbus::HistoryEntry;

use super::local::LocalHistory;

// ---------------------------------------------------------------------------
// Plugin enum – add new variants here when introducing other backends.
// ---------------------------------------------------------------------------

enum HistoryBackend {
    Local(LocalHistory),
}

impl HistoryBackend {
    async fn read(&self, user_id: &str) -> anyhow::Result<Vec<HistoryEntry>> {
        match self {
            Self::Local(p) => p.read(user_id).await,
        }
    }

    async fn read_from_chat(
        &self,
        user_id: &str,
        from_chat: &str,
    ) -> anyhow::Result<Option<HistoryEntry>> {
        match self {
            Self::Local(p) => p.read_from_chat(user_id, from_chat).await,
        }
    }

    async fn write(
        &self,
        chat_id: &str,
        user_id: &str,
        query: &str,
        response: &str,
    ) -> anyhow::Result<()> {
        match self {
            Self::Local(p) => p.write(chat_id, user_id, query, response).await,
        }
    }

    async fn clear(&self, user_id: &str) -> anyhow::Result<()> {
        match self {
            Self::Local(p) => p.clear(user_id).await,
        }
    }

    async fn clear_from_chat(
        &self,
        user_id: &str,
        from_chat: &str,
    ) -> anyhow::Result<()> {
        match self {
            Self::Local(p) => p.clear_from_chat(user_id, from_chat).await,
        }
    }
}

// ---------------------------------------------------------------------------
// Public facade
// ---------------------------------------------------------------------------

/// High-level facade that selects and delegates to the configured plugin.
pub struct HistoryManager {
    backend: HistoryBackend,
}

impl HistoryManager {
    pub async fn new(config: Arc<Config>) -> anyhow::Result<Self> {
        // Currently only "local" (SQLite) is supported.
        let backend = HistoryBackend::Local(LocalHistory::new(config).await?);
        Ok(Self { backend })
    }

    pub async fn read(&self, user_id: &str) -> anyhow::Result<Vec<HistoryEntry>> {
        self.backend.read(user_id).await
    }

    pub async fn read_from_chat(
        &self,
        user_id: &str,
        from_chat: &str,
    ) -> anyhow::Result<Option<HistoryEntry>> {
        self.backend.read_from_chat(user_id, from_chat).await
    }

    pub async fn write(
        &self,
        chat_id: &str,
        user_id: &str,
        query: &str,
        response: &str,
    ) -> anyhow::Result<()> {
        self.backend
            .write(chat_id, user_id, query, response)
            .await
    }

    pub async fn clear(&self, user_id: &str) -> anyhow::Result<()> {
        self.backend.clear(user_id).await
    }

    pub async fn clear_from_chat(
        &self,
        user_id: &str,
        from_chat: &str,
    ) -> anyhow::Result<()> {
        self.backend.clear_from_chat(user_id, from_chat).await
    }
}
