//! Configuration loading and schema definitions.
//!
//! Loads TOML configuration from `/etc/cli-assistant/config.toml` with sensible
//! defaults for every field. Supports OpenAI-compatible API endpoints.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::constants::DEFAULT_CONFIG_PATH;
use crate::errors::{ClaError, Result};

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
/// Supports any OpenAI-compatible API. The `endpoint` should include the
/// version path (e.g. `https://api.openai.com/v1`); the client appends
/// `/chat/completions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BackendSchema {
    /// Base URL of the OpenAI-compatible API endpoint.
    /// The client appends `/chat/completions` automatically.
    /// Examples: `https://api.openai.com/v1`, `https://my-proxy.example.com/v1`
    pub endpoint: String,
    /// Model name to use (e.g. `deepseek-v4-flash`, `gpt-4o`).
    pub model: String,
    /// API key for authentication (`Bearer` token).
    /// Can also be set via `CL_API_KEY` environment variable (env takes precedence).
    pub api_key: String,
    /// System prompt prepended to every conversation.
    pub prompt: String,
    /// Maximum tokens in the response.
    pub max_tokens: u32,
    /// Model context window in tokens. The conversation pages
    /// (`chat --interactive` / `--tui`) use it to decide when to compact
    /// history into a summary.
    pub context_length: u32,
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
            endpoint: String::from("https://api.deepseek.com/v1"),
            model: String::from("deepseek-v4-flash"),
            api_key: String::new(),
            prompt: String::from(
                "You are a command-line assistant for Linux system administration. \
                 Answer concisely and accurately, and prefer standard, widely available tools. \
                 Keep commands copy-pasteable; before any destructive or irreversible step, \
                 explain what it does and call out the risk. If a request is ambiguous, \
                 state your assumption briefly and answer the most likely intent.",
            ),
            max_tokens: 32768,
            context_length: 256000,
            temperature: 0.3,
            language: String::new(),
            auth: AuthSchema::default(),
            timeout: 120,
            proxies: BTreeMap::new(),
        }
    }
}

impl BackendSchema {
    /// Resolve the effective API key: config value, then `CL_API_KEY` env var.
    pub fn effective_api_key(&self) -> &str {
        // Env var takes precedence if set and non-empty.
        static ENV_KEY: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
        let env =
            ENV_KEY.get_or_init(|| std::env::var("CL_API_KEY").ok().filter(|s| !s.is_empty()));
        env.as_deref().unwrap_or(&self.api_key)
    }

    /// The full chat completions endpoint URL.
    pub fn chat_completions_url(&self) -> String {
        let base = self.endpoint.trim_end_matches('/');
        format!("{}/chat/completions", base)
    }

    /// The model listing endpoint URL, used by the setup wizard.
    pub fn models_url(&self) -> String {
        let base = self.endpoint.trim_end_matches('/');
        format!("{}/models", base)
    }

    /// Render `template` with the wizard-managed backend values substituted in.
    ///
    /// Only `endpoint`, `model`, `api_key` and `language` inside the
    /// `[backend]` section are replaced; every other line — comments, and any
    /// values the user edited by hand — is copied verbatim. Values are emitted
    /// as escaped TOML string literals, so arbitrary API keys cannot break the
    /// generated file.
    pub fn render_into_template(&self, template: &str) -> String {
        let replacements = [
            ("endpoint", self.endpoint.as_str()),
            ("api_key", self.api_key.as_str()),
            ("model", self.model.as_str()),
            ("language", self.language.as_str()),
        ];

        let mut out = String::with_capacity(template.len() + 128);
        let mut section = String::new();

        for line in template.lines() {
            let trimmed = line.trim_start();

            if trimmed.starts_with('[') {
                section = trimmed
                    .trim_start_matches('[')
                    .split(']')
                    .next()
                    .unwrap_or_default()
                    .to_string();
            } else if section == "backend" && !trimmed.starts_with('#') {
                if let Some((key, _)) = trimmed.split_once('=') {
                    let key = key.trim();
                    if let Some((_, value)) = replacements.iter().find(|(k, _)| *k == key) {
                        let indent = &line[..line.len() - trimmed.len()];
                        let literal = toml::Value::String(value.to_string()).to_string();
                        out.push_str(&format!("{}{} = {}\n", indent, key, literal));
                        continue;
                    }
                }
            }

            out.push_str(line);
            out.push('\n');
        }

        out
    }

    /// Build the effective system prompt, appending language instruction if configured.
    pub fn effective_prompt(&self) -> String {
        if self.language.is_empty() {
            return self.prompt.clone();
        }
        format!("{}\n\nAlways reply in {}.", self.prompt, self.language)
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub backend: BackendSchema,
    pub database: DatabaseSchema,
    pub history: HistorySchema,
    pub logging: LoggingSchema,
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

    /// Loads configuration from `/etc/cli-assistant/config.toml`.
    ///
    /// This fixed path is used on all distributions; XDG config variables are
    /// deliberately not consulted so behaviour is identical everywhere. Falls
    /// back to default values if the file is missing.
    pub fn load() -> Result<Self> {
        Self::load_from_path(std::path::Path::new(DEFAULT_CONFIG_PATH))
    }
}

// ---------------------------------------------------------------------------
// Config template rendering (setup wizard)
// ---------------------------------------------------------------------------

/// Embedded configuration template. The single source of truth is the
/// repository file `config/config.toml`, so generated files keep the
/// documented defaults and all of their comments.
const CONFIG_TEMPLATE: &str = include_str!("../../../config/config.toml");

/// The built-in configuration template, used when no config file exists yet.
pub fn default_config_template() -> &'static str {
    CONFIG_TEMPLATE
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
        assert_eq!(config.backend.timeout, 120); // default
        assert!(!config.history.enabled);
    }

    #[test]
    fn load_from_missing_path_returns_default() {
        let path = PathBuf::from("/nonexistent/path/config.toml");
        let config = AppConfig::load_from_path(&path).expect("load");
        assert_eq!(config.backend.endpoint, "https://api.deepseek.com/v1");
        assert_eq!(config.backend.model, "deepseek-v4-flash");
    }

    #[test]
    fn chat_completions_url() {
        // The endpoint already includes the version path; only /chat/completions
        // is appended, and a trailing slash is trimmed.
        let mut backend = BackendSchema {
            endpoint: "https://api.openai.com/v1".to_string(),
            ..Default::default()
        };
        assert_eq!(
            backend.chat_completions_url(),
            "https://api.openai.com/v1/chat/completions"
        );

        backend.endpoint = "https://api.openai.com/v1/".to_string();
        assert_eq!(
            backend.chat_completions_url(),
            "https://api.openai.com/v1/chat/completions"
        );

        backend.endpoint = "https://my-proxy.example.com/v2".to_string();
        assert_eq!(
            backend.chat_completions_url(),
            "https://my-proxy.example.com/v2/chat/completions"
        );
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

    #[test]
    fn models_url() {
        let mut backend = BackendSchema {
            endpoint: "https://api.openai.com/v1".to_string(),
            ..Default::default()
        };
        assert_eq!(backend.models_url(), "https://api.openai.com/v1/models");

        backend.endpoint = "https://my-proxy.example.com/v2/".to_string();
        assert_eq!(
            backend.models_url(),
            "https://my-proxy.example.com/v2/models"
        );
    }

    /// A backend carrying only the fields the setup wizard manages.
    fn wizard_values(endpoint: &str, api_key: &str, model: &str, language: &str) -> BackendSchema {
        BackendSchema {
            endpoint: endpoint.to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            language: language.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn rendered_template_parses_and_keeps_values() {
        let values = wizard_values(
            "https://api.deepseek.com/v1",
            "sk-secret",
            "deepseek-chat",
            "zh-CN",
        );
        let rendered = values.render_into_template(default_config_template());

        let config: AppConfig = toml::from_str(&rendered).expect("rendered config must parse");
        assert_eq!(config.backend.endpoint, "https://api.deepseek.com/v1");
        assert_eq!(config.backend.api_key, "sk-secret");
        assert_eq!(config.backend.model, "deepseek-chat");
        assert_eq!(config.backend.language, "zh-CN");

        // Fields the wizard does not ask about keep their template defaults.
        assert_eq!(config.backend.max_tokens, 32768);
        assert_eq!(config.backend.context_length, 256000);
        assert!((config.backend.temperature - 0.3).abs() < f32::EPSILON);
        assert_eq!(config.backend.timeout, 120);
        assert_eq!(
            config.database.path,
            PathBuf::from("/var/lib/cli-assistant/cla.db")
        );
        assert!(config.history.enabled);

        // Comments survive rendering.
        assert!(rendered.contains("# Base URL of the OpenAI-compatible API"));
        assert!(rendered.contains("# ── Database (SQLite)"));
    }

    #[test]
    fn rendered_template_escapes_hostile_values() {
        let values = wizard_values(
            "https://example.com/v1",
            "sk-\"quoted\"\\back\\slash",
            "m\"odel",
            "",
        );
        let rendered = values.render_into_template(default_config_template());

        let config: AppConfig = toml::from_str(&rendered).expect("rendered config must parse");
        assert_eq!(config.backend.api_key, "sk-\"quoted\"\\back\\slash");
        assert_eq!(config.backend.model, "m\"odel");
        assert_eq!(config.backend.language, "");
    }

    #[test]
    fn render_into_template_preserves_hand_edited_values() {
        // Re-running the wizard must not reset values the user edited outside it.
        let template = "# my config\n\
                        [backend]\n\
                        endpoint = \"https://old.example.com/v1\"\n\
                        model = \"old-model\"\n\
                        api_key = \"\"\n\
                        max_tokens = 64000\n\
                        language = \"\"\n\
                        \n\
                        [database]\n\
                        path = \"/tmp/custom.db\"\n";

        let values = wizard_values("https://new.example.com/v1", "sk-new", "new-model", "zh-CN");
        let rendered = values.render_into_template(template);

        let config: AppConfig = toml::from_str(&rendered).expect("rendered config must parse");
        assert_eq!(config.backend.endpoint, "https://new.example.com/v1");
        assert_eq!(config.backend.api_key, "sk-new");
        assert_eq!(config.backend.model, "new-model");
        assert_eq!(config.backend.language, "zh-CN");
        assert_eq!(config.backend.max_tokens, 64000);
        assert_eq!(config.database.path, PathBuf::from("/tmp/custom.db"));
        assert!(rendered.contains("# my config"));
    }
}
