//! First-time configuration wizard (`c setup`).
//!
//! Writes `/etc/cli-assistant/config.toml` from the embedded template so the
//! generated file keeps all of its comments. The wizard talks to the backend
//! directly (rather than through the daemon) because it must work before any
//! configuration — and therefore any usable daemon — exists.

use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use cla_common::config::{default_config_template, AppConfig, BackendSchema};
use cla_common::constants::DEFAULT_CONFIG_PATH;
use cla_common::files;
use nix::unistd::geteuid;

use crate::rendering::Renderer;

/// Maximum number of models shown in the selection list.
const MAX_MODELS_SHOWN: usize = 30;

/// Timeout for the `{endpoint}/models` probe.
const MODELS_TIMEOUT: Duration = Duration::from_secs(15);

/// Guard for commands that need a working backend. Returns an exit code when
/// the client must stop, `None` when it may proceed.
pub fn check_configured(renderer: &Renderer) -> Option<i32> {
    // CL_API_KEY satisfies the API-key requirement on its own.
    if env_key_is_set() {
        return None;
    }

    if !Path::new(DEFAULT_CONFIG_PATH).exists() {
        renderer.warning(&format!(
            "No configuration found at {}.",
            DEFAULT_CONFIG_PATH
        ));
        renderer.normal("Run `sudo c setup` to configure the endpoint, API key and model.");
        return Some(1);
    }

    match AppConfig::load() {
        Ok(config) if !config.backend.effective_api_key().is_empty() => None,
        Ok(_) => {
            renderer.warning("No API key configured.");
            renderer.normal("Run `sudo c setup` to finish setup, or set CL_API_KEY.");
            Some(1)
        }
        Err(e) => {
            renderer.error(&format!("Failed to parse {}: {}", DEFAULT_CONFIG_PATH, e));
            renderer.normal("Fix the file or re-run `sudo c setup`.");
            Some(1)
        }
    }
}

/// Wizard entry point. Returns the process exit code (0 ok, 1 error, 2 cancelled).
pub async fn run(renderer: &Renderer) -> i32 {
    if !geteuid().is_root() {
        renderer.error(&format!(
            "The wizard writes {} and must run as root.",
            DEFAULT_CONFIG_PATH
        ));
        renderer.normal("Run it with: sudo c setup");
        return 1;
    }

    let path = Path::new(DEFAULT_CONFIG_PATH);

    // Existing values become the prompt defaults, so re-running edits in place.
    let loaded = AppConfig::load();
    let existing_is_usable = loaded.is_ok();
    let existing = loaded.unwrap_or_default();
    let env_key_set = env_key_is_set();

    renderer.normal("cli-assistant setup — press Enter to accept the value in brackets.");
    renderer.normal("");

    let Some(endpoint) = ask_validated(
        "Endpoint",
        &existing.backend.endpoint,
        "Include the version path, e.g. https://api.openai.com/v1",
        "The endpoint must start with http:// or https://",
        |v| v.starts_with("http://") || v.starts_with("https://"),
    ) else {
        return cancelled(renderer);
    };

    // When CL_API_KEY is set we deliberately leave the file value empty: the
    // environment variable takes precedence anyway, and storing a second copy
    // would only go stale.
    let key_default = if env_key_set {
        String::new()
    } else {
        existing.backend.api_key.clone()
    };
    let Some(api_key) = ask_secret("API Key", &key_default, env_key_set) else {
        return cancelled(renderer);
    };

    renderer.info(&format!(
        "Querying {}/models ...",
        endpoint.trim_end_matches('/')
    ));
    let models = match fetch_models(&endpoint, &api_key).await {
        Ok(list) => {
            for (i, id) in list.iter().take(MAX_MODELS_SHOWN).enumerate() {
                renderer.normal(&format!("  [{}] {}", i + 1, id));
            }
            if list.len() > MAX_MODELS_SHOWN {
                renderer.normal(&format!("  ... and {} more", list.len() - MAX_MODELS_SHOWN));
            }
            list
        }
        Err(e) => {
            renderer.warning(&format!("Could not fetch the model list: {}", e));
            renderer.normal("Enter the model name manually.");
            Vec::new()
        }
    };

    // Prefer the configured model when the endpoint still offers it, otherwise
    // default to the first model the endpoint reported.
    let default_model = if models.is_empty() || models.contains(&existing.backend.model) {
        existing.backend.model.clone()
    } else {
        models[0].clone()
    };
    let Some(answer) = ask("Model", &default_model, None) else {
        return cancelled(renderer);
    };
    let model = select_model(&answer, &models, &default_model);

    let Some(language) = ask(
        "Reply language",
        &existing.backend.language,
        Some("e.g. zh-CN, en, ja — leave empty to let the model decide"),
    ) else {
        return cancelled(renderer);
    };

    renderer.normal("");
    renderer.normal(&format!("About to write {}", DEFAULT_CONFIG_PATH));
    renderer.normal(&format!("  endpoint : {}", endpoint));
    renderer.normal(&format!("  api_key  : {}", mask_key(&api_key, env_key_set)));
    renderer.normal(&format!("  model    : {}", model));
    renderer.normal(&format!(
        "  language : {}",
        if language.is_empty() {
            "(model default)"
        } else {
            language.as_str()
        }
    ));

    match confirm("Write and restart clad?") {
        None => return cancelled(renderer),
        Some(false) => {
            renderer.warning("Setup cancelled; nothing was written.");
            return 2;
        }
        Some(true) => {}
    }

    if let Some(parent) = path.parent() {
        if let Err(e) = files::create_folder(parent, true, 0o755) {
            renderer.error(&format!("Failed to create {}: {}", parent.display(), e));
            return 1;
        }
    }

    if path.exists() {
        let backup = format!("{}.bak", DEFAULT_CONFIG_PATH);
        match std::fs::copy(path, &backup) {
            Ok(_) => renderer.info(&format!("Previous configuration backed up to {}", backup)),
            Err(e) => renderer.warning(&format!("Could not back up the previous file: {}", e)),
        }
    }

    // Reuse the current file as the template when it parses, so values edited
    // by hand (max_tokens, prompt, ...) survive a re-run. On a first run, or
    // when the existing file is unusable, fall back to the embedded template.
    let template = if existing_is_usable && path.exists() {
        std::fs::read_to_string(path).unwrap_or_else(|_| default_config_template().to_string())
    } else {
        default_config_template().to_string()
    };
    let values = BackendSchema {
        endpoint,
        api_key,
        model,
        language,
        ..Default::default()
    };
    let rendered = values.render_into_template(&template);

    if let Err(e) = files::write_file(rendered.as_bytes(), path, 0o644) {
        renderer.error(&format!("Failed to write {}: {}", DEFAULT_CONFIG_PATH, e));
        return 1;
    }
    renderer.normal(&format!("Configuration written to {}", DEFAULT_CONFIG_PATH));

    renderer.info("Restarting clad.service ...");
    match Command::new("systemctl").args(["restart", "clad"]).output() {
        Ok(output) if output.status.success() => {
            renderer.normal("clad restarted. Try: c \"how do I check disk space?\"");
            0
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            renderer.error(&format!("Failed to restart clad: {}", stderr.trim()));
            renderer.normal("Restart it manually: sudo systemctl restart clad");
            1
        }
        Err(e) => {
            renderer.error(&format!("Could not run systemctl: {}", e));
            renderer.normal("Restart the daemon manually: sudo systemctl restart clad");
            1
        }
    }
}

/// Whether a non-empty `CL_API_KEY` is present in the environment.
fn env_key_is_set() -> bool {
    std::env::var("CL_API_KEY")
        .map(|k| !k.is_empty())
        .unwrap_or(false)
}

fn cancelled(renderer: &Renderer) -> i32 {
    renderer.warning("Input ended; setup cancelled.");
    2
}

/// Print a prompt with a default in brackets. Returns the trimmed answer (the
/// default when empty), or `None` on EOF.
fn ask(label: &str, default: &str, hint: Option<&str>) -> Option<String> {
    if let Some(hint) = hint {
        eprintln!("  {}", hint);
    }
    if default.is_empty() {
        eprint!("{}: ", label);
    } else {
        eprint!("{} [{}]: ", label, default);
    }
    let _ = io::stdout().flush();

    let answer = read_line()?;
    Some(if answer.is_empty() {
        default.to_string()
    } else {
        answer
    })
}

/// Like [`ask`], but re-prompts until the value passes `validate`.
fn ask_validated(
    label: &str,
    default: &str,
    hint: &str,
    invalid_message: &str,
    validate: impl Fn(&str) -> bool,
) -> Option<String> {
    eprintln!("  {}", hint);
    loop {
        let answer = ask(label, default, None)?;
        if validate(&answer) {
            return Some(answer);
        }
        eprintln!("  {}", invalid_message);
    }
}

/// Ask for the API key with terminal echo disabled.
fn ask_secret(label: &str, default: &str, env_fallback: bool) -> Option<String> {
    if env_fallback {
        eprintln!("  CL_API_KEY is set and takes precedence — press Enter to keep using it");
    } else if !default.is_empty() {
        eprintln!("  Current key is kept when left empty");
    }
    eprint!("{}: ", label);
    let _ = io::stdout().flush();

    let answer = read_line_hidden()?;
    Some(if answer.is_empty() {
        default.to_string()
    } else {
        answer
    })
}

/// Yes/no confirmation. An empty answer means yes; `None` on EOF.
fn confirm(question: &str) -> Option<bool> {
    eprint!("{} [Y/n]: ", question);
    let _ = io::stdout().flush();

    let answer = read_line()?;
    Some(matches!(answer.to_lowercase().as_str(), "" | "y" | "yes"))
}

/// Read one line from stdin, trimmed. `None` on EOF or read error.
fn read_line() -> Option<String> {
    let mut buffer = String::new();
    match io::stdin().read_line(&mut buffer) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(buffer.trim().to_string()),
    }
}

/// Read one line with echo disabled when stdin is a terminal.
fn read_line_hidden() -> Option<String> {
    use nix::sys::termios::{tcgetattr, tcsetattr, LocalFlags, SetArg};
    use std::os::fd::AsFd;

    let stdin = io::stdin();
    if !stdin.is_terminal() {
        return read_line();
    }

    let Ok(original) = tcgetattr(stdin.as_fd()) else {
        return read_line();
    };
    let mut no_echo = original.clone();
    no_echo.local_flags.remove(LocalFlags::ECHO);
    if tcsetattr(stdin.as_fd(), SetArg::TCSANOW, &no_echo).is_err() {
        return read_line();
    }

    let answer = read_line();

    // Restore echo before anything else can fail.
    let _ = tcsetattr(stdin.as_fd(), SetArg::TCSANOW, &original);
    println!();
    answer
}

/// Interpret the model answer: a 1-based index into the fetched list, or a
/// literal model name. An empty answer returns the default.
fn select_model(answer: &str, models: &[String], default: &str) -> String {
    if answer.is_empty() {
        return default.to_string();
    }
    if let Ok(index) = answer.parse::<usize>() {
        if index >= 1 && index <= models.len() {
            return models[index - 1].clone();
        }
    }
    answer.to_string()
}

/// Mask an API key for display: first and last four characters only.
fn mask_key(key: &str, env_fallback: bool) -> String {
    if key.is_empty() {
        return if env_fallback {
            "(from CL_API_KEY)".to_string()
        } else {
            "(empty)".to_string()
        };
    }
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 8 {
        return "********".to_string();
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{}…{}", head, tail)
}

/// Fetch model ids from `{endpoint}/models`. Returns a human-readable error
/// message on failure so the wizard can fall back to manual entry.
async fn fetch_models(endpoint: &str, api_key: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/models", endpoint.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(MODELS_TIMEOUT)
        .build()
        .map_err(|e| format!("failed to create HTTP client: {}", e))?;

    let mut request = client.get(&url).header("Accept", "application/json");
    if !api_key.is_empty() {
        request = request.bearer_auth(api_key);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("invalid JSON: {}", e))?;

    let models = parse_model_ids(&body);
    if models.is_empty() {
        return Err("the response contained no model ids".to_string());
    }
    Ok(models)
}

/// Extract model ids from an OpenAI-style `{"data": [{"id": ...}]}` payload,
/// accepting a bare `{"models": [...]}` shape as a fallback.
fn parse_model_ids(body: &serde_json::Value) -> Vec<String> {
    let entries = body
        .get("data")
        .and_then(|v| v.as_array())
        .or_else(|| body.get("models").and_then(|v| v.as_array()));

    let Some(entries) = entries else {
        return Vec::new();
    };

    entries
        .iter()
        .filter_map(|entry| {
            entry
                .get("id")
                .and_then(|v| v.as_str())
                .or_else(|| entry.as_str())
                .map(str::to_string)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn models() -> Vec<String> {
        vec!["gpt-4o".to_string(), "gpt-4o-mini".to_string()]
    }

    #[test]
    fn select_model_accepts_index_and_name() {
        assert_eq!(select_model("2", &models(), "gpt-4o"), "gpt-4o-mini");
        assert_eq!(select_model("llama3", &models(), "gpt-4o"), "llama3");
        // Out-of-range numbers are treated as literal model names.
        assert_eq!(select_model("9", &models(), "gpt-4o"), "9");
        assert_eq!(select_model("", &models(), "gpt-4o"), "gpt-4o");
    }

    #[test]
    fn parse_model_ids_reads_openai_shapes() {
        let openai = json!({"data": [{"id": "a"}, {"id": "b"}]});
        assert_eq!(parse_model_ids(&openai), vec!["a", "b"]);

        let bare = json!({"models": ["c", {"id": "d"}]});
        assert_eq!(parse_model_ids(&bare), vec!["c", "d"]);

        assert!(parse_model_ids(&json!({"unexpected": true})).is_empty());
    }

    #[test]
    fn mask_key_hides_the_middle() {
        assert_eq!(mask_key("", false), "(empty)");
        assert_eq!(mask_key("", true), "(from CL_API_KEY)");
        assert_eq!(mask_key("short", false), "********");
        assert_eq!(mask_key("sk-1234567890abcd", false), "sk-1…abcd");
    }
}
