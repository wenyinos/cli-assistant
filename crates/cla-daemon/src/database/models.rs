//! Database model structs mapped to the SQLite tables.

use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::{Error, FromRow, Row};

/// A named chat session that groups related history entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatModel {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

// `FromRow` is implemented by hand: deriving it would pull in sqlx-macros,
// whose bundled-SQLite proc-macro dylib fails to resolve sqlite3_unlock_notify
// on some toolchains (observed with Arch's rust package).

impl FromRow<'_, SqliteRow> for ChatModel {
    fn from_row(row: &SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            deleted_at: row.try_get("deleted_at")?,
        })
    }
}

/// A history record belonging to a chat – holds zero or more interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryModel {
    pub id: String,
    pub user_id: String,
    pub chat_id: String,
    pub created_at: String,
    pub updated_at: String,
    /// Compacted summary of earlier conversation turns. `None` until the
    /// conversation grows large enough to be compacted.
    pub summary: Option<String>,
    pub deleted_at: Option<String>,
}

impl FromRow<'_, SqliteRow> for HistoryModel {
    fn from_row(row: &SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            chat_id: row.try_get("chat_id")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            summary: row.try_get("summary")?,
            deleted_at: row.try_get("deleted_at")?,
        })
    }
}

/// A single question/response pair inside a history record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionModel {
    pub id: String,
    pub history_id: String,
    pub question: String,
    pub response: String,
    pub created_at: String,
    pub updated_at: String,
    /// Whether this turn has been folded into the history summary.
    pub summarized: bool,
    pub deleted_at: Option<String>,
}

impl FromRow<'_, SqliteRow> for InteractionModel {
    fn from_row(row: &SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            history_id: row.try_get("history_id")?,
            question: row.try_get("question")?,
            response: row.try_get("response")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            summarized: row.try_get("summarized")?,
            deleted_at: row.try_get("deleted_at")?,
        })
    }
}
