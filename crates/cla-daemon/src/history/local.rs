//! Local history plugin – backed by the SQLite database.

use std::sync::Arc;

use cla_common::Config;
use cla_dbus::HistoryEntry;

use crate::database::manager::DatabaseManager;
use crate::database::models::InteractionModel;
use crate::database::repository::{ChatRepository, HistoryRepository, InteractionRepository};

/// Conversation context for a chat: the compacted summary (if any) plus the
/// turns that have not been folded into it yet.
pub struct ChatContext {
    pub history_id: String,
    pub summary: Option<String>,
    pub interactions: Vec<InteractionModel>,
}

/// SQLite-backed history plugin.
pub struct LocalHistory {
    chat_repo: ChatRepository,
    history_repo: HistoryRepository,
    interaction_repo: InteractionRepository,
}

impl LocalHistory {
    pub async fn new(config: Arc<Config>) -> anyhow::Result<Self> {
        Ok(Self::from_manager(DatabaseManager::new(&config).await?))
    }

    /// Build from an existing manager so callers can share one connection pool.
    pub fn from_manager(manager: DatabaseManager) -> Self {
        Self {
            chat_repo: ChatRepository::new(manager.clone()),
            history_repo: HistoryRepository::new(manager.clone()),
            interaction_repo: InteractionRepository::new(manager),
        }
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

    /// Load the conversation context for a chat by name: its summary plus the
    /// turns that have not been folded into it yet, in chronological order.
    ///
    /// Returns `None` when the chat (or its history record) does not exist.
    pub async fn context_for_chat(
        &self,
        user_id: &str,
        chat_name: &str,
    ) -> anyhow::Result<Option<ChatContext>> {
        let chat = match self.chat_repo.select_by_name(user_id, chat_name).await? {
            Some(c) => c,
            None => return Ok(None),
        };

        let history = match self.history_repo.select_by_chat_id(&chat.id).await? {
            Some(h) => h,
            None => return Ok(None),
        };

        let interactions = self
            .interaction_repo
            .select_unsummarized_by_history_id(&history.id)
            .await?;

        Ok(Some(ChatContext {
            history_id: history.id,
            summary: history.summary,
            interactions,
        }))
    }

    /// Persist a compaction: store the new summary and mark the folded
    /// interactions so they are not sent again.
    pub async fn apply_compaction(
        &self,
        history_id: &str,
        summary: &str,
        summarized_ids: &[String],
    ) -> anyhow::Result<()> {
        self.history_repo
            .update_summary(history_id, summary)
            .await?;
        self.interaction_repo
            .mark_summarized(summarized_ids)
            .await?;
        Ok(())
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

    pub async fn clear_from_chat(&self, user_id: &str, from_chat: &str) -> anyhow::Result<()> {
        self.history_repo
            .delete_by_chat_name(user_id, from_chat)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config() -> Config {
        let mut config = Config::default();
        config.database.path =
            std::env::temp_dir().join(format!("cla-local-history-{}.db", uuid::Uuid::new_v4()));
        config
    }

    #[tokio::test]
    async fn write_and_read_preserves_question_and_response() {
        let config = Arc::new(temp_config());
        let manager = DatabaseManager::new(&config).await.expect("database");
        let chat = ChatRepository::new(manager)
            .insert("user-1", "default", Some("Default chat"))
            .await
            .expect("chat");
        let history = LocalHistory::new(config).await.expect("history");

        history
            .write(
                &chat.id,
                "user-1",
                "How do I check disk space?",
                "Use df -h.",
            )
            .await
            .expect("write");

        let entries = history.read("user-1").await.expect("read");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].question, "How do I check disk space?");
        assert_eq!(entries[0].response, "Use df -h.");
        assert_ne!(entries[0].question, entries[0].response);
    }

    #[tokio::test]
    async fn context_for_chat_tracks_summary_and_folded_turns() {
        let config = Arc::new(temp_config());
        let manager = DatabaseManager::new(&config).await.expect("database");
        let chat = ChatRepository::new(manager)
            .insert("user-1", "work", Some("Work chat"))
            .await
            .expect("chat");
        let history = LocalHistory::new(config).await.expect("history");

        // Unknown chats have no context at all.
        assert!(history
            .context_for_chat("user-1", "missing")
            .await
            .expect("query")
            .is_none());

        for i in 1..=3 {
            history
                .write(&chat.id, "user-1", &format!("q{i}"), &format!("r{i}"))
                .await
                .expect("write");
        }

        let context = history
            .context_for_chat("user-1", "work")
            .await
            .expect("context")
            .expect("chat context");
        assert!(context.summary.is_none());
        assert_eq!(context.interactions.len(), 3);
        assert_eq!(context.interactions[0].question, "q1");

        // Fold the first two turns into a summary.
        let folded: Vec<String> = context.interactions[..2]
            .iter()
            .map(|i| i.id.clone())
            .collect();
        history
            .apply_compaction(&context.history_id, "User asked q1 and q2.", &folded)
            .await
            .expect("compact");

        let context = history
            .context_for_chat("user-1", "work")
            .await
            .expect("context")
            .expect("chat context");
        assert_eq!(context.summary.as_deref(), Some("User asked q1 and q2."));
        assert_eq!(context.interactions.len(), 1);
        assert_eq!(context.interactions[0].question, "q3");

        // The user-facing history still shows every turn.
        let entries = history.read("user-1").await.expect("read");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].question.contains("q1"));
        assert!(entries[0].question.contains("q3"));
    }

    /// A database created before the summary/summarized columns existed must
    /// be migrated in place.
    #[tokio::test]
    async fn migrates_legacy_schema() {
        let config = temp_config();
        let path = config.database.path.clone();

        // Build a legacy database by hand (no summary/summarized columns).
        let url = format!("sqlite:{}?mode=rwc", path.display());
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .expect("legacy db");
        sqlx::query(
            "CREATE TABLE histories (
                id TEXT PRIMARY KEY, user_id TEXT NOT NULL, chat_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')), deleted_at TEXT)",
        )
        .execute(&pool)
        .await
        .expect("legacy histories");
        sqlx::query(
            "CREATE TABLE interactions (
                id TEXT PRIMARY KEY, history_id TEXT NOT NULL, question TEXT NOT NULL,
                response TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')), deleted_at TEXT)",
        )
        .execute(&pool)
        .await
        .expect("legacy interactions");
        pool.close().await;

        // Opening through DatabaseManager must add the missing columns, and
        // running it twice must stay idempotent.
        DatabaseManager::new(&config).await.expect("migrate once");
        DatabaseManager::new(&config).await.expect("migrate twice");

        let history = LocalHistory::new(Arc::new(config)).await.expect("history");
        let context = history.context_for_chat("user-1", "work").await;
        assert!(context.expect("query").is_none()); // empty legacy db: no chats
    }
}
