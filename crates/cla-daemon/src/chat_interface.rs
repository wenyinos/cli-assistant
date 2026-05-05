//! ChatInterface D-Bus implementation.
//!
//! Implements the `com.redhat.lightspeed.chat` D-Bus interface.

use std::sync::Arc;

use tracing::{error, info};
use zbus::fdo;

use cla_common::Config;
use cla_dbus::structures::{Question, Response};
use cla_dbus::ClaDbusError;

use crate::database::manager::DatabaseManager;
use crate::database::repository::ChatRepository;
use crate::http::query;
use crate::user_interface::UserSessionManager;

/// Stateful handle behind the `com.redhat.lightspeed.chat` D-Bus interface.
pub struct ChatInterface {
    chat_repo: ChatRepository,
    #[allow(dead_code)]
    session_manager: UserSessionManager,
    config: Arc<Config>,
}

impl ChatInterface {
    pub async fn new(config: Arc<Config>) -> anyhow::Result<Self> {
        let db_manager = DatabaseManager::new(&config).await?;
        let chat_repo = ChatRepository::new(db_manager.clone());
        let session_manager = UserSessionManager::new();

        Ok(Self {
            chat_repo,
            session_manager,
            config,
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
}

#[zbus::interface(name = "com.redhat.lightspeed.chat")]
impl ChatInterface {
    /// Submit a question to the LLM backend and return the answer.
    async fn ask_question(
        &self,
        user_id: &str,
        message_input: Question,
    ) -> fdo::Result<Response> {
        info!("AskQuestion from user={}", user_id);

        let user_message = Self::compose_user_message(&message_input);

        match query::submit(&self.config, &user_message).await {
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
        user_id: &str,
    ) -> fdo::Result<cla_dbus::structures::ChatList> {
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
    async fn delete_all_chat_for_user(&self, user_id: &str) -> fdo::Result<()> {
        info!("DeleteAllChatForUser for user={}", user_id);
        self.chat_repo
            .soft_delete_all(user_id)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    /// Delete a specific chat by name.
    async fn delete_chat_for_user(&self, user_id: &str, name: &str) -> fdo::Result<()> {
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
    async fn get_latest_chat_from_user(&self, user_id: &str) -> fdo::Result<String> {
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
    async fn is_chat_available(&self, user_id: &str, name: &str) -> fdo::Result<bool> {
        info!("IsChatAvailable for user={}, chat={}", user_id, name);
        let chat = self
            .chat_repo
            .select_by_name(user_id, name)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        Ok(chat.is_some())
    }

    /// Get chat ID by name.
    async fn get_chat_id(&self, user_id: &str, name: &str) -> fdo::Result<String> {
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
        user_id: &str,
        name: &str,
        description: &str,
    ) -> fdo::Result<String> {
        info!("CreateChat for user={}, name={}", user_id, name);
        let chat = self
            .chat_repo
            .insert(user_id, name, Some(description))
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        Ok(chat.id)
    }
}
