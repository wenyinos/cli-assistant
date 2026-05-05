//! LLM query submission via OpenAI-compatible `/v1/chat/completions` endpoint.

use serde_json::{json, Value};
use tracing::{debug, error, warn};

use cla_common::{ClaError, Config};

use super::client;

/// Maximum number of automatic retries on transient failures.
const MAX_RETRIES: u32 = 3;

/// Maps common HTTP status codes to human-readable error messages.
const ERROR_MESSAGES: &[(u16, &str)] = &[
    (400, "Bad request: the server could not understand the request"),
    (401, "Authentication failed: invalid API key"),
    (403, "Forbidden: access denied"),
    (404, "Not found: the requested endpoint does not exist"),
    (429, "Rate limited: too many requests, please try again later"),
    (500, "Internal server error"),
    (502, "Bad gateway"),
    (503, "Service unavailable: the server is temporarily unable to handle the request"),
];

fn message_for_status(code: u16) -> String {
    ERROR_MESSAGES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, msg)| (*msg).to_string())
        .unwrap_or_else(|| format!("HTTP error {}", code))
}

/// Submit a chat completion request to an OpenAI-compatible API.
///
/// Builds the payload from `config.backend` fields (model, prompt, max_tokens,
/// temperature) and the user's message. Returns the assistant's reply text.
pub async fn submit(config: &Config, user_message: &str) -> Result<String, ClaError> {
    let client = client::create_client(config)?;
    let url = config.backend.chat_completions_url();
    let api_key = config.backend.effective_api_key();

    if api_key.is_empty() {
        return Err(ClaError::chat(
            "no API key configured — set `api_key` in config or `CL_API_KEY` env var",
        ));
    }

    let payload = json!({
        "model": config.backend.model,
        "messages": [
            {
                "role": "system",
                "content": config.backend.effective_prompt()
            },
            {
                "role": "user",
                "content": user_message
            }
        ],
        "max_tokens": config.backend.max_tokens,
        "temperature": config.backend.temperature
    });

    let auth_header = format!("Bearer {}", api_key);
    let mut last_err: Option<String> = None;

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            let backoff_ms = 200 * 2u64.pow(attempt - 1);
            warn!(
                "Retrying request (attempt {}/{}) after {}ms",
                attempt, MAX_RETRIES, backoff_ms
            );
            tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
        }

        debug!("POST {} (attempt {})", url, attempt + 1);

        let response = match client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", &auth_header)
            .json(&payload)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("Request failed: {}", e);
                error!("{}", msg);
                last_err = Some(msg);
                continue;
            }
        };

        let status = response.status();

        if status.is_success() {
            let body: Value = response
                .json()
                .await
                .map_err(|e| ClaError::chat_with_source("failed to parse response body", e))?;

            // OpenAI response format: choices[0].message.content
            let content = body
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str());

            if let Some(text) = content {
                return Ok(text.to_string());
            }

            // Fallback: try legacy `data.text` format (RHEL Lightspeed original)
            if let Some(text) = body.get("data").and_then(|d| d.get("text")).and_then(|t| t.as_str())
            {
                return Ok(text.to_string());
            }

            return Err(ClaError::chat(format!(
                "unexpected response format: {}",
                serde_json::to_string_pretty(&body).unwrap_or_default()
            )));
        }

        let code = status.as_u16();
        let msg = message_for_status(code);
        error!("Backend returned {}: {}", code, msg);

        // Try to extract error detail from response body.
        let detail = response
            .json::<Value>()
            .await
            .ok()
            .and_then(|b| {
                b.get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();

        let full_msg = if detail.is_empty() {
            msg
        } else {
            format!("{}: {}", msg, detail)
        };

        // Only retry on server errors; client errors are final.
        if status.is_server_error() {
            last_err = Some(full_msg);
            continue;
        }

        return Err(ClaError::chat(full_msg));
    }

    Err(ClaError::chat(
        last_err.unwrap_or_else(|| "All retries exhausted".into()),
    ))
}
