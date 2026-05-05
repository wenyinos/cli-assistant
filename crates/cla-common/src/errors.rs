//! Error types for the command-line assistant.
//!
//! Maps to Python's exception hierarchy with exit codes for each command domain.

use std::path::PathBuf;

/// Base error type for all CLI assistant operations.
#[derive(Debug, thiserror::Error)]
pub enum ClaError {
    /// Chat command failed.
    #[error("chat command error: {message}")]
    ChatCommand {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Shell command failed.
    #[error("shell command error: {message}")]
    ShellCommand {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// History command failed.
    #[error("history command error: {message}")]
    HistoryCommand {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Feedback command failed.
    #[error("feedback command error: {message}")]
    FeedbackCommand {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Configuration error.
    #[error("configuration error: {message}")]
    Config {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Session management error.
    #[error("session error: {message}")]
    Session {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// I/O error.
    #[error("I/O error at {path:?}: {source}")]
    Io {
        path: Option<PathBuf>,
        #[source]
        source: std::io::Error,
    },

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// TOML parsing error.
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// UUID parsing error.
    #[error("UUID parse error: {0}")]
    UuidParse(#[from] uuid::Error),
}

impl ClaError {
    /// Returns the exit code associated with this error variant.
    ///
    /// Maps to Python exception codes: 80-83 for domain-specific errors,
    /// 1 for generic errors.
    pub fn exit_code(&self) -> i32 {
        match self {
            ClaError::ChatCommand { .. } => 80,
            ClaError::ShellCommand { .. } => 81,
            ClaError::HistoryCommand { .. } => 82,
            ClaError::FeedbackCommand { .. } => 83,
            _ => 1,
        }
    }

    // -- Convenience constructors --

    pub fn chat(message: impl Into<String>) -> Self {
        ClaError::ChatCommand {
            message: message.into(),
            source: None,
        }
    }

    pub fn chat_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        ClaError::ChatCommand {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn shell(message: impl Into<String>) -> Self {
        ClaError::ShellCommand {
            message: message.into(),
            source: None,
        }
    }

    pub fn shell_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        ClaError::ShellCommand {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn history(message: impl Into<String>) -> Self {
        ClaError::HistoryCommand {
            message: message.into(),
            source: None,
        }
    }

    pub fn history_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        ClaError::HistoryCommand {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn feedback(message: impl Into<String>) -> Self {
        ClaError::FeedbackCommand {
            message: message.into(),
            source: None,
        }
    }

    pub fn feedback_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        ClaError::FeedbackCommand {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        ClaError::Config {
            message: message.into(),
            source: None,
        }
    }

    pub fn config_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        ClaError::Config {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn session(message: impl Into<String>) -> Self {
        ClaError::Session {
            message: message.into(),
            source: None,
        }
    }

    pub fn session_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        ClaError::Session {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn io(path: Option<PathBuf>, source: std::io::Error) -> Self {
        ClaError::Io { path, source }
    }
}

/// Convenient `Result` alias for crate operations.
pub type Result<T> = std::result::Result<T, ClaError>;

impl From<std::io::Error> for ClaError {
    fn from(source: std::io::Error) -> Self {
        ClaError::Io { path: None, source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_match_python() {
        assert_eq!(ClaError::chat("test").exit_code(), 80);
        assert_eq!(ClaError::shell("test").exit_code(), 81);
        assert_eq!(ClaError::history("test").exit_code(), 82);
        assert_eq!(ClaError::feedback("test").exit_code(), 83);
        assert_eq!(ClaError::config("test").exit_code(), 1);
        assert_eq!(ClaError::session("test").exit_code(), 1);
    }

    #[test]
    fn error_display() {
        let err = ClaError::chat("connection refused");
        assert!(err.to_string().contains("connection refused"));
    }
}
