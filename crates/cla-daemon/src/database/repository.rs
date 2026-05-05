//! Repository layer – typed data-access helpers for each table.

use uuid::Uuid;

use super::manager::DatabaseManager;
use super::models::{ChatModel, HistoryModel, InteractionModel};

/// Current UTC timestamp as an ISO-8601 string suitable for SQLite TEXT columns.
fn now_str() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

// =========================================================================
// ChatRepository
// =========================================================================

#[derive(Clone)]
pub struct ChatRepository {
    manager: DatabaseManager,
}

impl ChatRepository {
    pub fn new(manager: DatabaseManager) -> Self {
        Self { manager }
    }

    /// Insert a new chat and return the persisted row.
    pub async fn insert(
        &self,
        user_id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<ChatModel, sqlx::Error> {
        let id = new_id();
        let ts = now_str();
        sqlx::query_as::<_, ChatModel>(
            "INSERT INTO chats (id, user_id, name, description, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING *",
        )
        .bind(&id)
        .bind(user_id)
        .bind(name)
        .bind(description)
        .bind(&ts)
        .bind(&ts)
        .fetch_one(self.manager.pool())
        .await
    }

    /// Return the most recently created non-deleted chat for a user.
    pub async fn select_latest_chat(
        &self,
        user_id: &str,
    ) -> Result<Option<ChatModel>, sqlx::Error> {
        sqlx::query_as::<_, ChatModel>(
            "SELECT * FROM chats
             WHERE user_id = $1 AND deleted_at IS NULL
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(self.manager.pool())
        .await
    }

    /// Find a non-deleted chat by its unique (user, name) pair.
    pub async fn select_by_name(
        &self,
        user_id: &str,
        name: &str,
    ) -> Result<Option<ChatModel>, sqlx::Error> {
        sqlx::query_as::<_, ChatModel>(
            "SELECT * FROM chats
             WHERE user_id = $1 AND name = $2 AND deleted_at IS NULL",
        )
        .bind(user_id)
        .bind(name)
        .fetch_optional(self.manager.pool())
        .await
    }

    /// Find a non-deleted chat by its primary key.
    #[allow(dead_code)]
    pub async fn select_by_id(&self, id: &str) -> Result<Option<ChatModel>, sqlx::Error> {
        sqlx::query_as::<_, ChatModel>(
            "SELECT * FROM chats WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(self.manager.pool())
        .await
    }

    /// Return all non-deleted chats for a user, newest first.
    pub async fn select_all_by_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<ChatModel>, sqlx::Error> {
        sqlx::query_as::<_, ChatModel>(
            "SELECT * FROM chats
             WHERE user_id = $1 AND deleted_at IS NULL
             ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(self.manager.pool())
        .await
    }

    /// Soft-delete a chat by setting `deleted_at`.
    pub async fn soft_delete(&self, id: &str) -> Result<(), sqlx::Error> {
        let ts = now_str();
        sqlx::query(
            "UPDATE chats SET deleted_at = $1, updated_at = $2 WHERE id = $3",
        )
        .bind(&ts)
        .bind(&ts)
        .bind(id)
        .execute(self.manager.pool())
        .await?;
        Ok(())
    }

    /// Soft-delete all chats for a user.
    pub async fn soft_delete_all(&self, user_id: &str) -> Result<(), sqlx::Error> {
        let ts = now_str();
        sqlx::query(
            "UPDATE chats SET deleted_at = $1, updated_at = $2 WHERE user_id = $3 AND deleted_at IS NULL",
        )
        .bind(&ts)
        .bind(&ts)
        .bind(user_id)
        .execute(self.manager.pool())
        .await?;
        Ok(())
    }
}

// =========================================================================
// HistoryRepository
// =========================================================================

#[derive(Clone)]
pub struct HistoryRepository {
    manager: DatabaseManager,
}

impl HistoryRepository {
    pub fn new(manager: DatabaseManager) -> Self {
        Self { manager }
    }

    /// Insert a new history record.
    pub async fn insert(
        &self,
        user_id: &str,
        chat_id: &str,
    ) -> Result<HistoryModel, sqlx::Error> {
        let id = new_id();
        let ts = now_str();
        sqlx::query_as::<_, HistoryModel>(
            "INSERT INTO histories (id, user_id, chat_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *",
        )
        .bind(&id)
        .bind(user_id)
        .bind(chat_id)
        .bind(&ts)
        .bind(&ts)
        .fetch_one(self.manager.pool())
        .await
    }

    /// Return the (non-deleted) history entry that belongs to a chat.
    pub async fn select_by_chat_id(
        &self,
        chat_id: &str,
    ) -> Result<Option<HistoryModel>, sqlx::Error> {
        sqlx::query_as::<_, HistoryModel>(
            "SELECT * FROM histories
             WHERE chat_id = $1 AND deleted_at IS NULL",
        )
        .bind(chat_id)
        .fetch_optional(self.manager.pool())
        .await
    }

    /// Return all non-deleted history entries for a user, newest first.
    pub async fn select_all_history(
        &self,
        user_id: &str,
    ) -> Result<Vec<HistoryModel>, sqlx::Error> {
        sqlx::query_as::<_, HistoryModel>(
            "SELECT * FROM histories
             WHERE user_id = $1 AND deleted_at IS NULL
             ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(self.manager.pool())
        .await
    }

    /// Soft-delete **all** history for a user (cascading to interactions).
    pub async fn delete_all(&self, user_id: &str) -> Result<(), sqlx::Error> {
        let ts = now_str();

        // Soft-delete interactions first.
        sqlx::query(
            "UPDATE interactions SET deleted_at = $1, updated_at = $2
             WHERE history_id IN (
                 SELECT id FROM histories WHERE user_id = $3 AND deleted_at IS NULL
             )",
        )
        .bind(&ts)
        .bind(&ts)
        .bind(user_id)
        .execute(self.manager.pool())
        .await?;

        // Then soft-delete histories.
        sqlx::query(
            "UPDATE histories SET deleted_at = $1, updated_at = $2
             WHERE user_id = $3 AND deleted_at IS NULL",
        )
        .bind(&ts)
        .bind(&ts)
        .bind(user_id)
        .execute(self.manager.pool())
        .await?;

        Ok(())
    }

    /// Soft-delete history for a specific chat (cascading to interactions).
    pub async fn delete_by_chat_name(
        &self,
        user_id: &str,
        chat_name: &str,
    ) -> Result<(), sqlx::Error> {
        let ts = now_str();

        // Find the chat first.
        let chat = sqlx::query_as::<_, ChatModel>(
            "SELECT * FROM chats WHERE user_id = $1 AND name = $2 AND deleted_at IS NULL",
        )
        .bind(user_id)
        .bind(chat_name)
        .fetch_optional(self.manager.pool())
        .await?;

        let chat = match chat {
            Some(c) => c,
            None => return Ok(()),
        };

        // Soft-delete interactions for this chat's histories.
        sqlx::query(
            "UPDATE interactions SET deleted_at = $1, updated_at = $2
             WHERE history_id IN (
                 SELECT id FROM histories WHERE chat_id = $3 AND deleted_at IS NULL
             )",
        )
        .bind(&ts)
        .bind(&ts)
        .bind(&chat.id)
        .execute(self.manager.pool())
        .await?;

        // Soft-delete histories.
        sqlx::query(
            "UPDATE histories SET deleted_at = $1, updated_at = $2
             WHERE chat_id = $3 AND deleted_at IS NULL",
        )
        .bind(&ts)
        .bind(&ts)
        .bind(&chat.id)
        .execute(self.manager.pool())
        .await?;

        Ok(())
    }
}

// =========================================================================
// InteractionRepository
// =========================================================================

#[derive(Clone)]
pub struct InteractionRepository {
    manager: DatabaseManager,
}

impl InteractionRepository {
    pub fn new(manager: DatabaseManager) -> Self {
        Self { manager }
    }

    /// Insert a new interaction (question + response) into a history record.
    pub async fn insert(
        &self,
        history_id: &str,
        question: &str,
        response: &str,
    ) -> Result<InteractionModel, sqlx::Error> {
        let id = new_id();
        let ts = now_str();
        sqlx::query_as::<_, InteractionModel>(
            "INSERT INTO interactions (id, history_id, question, response, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING *",
        )
        .bind(&id)
        .bind(history_id)
        .bind(question)
        .bind(response)
        .bind(&ts)
        .bind(&ts)
        .fetch_one(self.manager.pool())
        .await
    }

    /// Return all non-deleted interactions for a given history record.
    pub async fn select_by_history_id(
        &self,
        history_id: &str,
    ) -> Result<Vec<InteractionModel>, sqlx::Error> {
        sqlx::query_as::<_, InteractionModel>(
            "SELECT * FROM interactions
             WHERE history_id = $1 AND deleted_at IS NULL
             ORDER BY created_at ASC",
        )
        .bind(history_id)
        .fetch_all(self.manager.pool())
        .await
    }

    /// Soft-delete a single interaction.
    #[allow(dead_code)]
    pub async fn soft_delete(&self, id: &str) -> Result<(), sqlx::Error> {
        let ts = now_str();
        sqlx::query(
            "UPDATE interactions SET deleted_at = $1, updated_at = $2 WHERE id = $3",
        )
        .bind(&ts)
        .bind(&ts)
        .bind(id)
        .execute(self.manager.pool())
        .await?;
        Ok(())
    }
}

// Re-import ChatModel used in HistoryRepository::delete_by_chat_name
// (already imported at top of file)
