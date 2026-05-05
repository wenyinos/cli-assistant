//! Database model structs mapped to the SQLite tables.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A named chat session that groups related history entries.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ChatModel {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

/// A history record belonging to a chat – holds zero or more interactions.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct HistoryModel {
    pub id: String,
    pub user_id: String,
    pub chat_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

/// A single question/response pair inside a history record.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct InteractionModel {
    pub id: String,
    pub history_id: String,
    pub question: String,
    pub response: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}
