//! Configuration loading and schema definitions.
//!
//! Loads TOML configuration from `/etc/cli-assistant/config.toml` with sensible
//! defaults for every field. Supports OpenAI-compatible API endpoints.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::constants::DEFAULT_CONFIG_PATH;
use crate::errors::{ClaError, Result};
use crate::environment::get_xdg_config_path;

// ---------------------------------------------------------------------------
// Schema types
// ---------------------------------------------------------------------------

/// TLS authentication for the backend endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthSchema {
    /// Client certificate PEM file.
    pub cert_file: PathBuf,
    /// Client private key PEM file.
    pub key_file: PathBuf,
}

impl Default for AuthSchema {
    fn default() -> Self {
        Self {
            cert_file: PathBuf::from("/etc/pki/consumer/cert.pem"),
            key_file: PathBuf::from("/etc/pki/consumer/key.pem"),
        }
    }
}

/// Backend service connection settings.
///
/// Supports any OpenAI-compatible API. The `endpoint` should be the base URL
/// (e.g. `https://api.openai.com`); the client appends `/v1/chat/completions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BackendSchema {
    /// Base URL of the OpenAI-compatible API endpoint.
    /// The client appends `/v1/chat/completions` automatically.
    pub endpoint: String,
    /// Model name to use (e.g. `gpt-4`, `gpt-3.5-turbo`).
    pub model: String,
    /// API key for authentication (`Bearer` token).
    /// Can also be set via `CL_API_KEY` environment variable (env takes precedence).
    pub api_key: String,
    /// System prompt prepended to every conversation.
    pub prompt: String,
    /// Maximum tokens in the response.
    pub max_tokens: u32,
    /// Sampling temperature (0.0–2.0). Higher = more random.
    pub temperature: f32,
    /// Default response language (e.g. `"zh-CN"`, `"en"`, `"ja"`).
    /// If non-empty, the system prompt will include an instruction to reply
    /// in this language. Leave empty to let the model decide.
    pub language: String,
    /// TLS authentication settings (for mTLS with RHEL Lightspeed backend).
    pub auth: AuthSchema,
    /// Request timeout in seconds.
    pub timeout: u64,
    /// Proxy configuration (protocol → URL).
    pub proxies: BTreeMap<String, String>,
}

impl Default for BackendSchema {
    fn default() -> Self {
        Self {
            endpoint: String::from("https://api.openai.com"),
            model: String::from("gpt-4"),
            api_key: String::new(),
            prompt: String::from("You are a helpful assistant for Linux system administration."),
            max_tokens: 4096,
            temperature: 0.7,
            language: String::new(),
            auth: AuthSchema::default(),
            timeout: 60,
            proxies: BTreeMap::new(),
        }
    }
}

impl BackendSchema {
    /// Resolve the effective API key: config value, then `CL_API_KEY` env var.
    pub fn effective_api_key(&self) -> &str {
        // Env var takes precedence if set and non-empty.
        static ENV_KEY: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
        let env = ENV_KEY.get_or_init(|| std::env::var("CL_API_KEY").ok().filter(|s| !s.is_empty()));
        env.as_deref().unwrap_or(&self.api_key)
    }

    /// The full chat completions endpoint URL.
    pub fn chat_completions_url(&self) -> String {
        let base = self.endpoint.trim_end_matches('/');
        format!("{}/v1/chat/completions", base)
    }

    /// Build the effective system prompt, appending language instruction if configured.
    pub fn effective_prompt(&self) -> String {
        if self.language.is_empty() {
            return self.prompt.clone();
        }
        format!(
            "{}\n\nAlways reply in {}.",
            self.prompt, self.language
        )
    }
}

/// Database connection settings (SQLite only).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseSchema {
    /// Path to the SQLite database file.
    pub path: PathBuf,
}

impl Default for DatabaseSchema {
    fn default() -> Self {
        Self {
            path: PathBuf::from("/var/lib/cli-assistant/cla.db"),
        }
    }
}

/// History feature settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HistorySchema {
    /// Whether history recording is enabled.
    pub enabled: bool,
}

impl Default for HistorySchema {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Audit logging sub-config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuditSchema {
    /// Whether audit logging is enabled.
    pub enabled: bool,
}

impl Default for AuditSchema {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingSchema {
    /// Log level (TRACE, DEBUG, INFO, WARN, ERROR).
    pub level: String,
    /// Audit logging settings.
    pub audit: AuditSchema,
}

impl Default for LoggingSchema {
    fn default() -> Self {
        Self {
            level: String::from("INFO"),
            audit: AuditSchema::default(),
        }
    }
}

/// Top-level application configuration.
///
/// Maps directly to the TOML config file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub backend: BackendSchema,
    pub database: DatabaseSchema,
    pub history: HistorySchema,
    pub logging: LoggingSchema,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            backend: BackendSchema::default(),
            database: DatabaseSchema::default(),
            history: HistorySchema::default(),
            logging: LoggingSchema::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

impl AppConfig {
    /// Loads configuration from the specified TOML file path.
    ///
    /// If the file does not exist, returns the default configuration.
    /// Returns an error if the file exists but cannot be parsed.
    pub fn load_from_path(path: &std::path::Path) -> Result<Self> {
        if !path.exists() {
            tracing::debug!("Config file not found at {:?}, using defaults", path);
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(path).map_err(|e| {
            ClaError::config_with_source(
                format!("failed to read config file: {}", path.display()),
                e,
            )
        })?;

        let config: AppConfig = toml::from_str(&contents)?;
        tracing::debug!("Loaded configuration from {:?}", path);
        Ok(config)
    }

    /// Loads configuration from the default config path.
    ///
    /// Search order:
    /// 1. `/etc/cli-assistant/config.toml`
    /// 2. `$XDG_CONFIG_HOME/cli-assistant/config.toml`
    /// 3. Falls back to default values if no file is found.
    pub fn load() -> Result<Self> {
        // Primary: system-wide default
        let primary = PathBuf::from(DEFAULT_CONFIG_PATH);
        if primary.exists() {
            return Self::load_from_path(&primary);
        }

        // Secondary: XDG config home
        let xdg_path = get_xdg_config_path().join("config.toml");
        if xdg_path.exists() {
            return Self::load_from_path(&xdg_path);
        }

        tracing::debug!("No config file found, using defaults");
        Ok(Self::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_roundtrips() {
        let config = AppConfig::default();
        let serialized = toml::to_string(&config).expect("serialize");
        let deserialized: AppConfig = toml::from_str(&serialized).expect("deserialize");
        assert_eq!(config.backend.endpoint, deserialized.backend.endpoint);
        assert_eq!(config.backend.model, deserialized.backend.model);
        assert_eq!(config.backend.max_tokens, deserialized.backend.max_tokens);
        assert_eq!(config.database.path, deserialized.database.path);
        assert_eq!(config.history.enabled, deserialized.history.enabled);
        assert_eq!(config.logging.level, deserialized.logging.level);
    }

    #[test]
    fn partial_config_uses_defaults() {
        let toml = r#"
[backend]
endpoint = "https://my-api.example.com"
model = "gpt-3.5-turbo"
api_key = "sk-test123"
max_tokens = 2048
temperature = 0.3

[history]
enabled = false
"#;
        let config: AppConfig = toml::from_str(toml).expect("parse");
        assert_eq!(config.backend.endpoint, "https://my-api.example.com");
        assert_eq!(config.backend.model, "gpt-3.5-turbo");
        assert_eq!(config.backend.api_key, "sk-test123");
        assert_eq!(config.backend.max_tokens, 2048);
        assert!((config.backend.temperature - 0.3).abs() < f32::EPSILON);
        assert_eq!(config.backend.timeout, 60); // default
        assert!(!config.history.enabled);
    }

    #[test]
    fn load_from_missing_path_returns_default() {
        let path = PathBuf::from("/nonexistent/path/config.toml");
        let config = AppConfig::load_from_path(&path).expect("load");
        assert_eq!(config.backend.endpoint, "https://api.openai.com");
        assert_eq!(config.backend.model, "gpt-4");
    }

    #[test]
    fn chat_completions_url() {
        let mut backend = BackendSchema::default();
        backend.endpoint = "https://api.openai.com/".to_string();
        assert_eq!(backend.chat_completions_url(), "https://api.openai.com/v1/chat/completions");

        backend.endpoint = "https://my-proxy.example.com".to_string();
        assert_eq!(backend.chat_completions_url(), "https://my-proxy.example.com/v1/chat/completions");
    }

    #[test]
    fn auth_schema_defaults() {
        let auth = AuthSchema::default();
        assert_eq!(auth.cert_file, PathBuf::from("/etc/pki/consumer/cert.pem"));
        assert_eq!(auth.key_file, PathBuf::from("/etc/pki/consumer/key.pem"));
    }

    #[test]
    fn database_schema_defaults() {
        let db = DatabaseSchema::default();
        assert_eq!(db.path, PathBuf::from("/var/lib/cli-assistant/cla.db"));
    }
}
