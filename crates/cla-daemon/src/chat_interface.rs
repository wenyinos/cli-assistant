//! ChatInterface D-Bus implementation.
//!
//! Implements the `com.redhat.lightspeed.chat` D-Bus interface.

use std::sync::Arc;

use tracing::{error, info, warn};
use zbus::fdo;
use zbus::message::Header;
use zbus::Connection;

use cla_common::{Config, UserSessionManager};
use cla_dbus::structures::{Question, Response};
use cla_dbus::ClaDbusError;

use crate::authorization;
use crate::database::manager::DatabaseManager;
use crate::database::repository::ChatRepository;
use crate::history::local::LocalHistory;
use crate::http::query;

/// Stateful handle behind the `com.redhat.lightspeed.chat` D-Bus interface.
pub struct ChatInterface {
    chat_repo: ChatRepository,
    #[allow(dead_code)]
    session_manager: UserSessionManager,
    config: Arc<Config>,
    history: LocalHistory,
}

impl ChatInterface {
    pub async fn new(config: Arc<Config>) -> anyhow::Result<Self> {
        let db_manager = DatabaseManager::new(&config).await?;
        let chat_repo = ChatRepository::new(db_manager.clone());
        let history = LocalHistory::from_manager(db_manager);
        let session_manager = UserSessionManager::new()
            .map_err(|e| anyhow::anyhow!("failed to initialize session manager: {}", e))?;

        Ok(Self {
            chat_repo,
            session_manager,
            config,
            history,
        })
    }

    /// Compose the user message from the question input, including any
    /// attached context (stdin, attachments, terminal output).
    fn compose_user_message(input: &Question) -> String {
        let mut parts = vec![input.message.clone()];

        if let Some(ref stdin) = input.stdin {
            if !stdin.stdin.is_empty() {
                parts.push(format!("\n\n[stdin context]\n{}", stdin.stdin));
            }
        }
        if let Some(ref att) = input.attachment {
            if !att.contents.is_empty() {
                parts.push(format!(
                    "\n\n[attached file: {}]\n{}",
                    att.mimetype, att.contents
                ));
            }
        }
        if let Some(ref term) = input.terminal {
            if !term.output.is_empty() {
                parts.push(format!("\n\n[terminal output]\n{}", term.output));
            }
        }

        parts.join("")
    }

    /// Load the conversation context for a question that carries
    /// `context_chat` — compacting the history first when it has outgrown the
    /// context budget.
    ///
    /// Any failure here is logged and downgraded to an empty context: a broken
    /// summary must never block an answer.
    async fn conversation_context(
        &self,
        user_id: &str,
        message_input: &Question,
        user_message: &str,
    ) -> (Option<String>, Vec<(String, String)>) {
        let Some(chat_name) = message_input.context_chat.as_deref() else {
            return (None, Vec::new());
        };

        // Without history recording there is nothing to attach.
        if !self.config.history.enabled {
            return (None, Vec::new());
        }

        let context = match self.history.context_for_chat(user_id, chat_name).await {
            Ok(Some(context)) => context,
            Ok(None) => return (None, Vec::new()),
            Err(e) => {
                warn!("Failed to load conversation context for chat '{chat_name}': {e}");
                return (None, Vec::new());
            }
        };

        let mut summary = context.summary.clone();
        let mut turns: Vec<(String, String)> = context
            .interactions
            .iter()
            .map(|i| (i.question.clone(), i.response.clone()))
            .collect();

        if let Some(fold_until) =
            query::compaction_point(&self.config, user_message, summary.as_deref(), &turns)
        {
            let folded = turns[..fold_until].to_vec();
            let folded_ids: Vec<String> = context.interactions[..fold_until]
                .iter()
                .map(|i| i.id.clone())
                .collect();
            let material = query::compaction_input(summary.as_deref(), &folded);

            match query::summarize(&self.config, &material).await {
                Ok(new_summary) => {
                    match self
                        .history
                        .apply_compaction(&context.history_id, &new_summary, &folded_ids)
                        .await
                    {
                        Ok(()) => {
                            info!(
                                "Compacted {fold_until} turns of chat '{chat_name}' into a summary"
                            );
                            summary = Some(new_summary);
                            turns.drain(..fold_until);
                        }
                        Err(e) => warn!("Failed to store the conversation summary: {e}"),
                    }
                }
                Err(e) => warn!("Failed to compact the conversation history: {e}"),
            }
        }

        (summary, turns)
    }
}

#[zbus::interface(name = "com.redhat.lightspeed.chat")]
impl ChatInterface {
    /// Submit a question to the LLM backend and return the answer.
    async fn ask_question(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
        message_input: Question,
    ) -> fdo::Result<Response> {
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
        info!("AskQuestion from user={}", user_id);
        crate::audit::event(
            self.config.logging.audit.enabled,
            "AskQuestion",
            user_id,
            None,
        );

        let user_message = Self::compose_user_message(&message_input);

        // Conversation pages attach recent history; single-shot calls don't.
        let (summary, turns) = self
            .conversation_context(user_id, &message_input, &user_message)
            .await;

        match query::submit(&self.config, &user_message, summary.as_deref(), &turns).await {
            Ok(text) => Ok(Response { message: text }),
            Err(e) => {
                error!("LLM query failed: {}", e);
                Err(ClaDbusError::RequestFailed(e.to_string()).into())
            }
        }
    }

    /// List all chats for a user.
    async fn get_all_chat_from_user(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
    ) -> fdo::Result<cla_dbus::structures::ChatList> {
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
        info!("GetAllChatFromUser for user={}", user_id);
        let models = self
            .chat_repo
            .select_all_by_user(user_id)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        let chats = models
            .into_iter()
            .map(|m| cla_dbus::structures::ChatEntry {
                id: m.id,
                name: m.name,
                description: m.description.unwrap_or_default(),
                created_at: m.created_at,
                updated_at: m.updated_at,
                deleted_at: m.deleted_at,
            })
            .collect();

        Ok(cla_dbus::structures::ChatList { chats })
    }

    /// Delete all chats for a user.
    async fn delete_all_chat_for_user(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
    ) -> fdo::Result<()> {
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
        info!("DeleteAllChatForUser for user={}", user_id);
        self.chat_repo
            .soft_delete_all(user_id)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    /// Delete a specific chat by name.
    async fn delete_chat_for_user(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
        name: &str,
    ) -> fdo::Result<()> {
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
        info!("DeleteChatForUser for user={}, chat={}", user_id, name);
        let chat = self
            .chat_repo
            .select_by_name(user_id, name)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        match chat {
            Some(c) => self
                .chat_repo
                .soft_delete(&c.id)
                .await
                .map_err(|e| fdo::Error::Failed(e.to_string())),
            None => Err(ClaDbusError::ChatNotFound(name.to_string()).into()),
        }
    }

    /// Get the latest chat name for a user.
    async fn get_latest_chat_from_user(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
    ) -> fdo::Result<String> {
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
        info!("GetLatestChatFromUser for user={}", user_id);
        let chat = self
            .chat_repo
            .select_latest_chat(user_id)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        match chat {
            Some(c) => Ok(c.name),
            None => Err(ClaDbusError::ChatNotFound("no chats".to_string()).into()),
        }
    }

    /// Check if a chat is available.
    async fn is_chat_available(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
        name: &str,
    ) -> fdo::Result<bool> {
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
        info!("IsChatAvailable for user={}, chat={}", user_id, name);
        let chat = self
            .chat_repo
            .select_by_name(user_id, name)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        Ok(chat.is_some())
    }

    /// Get chat ID by name.
    async fn get_chat_id(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
        name: &str,
    ) -> fdo::Result<String> {
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
        info!("GetChatId for user={}, chat={}", user_id, name);
        let chat = self
            .chat_repo
            .select_by_name(user_id, name)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        match chat {
            Some(c) => Ok(c.id),
            None => Err(ClaDbusError::ChatNotFound(name.to_string()).into()),
        }
    }

    /// Create a new chat and return its ID.
    async fn create_chat(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        user_id: &str,
        name: &str,
        description: &str,
    ) -> fdo::Result<String> {
        authorization::authorize_internal_user(conn, &header, user_id, &self.session_manager)
            .await?;
        info!("CreateChat for user={}, name={}", user_id, name);
        crate::audit::event(
            self.config.logging.audit.enabled,
            "CreateChat",
            user_id,
            None,
        );
        let chat = self
            .chat_repo
            .insert(user_id, name, Some(description))
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        Ok(chat.id)
    }
}
