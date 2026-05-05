//! Local history plugin – backed by the SQLite database.

use std::sync::Arc;

use cla_common::Config;
use cla_dbus::HistoryEntry;

use crate::database::manager::DatabaseManager;
use crate::database::repository::{ChatRepository, HistoryRepository, InteractionRepository};

/// SQLite-backed history plugin.
pub struct LocalHistory {
    chat_repo: ChatRepository,
    history_repo: HistoryRepository,
    interaction_repo: InteractionRepository,
}

impl LocalHistory {
    pub async fn new(config: Arc<Config>) -> anyhow::Result<Self> {
        let manager = DatabaseManager::new(&config).await?;
        Ok(Self {
            chat_repo: ChatRepository::new(manager.clone()),
            history_repo: HistoryRepository::new(manager.clone()),
            interaction_repo: InteractionRepository::new(manager),
        })
    }

    pub async fn read(&self, user_id: &str) -> anyhow::Result<Vec<HistoryEntry>> {
        let histories = self.history_repo.select_all_history(user_id).await?;

        let mut entries = Vec::with_capacity(histories.len());
        for h in histories {
            let interactions = self.interaction_repo.select_by_history_id(&h.id).await?;

            // Aggregate into a single entry per history record.
            let question = interactions
                .iter()
                .map(|i| i.question.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let response = interactions
                .iter()
                .map(|i| i.response.as_str())
                .collect::<Vec<_>>()
                .join("\n");

            entries.push(HistoryEntry {
                id: h.id,
                chat_id: h.chat_id,
                question,
                response,
                created_at: h.created_at,
            });
        }

        Ok(entries)
    }

    pub async fn read_from_chat(
        &self,
        user_id: &str,
        from_chat: &str,
    ) -> anyhow::Result<Option<HistoryEntry>> {
        let chat = match self.chat_repo.select_by_name(user_id, from_chat).await? {
            Some(c) => c,
            None => return Ok(None),
        };

        let history = match self.history_repo.select_by_chat_id(&chat.id).await? {
            Some(h) => h,
            None => return Ok(None),
        };

        let interactions = self
            .interaction_repo
            .select_by_history_id(&history.id)
            .await?;

        let question = interactions
            .iter()
            .map(|i| i.question.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let response = interactions
            .iter()
            .map(|i| i.response.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(Some(HistoryEntry {
            id: history.id,
            chat_id: history.chat_id,
            question,
            response,
            created_at: history.created_at,
        }))
    }

    pub async fn write(
        &self,
        chat_id: &str,
        user_id: &str,
        query: &str,
        response: &str,
    ) -> anyhow::Result<()> {
        // Ensure we have a history record for this chat.
        let history = match self.history_repo.select_by_chat_id(chat_id).await? {
            Some(h) => h,
            None => self.history_repo.insert(user_id, chat_id).await?,
        };

        self.interaction_repo
            .insert(&history.id, query, response)
            .await?;

        Ok(())
    }

    pub async fn clear(&self, user_id: &str) -> anyhow::Result<()> {
        self.history_repo.delete_all(user_id).await?;
        Ok(())
    }

    pub async fn clear_from_chat(
        &self,
        user_id: &str,
        from_chat: &str,
    ) -> anyhow::Result<()> {
        self.history_repo
            .delete_by_chat_name(user_id, from_chat)
            .await?;
        Ok(())
    }
}
