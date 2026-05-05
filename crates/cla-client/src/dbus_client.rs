//! D-Bus client wrapper for communicating with the clad daemon.

use zbus::Connection;

use cla_dbus::constants::{
    CHAT_BUS_NAME, CHAT_OBJECT_PATH, HISTORY_OBJECT_PATH, USER_OBJECT_PATH,
};
use cla_dbus::structures::{ChatList, HistoryList, Question, Response};

/// Client that communicates with the clad daemon over D-Bus.
pub struct DbusClient {
    connection: Connection,
}

impl DbusClient {
    /// Connect to the system D-Bus.
    pub async fn new() -> anyhow::Result<Self> {
        let connection = Connection::system().await?;
        Ok(Self { connection })
    }

    /// Get the user ID for the given effective user ID.
    pub async fn get_user_id(&self, effective_user_id: u32) -> anyhow::Result<String> {
        let reply = self
            .connection
            .call_method(
                Some(CHAT_BUS_NAME),
                USER_OBJECT_PATH,
                Some("com.redhat.lightspeed.user"),
                "GetUserId",
                &(effective_user_id,),
            )
            .await?;
        Ok(reply.body().deserialize()?)
    }

    /// Ask a question to the LLM.
    pub async fn ask_question(
        &self,
        user_id: &str,
        question: Question,
    ) -> anyhow::Result<Response> {
        let reply = self
            .connection
            .call_method(
                Some(CHAT_BUS_NAME),
                CHAT_OBJECT_PATH,
                Some("com.redhat.lightspeed.chat"),
                "AskQuestion",
                &(user_id, question),
            )
            .await?;
        Ok(reply.body().deserialize()?)
    }

    /// Create a new chat session.
    pub async fn create_chat(
        &self,
        user_id: &str,
        name: &str,
        description: &str,
    ) -> anyhow::Result<String> {
        let reply = self
            .connection
            .call_method(
                Some(CHAT_BUS_NAME),
                CHAT_OBJECT_PATH,
                Some("com.redhat.lightspeed.chat"),
                "CreateChat",
                &(user_id, name, description),
            )
            .await?;
        Ok(reply.body().deserialize()?)
    }

    /// Get chat ID by name.
    pub async fn get_chat_id(&self, user_id: &str, name: &str) -> anyhow::Result<String> {
        let reply = self
            .connection
            .call_method(
                Some(CHAT_BUS_NAME),
                CHAT_OBJECT_PATH,
                Some("com.redhat.lightspeed.chat"),
                "GetChatId",
                &(user_id, name),
            )
            .await?;
        Ok(reply.body().deserialize()?)
    }

    /// Check if a chat is available.
    pub async fn is_chat_available(&self, user_id: &str, name: &str) -> anyhow::Result<bool> {
        let reply = self
            .connection
            .call_method(
                Some(CHAT_BUS_NAME),
                CHAT_OBJECT_PATH,
                Some("com.redhat.lightspeed.chat"),
                "IsChatAvailable",
                &(user_id, name),
            )
            .await?;
        Ok(reply.body().deserialize()?)
    }

    /// List all chats for a user.
    pub async fn get_all_chats(&self, user_id: &str) -> anyhow::Result<ChatList> {
        let reply = self
            .connection
            .call_method(
                Some(CHAT_BUS_NAME),
                CHAT_OBJECT_PATH,
                Some("com.redhat.lightspeed.chat"),
                "GetAllChatFromUser",
                &(user_id,),
            )
            .await?;
        Ok(reply.body().deserialize()?)
    }

    /// Delete a specific chat.
    pub async fn delete_chat(&self, user_id: &str, name: &str) -> anyhow::Result<()> {
        self.connection
            .call_method(
                Some(CHAT_BUS_NAME),
                CHAT_OBJECT_PATH,
                Some("com.redhat.lightspeed.chat"),
                "DeleteChatForUser",
                &(user_id, name),
            )
            .await?;
        Ok(())
    }

    /// Delete all chats.
    pub async fn delete_all_chats(&self, user_id: &str) -> anyhow::Result<()> {
        self.connection
            .call_method(
                Some(CHAT_BUS_NAME),
                CHAT_OBJECT_PATH,
                Some("com.redhat.lightspeed.chat"),
                "DeleteAllChatForUser",
                &(user_id,),
            )
            .await?;
        Ok(())
    }

    /// Get all history for a user.
    pub async fn get_history(&self, user_id: &str) -> anyhow::Result<HistoryList> {
        let reply = self
            .connection
            .call_method(
                Some(CHAT_BUS_NAME),
                HISTORY_OBJECT_PATH,
                Some("com.redhat.lightspeed.history"),
                "GetHistory",
                &(user_id,),
            )
            .await?;
        Ok(reply.body().deserialize()?)
    }

    /// Get first conversation from a chat.
    pub async fn get_first_conversation(
        &self,
        user_id: &str,
        from_chat: &str,
    ) -> anyhow::Result<HistoryList> {
        let reply = self
            .connection
            .call_method(
                Some(CHAT_BUS_NAME),
                HISTORY_OBJECT_PATH,
                Some("com.redhat.lightspeed.history"),
                "GetFirstConversation",
                &(user_id, from_chat),
            )
            .await?;
        Ok(reply.body().deserialize()?)
    }

    /// Get last conversation from a chat.
    pub async fn get_last_conversation(
        &self,
        user_id: &str,
        from_chat: &str,
    ) -> anyhow::Result<HistoryList> {
        let reply = self
            .connection
            .call_method(
                Some(CHAT_BUS_NAME),
                HISTORY_OBJECT_PATH,
                Some("com.redhat.lightspeed.history"),
                "GetLastConversation",
                &(user_id, from_chat),
            )
            .await?;
        Ok(reply.body().deserialize()?)
    }

    /// Get filtered conversation from a chat.
    pub async fn get_filtered_conversation(
        &self,
        user_id: &str,
        filter: &str,
        from_chat: &str,
    ) -> anyhow::Result<HistoryList> {
        let reply = self
            .connection
            .call_method(
                Some(CHAT_BUS_NAME),
                HISTORY_OBJECT_PATH,
                Some("com.redhat.lightspeed.history"),
                "GetFilteredConversation",
                &(user_id, filter, from_chat),
            )
            .await?;
        Ok(reply.body().deserialize()?)
    }

    /// Write history entry.
    pub async fn write_history(
        &self,
        chat_id: &str,
        user_id: &str,
        question: &str,
        response: &str,
    ) -> anyhow::Result<()> {
        self.connection
            .call_method(
                Some(CHAT_BUS_NAME),
                HISTORY_OBJECT_PATH,
                Some("com.redhat.lightspeed.history"),
                "WriteHistory",
                &(chat_id, user_id, question, response),
            )
            .await?;
        Ok(())
    }

    /// Clear history for a specific chat.
    pub async fn clear_history(&self, user_id: &str, from_chat: &str) -> anyhow::Result<()> {
        self.connection
            .call_method(
                Some(CHAT_BUS_NAME),
                HISTORY_OBJECT_PATH,
                Some("com.redhat.lightspeed.history"),
                "ClearHistory",
                &(user_id, from_chat),
            )
            .await?;
        Ok(())
    }

    /// Clear all history.
    pub async fn clear_all_history(&self, user_id: &str) -> anyhow::Result<()> {
        self.connection
            .call_method(
                Some(CHAT_BUS_NAME),
                HISTORY_OBJECT_PATH,
                Some("com.redhat.lightspeed.history"),
                "ClearAllHistory",
                &(user_id,),
            )
            .await?;
        Ok(())
    }
}
