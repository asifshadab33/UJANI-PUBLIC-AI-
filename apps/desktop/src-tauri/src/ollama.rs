//! Strictly local Ollama adapter. It contacts only the loopback address and never a cloud endpoint.
use serde::{Deserialize, Serialize};
use std::time::Duration;

const BASE_URL: &str = "http://127.0.0.1:11434";
const MAX_PROMPT_BYTES: usize = 32 * 1024;
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_ATTEMPTS: u8 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
}
#[derive(Deserialize)]
struct ModelList { models: Vec<OllamaModel> }
#[derive(Serialize)]
struct GenerateRequest<'a> { model: &'a str, prompt: &'a str, stream: bool }
#[derive(Deserialize)]
struct GenerateResponse { response: String }

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(120)).build()
        .map_err(|e| format!("Could not initialise local Ollama client: {e}"))
}
fn retryable_status(status: reqwest::StatusCode) -> bool {\n    status.is_server_error()\n}\n\nfn valid_model(model: &str) -> bool {
    !model.is_empty() && model.len() <= 128 &&
        model.chars().all(|c| c.is_ascii_alphanumeric() || "._:-/".contains(c))
}
async fn parse_limited<T: for<'de> Deserialize<'de>>(mut response: reqwest::Response) -> Result<T, String> {
    let status = response.status();
    if !status.is_success() { return Err(format!("Local Ollama returned HTTP {status}.")); }
    if response.content_length().is_some_and(|n| n > MAX_RESPONSE_BYTES as u64) {
        return Err("Local Ollama response exceeds the 2 MiB limit.".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| format!("Could not read local Ollama response: {e}"))? {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err("Local Ollama response exceeds the 2 MiB limit.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("Could not parse local Ollama response: {e}"))
}

pub async fn is_available() -> bool {
    match client() {
        Ok(c) => c.get(format!("{BASE_URL}/api/tags")).send().await
            .map(|r| r.status().is_success()).unwrap_or(false),
        Err(_) => false,
    }
}
pub async fn list_models() -> Result<Vec<OllamaModel>, String> {
    let response = client()?.get(format!("{BASE_URL}/api/tags")).send().await
        .map_err(|e| format!("Cannot connect to Ollama at 127.0.0.1:11434: {e}"))?;
    let list: ModelList = parse_limited(response).await?;
    Ok(list.models)
}

/// Retry only transient connection/time-out and server failures. Validation errors,
/// oversized responses, malformed JSON and other client errors are never retried.
pub async fn generate(model: &str, prompt: &str) -> Result<String, String> {
    if !valid_model(model) { return Err("Invalid local model identifier.".into()); }
    if prompt.trim().is_empty() { return Err("Prompt cannot be empty.".into()); }
    if prompt.len() > MAX_PROMPT_BYTES { return Err("Prompt exceeds the 32 KiB local limit.".into()); }

    let c = client()?;
    let request = GenerateRequest { model, prompt, stream: false };
    let mut last_error = String::new();

    for attempt in 0..MAX_ATTEMPTS {
        let response = c.post(format!("{BASE_URL}/api/generate")).json(&request).send().await;
        match response {
            Ok(response) if retryable_status(response.status()) => {
                last_error = format!("Local Ollama returned HTTP {}.", response.status());
            }
            Ok(response) => {
                let result: GenerateResponse = parse_limited(response).await?;
                if result.response.trim().is_empty() {
                    return Err("Local Ollama returned an empty response.".into());
                }
                return Ok(result.response);
            }
            Err(error) if error.is_timeout() || error.is_connect() => {
                last_error = format!("Local Ollama request failed: {error}");
            }
            Err(error) => return Err(format!("Local Ollama request failed: {error}")),
        }

        if attempt + 1 < MAX_ATTEMPTS {
            let delay_ms = 250_u64 * (1_u64 << attempt);
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    Err(format!("Local Ollama request failed after {MAX_ATTEMPTS} attempts: {last_error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_bounded_local_model_identifiers() {
        assert!(valid_model("llama3.2"));
        assert!(valid_model("org/model:tag"));
        assert!(!valid_model(""));
        assert!(!valid_model("../model"));
        assert!(!valid_model("model name"));
        assert!(!valid_model(&"m".repeat(129)));\n    }\n\n    #[test]\n    fn retries_server_errors_but_not_client_errors() {\n        assert!(retryable_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR));\n        assert!(retryable_status(reqwest::StatusCode::SERVICE_UNAVAILABLE));\n        assert!(!retryable_status(reqwest::StatusCode::BAD_REQUEST));\n        assert!(!retryable_status(reqwest::StatusCode::UNAUTHORIZED));
    }
}
