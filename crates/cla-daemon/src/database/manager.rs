//! SQLite connection manager using sqlx.

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use tracing::info;

use cla_common::Config;

/// Thin wrapper around a sqlx [`SqlitePool`] that auto-creates the schema on
/// first use.
#[derive(Clone)]
pub struct DatabaseManager {
    pool: SqlitePool,
}

impl DatabaseManager {
    /// Open (or create) the SQLite database defined in `config.database.path`
    /// and ensure all tables exist.
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let db_path = &config.database.path;

        // Ensure the parent directory exists.
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let url = format!("sqlite:{}?mode=rwc", db_path.display());
        info!("Connecting to database at {}", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;

        let manager = Self { pool };
        manager.create_tables().await?;
        manager.migrate().await?;

        Ok(manager)
    }

    /// Execute DDL to create the schema if it doesn't already exist.
    async fn create_tables(&self) -> anyhow::Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS chats (
                id          TEXT PRIMARY KEY,
                user_id     TEXT NOT NULL,
                name        TEXT NOT NULL,
                description TEXT,
                created_at  TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
                deleted_at  TEXT
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS histories (
                id         TEXT PRIMARY KEY,
                user_id    TEXT NOT NULL,
                chat_id    TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                summary    TEXT,
                deleted_at TEXT,
                FOREIGN KEY (chat_id) REFERENCES chats(id)
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS interactions (
                id         TEXT PRIMARY KEY,
                history_id TEXT NOT NULL,
                question   TEXT NOT NULL,
                response   TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                summarized INTEGER NOT NULL DEFAULT 0,
                deleted_at TEXT,
                FOREIGN KEY (history_id) REFERENCES histories(id)
            )",
        )
        .execute(&self.pool)
        .await?;

        // Useful indexes
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_chats_user_id ON chats(user_id) WHERE deleted_at IS NULL",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_histories_chat_id ON histories(chat_id) WHERE deleted_at IS NULL",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_interactions_history_id ON interactions(history_id) WHERE deleted_at IS NULL",
        )
        .execute(&self.pool)
        .await?;

        info!("Database schema verified");
        Ok(())
    }

    /// Borrow the underlying connection pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Add columns introduced after a database was first created. SQLite has
    /// no `ADD COLUMN IF NOT EXISTS`, so check `pragma_table_info` first; the
    /// procedure is idempotent and safe to run on every start.
    async fn migrate(&self) -> anyhow::Result<()> {
        for (table, column, definition) in [
            ("histories", "summary", "TEXT"),
            ("interactions", "summarized", "INTEGER NOT NULL DEFAULT 0"),
        ] {
            if self.column_exists(table, column).await? {
                continue;
            }
            let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
            sqlx::query(&sql).execute(&self.pool).await?;
            info!("Added column {table}.{column}");
        }
        Ok(())
    }

    /// Whether `table` already has a column named `column`.
    async fn column_exists(&self, table: &str, column: &str) -> anyhow::Result<bool> {
        let names: Vec<String> =
            sqlx::query_scalar(&format!("SELECT name FROM pragma_table_info('{table}')"))
                .fetch_all(&self.pool)
                .await?;
        Ok(names.iter().any(|name| name == column))
    }
}
