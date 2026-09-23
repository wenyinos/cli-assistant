//! LLM query submission via OpenAI-compatible `/v1/chat/completions` endpoint.

use serde_json::{json, Value};
use tracing::{debug, error, warn};

use cla_common::{ClaError, Config};

use super::client;

/// Maximum number of automatic retries on transient failures.
const MAX_RETRIES: u32 = 3;

/// Maps common HTTP status codes to human-readable error messages.
const ERROR_MESSAGES: &[(u16, &str)] = &[
    (
        400,
        "Bad request: the server could not understand the request",
    ),
    (401, "Authentication failed: invalid API key"),
    (403, "Forbidden: access denied"),
    (404, "Not found: the requested endpoint does not exist"),
    (
        429,
        "Rate limited: too many requests, please try again later",
    ),
    (500, "Internal server error"),
    (502, "Bad gateway"),
    (
        503,
        "Service unavailable: the server is temporarily unable to handle the request",
    ),
];

fn message_for_status(code: u16) -> String {
    ERROR_MESSAGES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, msg)| (*msg).to_string())
        .unwrap_or_else(|| format!("HTTP error {}", code))
}

/// Turns kept verbatim when compacting; everything older is folded into the
/// summary.
const COMPACT_KEEP_TURNS: usize = 6;

/// Share of the history budget that triggers compaction.
const COMPACT_THRESHOLD: f64 = 0.75;

/// Output cap for the summarization request (the instruction asks for ≤600 words).
const COMPACTION_MAX_TOKENS: u32 = 2048;

/// Instructions handed to the model when compacting a conversation.
const COMPACTION_INSTRUCTION: &str = "You are compacting the history of a Linux system administration \
conversation so it can continue in a smaller context. Keep the user's goals, environment details, \
decisions made, commands that worked or failed, and unresolved problems. Drop greetings, repetition, \
and the full text of long command outputs. Reply in the same language as the conversation, write at \
most 600 words, and output only the summary.";

/// Approximate the token count of `text` without a tokenizer: ASCII characters
/// count roughly four per token, non-ASCII (e.g. CJK) one per token. The
/// estimate is deliberately conservative, so compaction runs early rather than
/// after the API rejects an oversized request.
pub fn estimate_tokens(text: &str) -> u64 {
    let total = text.chars().count() as u64;
    let ascii = text.chars().filter(char::is_ascii).count() as u64;
    ascii / 4 + (total - ascii)
}

/// Build the OpenAI-compatible chat completion payload, injecting the summary
/// and the retained turns (oldest first) ahead of the current question.
fn build_payload(
    config: &Config,
    user_message: &str,
    summary: Option<&str>,
    turns: &[(String, String)],
) -> Value {
    let mut system = config.backend.effective_prompt();
    if let Some(summary) = summary.filter(|s| !s.trim().is_empty()) {
        system.push_str("\n\n[Summary of earlier conversation]\n");
        system.push_str(summary);
    }

    let mut messages = vec![json!({"role": "system", "content": system})];
    for (question, response) in turns {
        messages.push(json!({"role": "user", "content": question}));
        messages.push(json!({"role": "assistant", "content": response}));
    }
    messages.push(json!({"role": "user", "content": user_message}));

    json!({
        "model": config.backend.model,
        "messages": messages,
        "max_tokens": config.backend.max_tokens,
        "temperature": config.backend.temperature
    })
}

/// How many leading turns should be folded into the summary before sending the
/// next question. `None` while the history still fits the budget or there is
/// nothing left to fold.
pub fn compaction_point(
    config: &Config,
    user_message: &str,
    summary: Option<&str>,
    turns: &[(String, String)],
) -> Option<usize> {
    if turns.len() <= COMPACT_KEEP_TURNS {
        return None;
    }

    let reserved = estimate_tokens(&config.backend.effective_prompt())
        + estimate_tokens(user_message)
        + summary.map(estimate_tokens).unwrap_or(0);
    let budget = (config.backend.context_length as u64)
        .saturating_sub(config.backend.max_tokens as u64)
        .saturating_sub(reserved);

    let used: u64 = turns
        .iter()
        .map(|(question, response)| estimate_tokens(question) + estimate_tokens(response))
        .sum();

    if (used as f64) <= (budget as f64) * COMPACT_THRESHOLD {
        return None;
    }

    Some(turns.len() - COMPACT_KEEP_TURNS)
}

/// Render the material handed to the model when compacting a conversation.
pub fn compaction_input(summary: Option<&str>, turns: &[(String, String)]) -> String {
    let mut text = String::new();
    if let Some(summary) = summary.filter(|s| !s.trim().is_empty()) {
        text.push_str("Summary of the conversation so far:\n");
        text.push_str(summary);
        text.push_str("\n\n");
    }
    text.push_str("Turns to fold in:\n\n");
    for (question, response) in turns {
        text.push_str("User: ");
        text.push_str(question);
        text.push_str("\nAssistant: ");
        text.push_str(response);
        text.push_str("\n\n");
    }
    text
}

/// Exponential backoff for retry attempt `attempt` (1-based).
fn backoff_ms(attempt: u32) -> u64 {
    200 * 2u64.pow(attempt - 1)
}

/// Return true when an HTTP status should be retried.
fn is_retryable_status(code: u16) -> bool {
    (500..=599).contains(&code)
}

/// Extract assistant text from OpenAI-compatible or legacy response bodies.
fn extract_response_text(body: &Value) -> Option<String> {
    body.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(str::to_string)
        .or_else(|| {
            body.get("data")
                .and_then(|d| d.get("text"))
                .and_then(|t| t.as_str())
                .map(str::to_string)
        })
}

/// Submit a chat completion request to an OpenAI-compatible API.
///
/// `summary` and `turns` carry the conversation context used by the interactive
/// pages; pass `None` and an empty slice for a single-turn question.
pub async fn submit(
    config: &Config,
    user_message: &str,
    summary: Option<&str>,
    turns: &[(String, String)],
) -> Result<String, ClaError> {
    let payload = build_payload(config, user_message, summary, turns);
    post_chat(config, &payload).await
}

/// Ask the model to fold `text` into a compact conversation summary.
pub async fn summarize(config: &Config, text: &str) -> Result<String, ClaError> {
    let payload = json!({
        "model": config.backend.model,
        "messages": [
            {"role": "system", "content": COMPACTION_INSTRUCTION},
            {"role": "user", "content": text}
        ],
        "max_tokens": COMPACTION_MAX_TOKENS,
        "temperature": config.backend.temperature
    });
    post_chat(config, &payload).await
}

/// POST `payload` to the chat completions endpoint, retrying server errors.
async fn post_chat(config: &Config, payload: &Value) -> Result<String, ClaError> {
    let client = client::create_client(config)?;
    let url = config.backend.chat_completions_url();
    let api_key = config.backend.effective_api_key();

    if api_key.is_empty() {
        return Err(ClaError::chat(
            "no API key configured — set `api_key` in config or `CL_API_KEY` env var",
        ));
    }

    let auth_header = format!("Bearer {}", api_key);
    let mut last_err: Option<String> = None;

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            warn!(
                "Retrying request (attempt {}/{}) after {}ms",
                attempt,
                MAX_RETRIES,
                backoff_ms(attempt)
            );
            tokio::time::sleep(std::time::Duration::from_millis(backoff_ms(attempt))).await;
        }

        debug!("POST {} (attempt {})", url, attempt + 1);

        let response = match client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", &auth_header)
            .json(payload)
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
            if let Some(text) = extract_response_text(&body) {
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
        if is_retryable_status(code) {
            last_err = Some(full_msg);
            continue;
        }

        return Err(ClaError::chat(full_msg));
    }

    Err(ClaError::chat(
        last_err.unwrap_or_else(|| "All retries exhausted".into()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn serve_one(status: u16, body: Value) -> std::net::SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let address = listener.local_addr().expect("local address");
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut request = [0u8; 4096];
            let _ = socket.read(&mut request).await;

            let body = serde_json::to_vec(&body).expect("json body");
            let response = format!(
                "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                status,
                body.len()
            );
            let mut bytes = response.into_bytes();
            bytes.extend_from_slice(&body);
            socket.write_all(&bytes).await.expect("write response");
        });
        address
    }

    fn config_for(address: std::net::SocketAddr) -> Config {
        let mut config = Config::default();
        config.backend.endpoint = format!("http://{}/v1", address);
        config.backend.api_key = "test-key".to_string();
        config
    }

    #[test]
    fn message_for_status_returns_known_messages() {
        assert_eq!(
            message_for_status(401),
            "Authentication failed: invalid API key"
        );
        assert_eq!(
            message_for_status(429),
            "Rate limited: too many requests, please try again later"
        );
        assert_eq!(message_for_status(418), "HTTP error 418");
    }

    #[test]
    fn payload_includes_backend_configuration() {
        let mut config = Config::default();
        config.backend.model = "test-model".to_string();
        config.backend.prompt = "test prompt".to_string();
        config.backend.language = "zh-CN".to_string();
        config.backend.max_tokens = 1234;
        config.backend.temperature = 0.25;

        let payload = build_payload(&config, "hello", None, &[]);

        assert_eq!(payload["model"], "test-model");
        assert_eq!(
            payload["messages"][0]["content"],
            "test prompt\n\nAlways reply in zh-CN."
        );
        assert_eq!(payload["messages"][1]["content"], "hello");
        assert_eq!(payload["max_tokens"], 1234);
        assert_eq!(payload["temperature"], 0.25);
    }

    #[test]
    fn estimate_tokens_counts_ascii_and_cjk() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1); // 4 ASCII chars ≈ 1 token
        assert_eq!(estimate_tokens("你好"), 2); // one per non-ASCII char
        assert_eq!(estimate_tokens("ab你好"), 2); // 2/4 + 2
    }

    #[test]
    fn payload_injects_summary_and_turns() {
        let mut config = Config::default();
        config.backend.prompt = "test prompt".to_string();

        let turns = vec![
            ("q1".to_string(), "r1".to_string()),
            ("q2".to_string(), "r2".to_string()),
        ];
        let payload = build_payload(&config, "q3", Some("earlier summary"), &turns);

        let system = payload["messages"][0]["content"].as_str().unwrap();
        assert_eq!(payload["messages"][0]["role"], "system");
        assert!(system.starts_with("test prompt"));
        assert!(system.contains("[Summary of earlier conversation]"));
        assert!(system.contains("earlier summary"));

        assert_eq!(payload["messages"][1]["role"], "user");
        assert_eq!(payload["messages"][1]["content"], "q1");
        assert_eq!(payload["messages"][2]["role"], "assistant");
        assert_eq!(payload["messages"][2]["content"], "r1");
        assert_eq!(payload["messages"][3]["content"], "q2");
        assert_eq!(payload["messages"][4]["content"], "r2");
        assert_eq!(payload["messages"][5]["role"], "user");
        assert_eq!(payload["messages"][5]["content"], "q3");
        assert_eq!(payload["messages"].as_array().unwrap().len(), 6);
    }

    #[test]
    fn payload_without_context_stays_single_turn() {
        let config = Config::default();
        let payload = build_payload(&config, "hello", None, &[]);
        assert_eq!(payload["messages"].as_array().unwrap().len(), 2);
        assert_eq!(payload["messages"][1]["content"], "hello");
    }

    #[test]
    fn compaction_point_triggers_only_when_over_budget() {
        let mut config = Config::default();
        config.backend.context_length = 10_000;
        config.backend.max_tokens = 1_000;

        // Few turns: never compact, even when one turn is huge.
        let few = vec![("x".repeat(50_000), "y".to_string())];
        assert_eq!(compaction_point(&config, "hi", None, &few), None);

        // Many small turns fit the budget.
        let small: Vec<(String, String)> = (0..10)
            .map(|_| ("short".to_string(), "answer".to_string()))
            .collect();
        assert_eq!(compaction_point(&config, "hi", None, &small), None);

        // Many large turns overflow: fold everything but the last 6.
        let large: Vec<(String, String)> = (0..10)
            .map(|_| ("x".repeat(2_000), "y".repeat(2_000)))
            .collect();
        assert_eq!(compaction_point(&config, "hi", None, &large), Some(4));
    }

    #[test]
    fn compaction_input_merges_summary_and_turns() {
        let turns = vec![("q1".to_string(), "r1".to_string())];

        let text = compaction_input(Some("old summary"), &turns);
        assert!(text.contains("Summary of the conversation so far:"));
        assert!(text.contains("old summary"));
        assert!(text.contains("User: q1"));
        assert!(text.contains("Assistant: r1"));

        let text = compaction_input(None, &turns);
        assert!(!text.contains("Summary of the conversation so far:"));
    }

    #[test]
    fn retry_helpers_use_exponential_backoff_and_server_errors() {
        assert_eq!(backoff_ms(1), 200);
        assert_eq!(backoff_ms(2), 400);
        assert_eq!(backoff_ms(3), 800);
        assert!(is_retryable_status(500));
        assert!(is_retryable_status(503));
        assert!(!is_retryable_status(400));
    }

    #[test]
    fn extract_response_text_supports_openai_and_legacy_formats() {
        let openai = json!({"choices": [{"message": {"content": "answer"}}]});
        assert_eq!(extract_response_text(&openai), Some("answer".to_string()));

        let legacy = json!({"data": {"text": "legacy answer"}});
        assert_eq!(
            extract_response_text(&legacy),
            Some("legacy answer".to_string())
        );

        assert_eq!(extract_response_text(&json!({"unexpected": true})), None);
    }

    #[tokio::test]
    async fn submit_parses_successful_openai_response() {
        let address = serve_one(
            200,
            json!({"choices": [{"message": {"content": "answer"}}]}),
        )
        .await;
        let config = config_for(address);

        let response = submit(&config, "hello", None, &[]).await.expect("submit");
        assert_eq!(response, "answer");
    }

    #[tokio::test]
    async fn submit_does_not_retry_client_errors() {
        let address = serve_one(401, json!({"error": {"message": "bad key"}})).await;
        let config = config_for(address);

        let error = submit(&config, "hello", None, &[])
            .await
            .expect_err("submit");
        assert!(error.to_string().contains("Authentication failed"));
    }

    #[tokio::test]
    async fn submit_retries_server_error_then_succeeds() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let address = listener.local_addr().expect("local address");
        tokio::spawn(async move {
            for (index, body) in [
                json!({"error": {"message": "temporary"}}),
                json!({"choices": [{"message": {"content": "recovered"}}]}),
            ]
            .into_iter()
            .enumerate()
            {
                let (mut socket, _) = listener.accept().await.expect("accept");
                let mut request = [0u8; 4096];
                let _ = socket.read(&mut request).await;

                let status = if index == 0 { 500 } else { 200 };
                let body = serde_json::to_vec(&body).expect("json body");
                let response = format!(
                    "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    status,
                    body.len()
                );
                let mut bytes = response.into_bytes();
                bytes.extend_from_slice(&body);
                socket.write_all(&bytes).await.expect("write response");
            }
        });
        let config = config_for(address);

        let response = submit(&config, "hello", None, &[]).await.expect("submit");
        assert_eq!(response, "recovered");
    }
}
