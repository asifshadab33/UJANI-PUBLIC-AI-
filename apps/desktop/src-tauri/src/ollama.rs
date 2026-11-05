//! Strictly local Ollama adapter. It contacts only the loopback address and never a cloud endpoint.
use serde::{Deserialize, Serialize};
use std::time::Duration;

const BASE_URL: &str = "http://127.0.0.1:11434";
const MAX_PROMPT_BYTES: usize = 32 * 1024;
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

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
fn valid_model(model: &str) -> bool {
    !model.is_empty() && model.len() <= 128 &&
        model.chars().all(|c| c.is_ascii_alphanumeric() || "._:-/".contains(c))
}
async fn parse_limited<T: for<'de> Deserialize<'de>>(response: reqwest::Response) -> Result<T, String> {
    let status = response.status();
    if !status.is_success() { return Err(format!("Local Ollama returned HTTP {status}.")); }
    if response.content_length().is_some_and(|n| n > MAX_RESPONSE_BYTES as u64) {
        return Err("Local Ollama response exceeds the 2 MiB limit.".into());
    }
    let bytes = response.bytes().await.map_err(|e| format!("Could not read local Ollama response: {e}"))?;
    if bytes.len() > MAX_RESPONSE_BYTES { return Err("Local Ollama response exceeds the 2 MiB limit.".into()); }
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
pub async fn generate(model: &str, prompt: &str) -> Result<String, String> {
    if !valid_model(model) { return Err("Invalid local model identifier.".into()); }
    if prompt.trim().is_empty() { return Err("Prompt cannot be empty.".into()); }
    if prompt.len() > MAX_PROMPT_BYTES { return Err("Prompt exceeds the 32 KiB local limit.".into()); }
    let c = client()?;
    let response = c.post(format!("{BASE_URL}/api/generate"))
        .json(&GenerateRequest { model, prompt, stream: false })
        .send().await.map_err(|e| format!("Local Ollama request failed: {e}"))?;
    let result: GenerateResponse = parse_limited(response).await?;
    if result.response.trim().is_empty() { return Err("Local Ollama returned an empty response.".into()); }
    Ok(result.response)
}
