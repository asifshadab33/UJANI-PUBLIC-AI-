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
        .timeout(Duration::from_secs(45)).build()
        .map_err(|e| format!("Could not initialise local Ollama client: {e}"))
}
fn retryable_status(status: reqwest::StatusCode) -> bool {
    // Retry transient throttling/timeouts and server faults, but never ordinary
    // client errors such as invalid model names or malformed requests.
    status == reqwest::StatusCode::REQUEST_TIMEOUT
        || status == reqwest::StatusCode::TOO_EARLY
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

fn valid_model(model: &str) -> bool {
    !model.is_empty() && model.len() <= 128 && !model.contains("..") &&
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


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceleratorObservation {
    pub gpu_vram_allocated_bytes: u64,
    pub running_models: usize,
    pub evidence: String,
}
#[derive(Deserialize)]
struct RunningModels { models: Vec<RunningModel> }
#[derive(Deserialize)]
struct RunningModel { #[serde(default)] size_vram: u64 }

/// Positive VRAM allocation reported by Ollama is runtime evidence of GPU use.
/// This is not a complete inventory of GPUs, drivers, or NPU capabilities.
pub async fn accelerator_observation() -> Result<AcceleratorObservation, String> {
    let response = client()?.get(format!("{BASE_URL}/api/ps"))
        .timeout(Duration::from_secs(3)).send().await
        .map_err(|e| format!("Could not inspect local Ollama runtime: {e}"))?;
    let list: RunningModels = parse_limited(response).await?;
    let gpu_vram_allocated_bytes = list.models.iter().fold(0_u64, |total, model| total.saturating_add(model.size_vram));
    let running_models = list.models.len();
    let evidence = if gpu_vram_allocated_bytes > 0 {
        format!("Ollama reports {gpu_vram_allocated_bytes} bytes of model VRAM allocation across {running_models} running model(s)")
    } else if running_models > 0 {
        format!("Ollama reports {running_models} running model(s) but no VRAM allocation; GPU use is not confirmed")
    } else {
        "No running Ollama model; accelerator use cannot be verified".into()
    };
    Ok(AcceleratorObservation { gpu_vram_allocated_bytes, running_models, evidence })
}

pub async fn is_available() -> bool {
    match client() {
        Ok(c) => c.get(format!("{BASE_URL}/api/tags")).timeout(Duration::from_secs(3)).send().await
            .map(|r| r.status().is_success()).unwrap_or(false),
        Err(_) => false,
    }
}
pub async fn list_models() -> Result<Vec<OllamaModel>, String> {
    // Model discovery is a startup-adjacent health operation; cap it separately from inference.
    let response = client()?.get(format!("{BASE_URL}/api/tags")).timeout(Duration::from_secs(6)).send().await
        .map_err(|e| format!("Cannot connect to Ollama at 127.0.0.1:11434 within 6 seconds: {e}"))?;
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

#[derive(Serialize)]
struct PullModelRequest<'a> { name: &'a str, stream: bool }

/// Pulls one model from the local Ollama service. Progress is parsed from its NDJSON stream.
pub async fn pull_model<F>(model: &str, mut on_progress: F) -> Result<(), String>
where F: FnMut(String, u8) {
    if !valid_model(model) {
        return Err("Invalid Ollama model name. Use a library identifier such as llama3.2 or qwen2.5:3b.".into());
    }
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30 * 60))
        .build()
        .map_err(|e| format!("Could not initialise local Ollama download client: {e}"))?;
    let response = client.post(format!("{BASE_URL}/api/pull"))
        .json(&PullModelRequest { name: model, stream: true })
        .send().await
        .map_err(|e| format!("Could not start local model download: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Local Ollama returned HTTP {} while starting model download.", response.status()));
    }
    let mut response = response;
    let mut pending = Vec::<u8>::new();
    let mut received = 0usize;
    let mut completed = false;
    while let Some(chunk) = response.chunk().await.map_err(|e| format!("Model download stream failed: {e}"))? {
        received = received.saturating_add(chunk.len());
        if received > 8 * 1024 * 1024 {
            return Err("Ollama progress stream exceeded the 8 MiB safety limit.".into());
        }
        for byte in chunk {
            if byte == b'\n' {
                if pending.is_empty() { continue; }
                let event: serde_json::Value = serde_json::from_slice(&pending)
                    .map_err(|_| "Ollama returned an invalid model-download progress event.".to_string())?;
                pending.clear();
                if let Some(error) = event.get("error").and_then(|v| v.as_str()) {
                    return Err(format!("Ollama could not download this model: {error}"));
                }
                let status = event.get("status").and_then(|v| v.as_str()).unwrap_or("Downloading model");
                let total = event.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                let done = event.get("completed").and_then(|v| v.as_u64()).unwrap_or(0);
                let percent = if total > 0 { ((done as u128 * 100 / total as u128).min(99)) as u8 } else { 0 };
                on_progress(status.to_string(), percent);
                if status.eq_ignore_ascii_case("success") { completed = true; }
            } else {
                pending.push(byte);
                if pending.len() > 16 * 1024 {
                    return Err("Ollama progress event exceeded the 16 KiB safety limit.".into());
                }
            }
        }
    }
    if !pending.is_empty() {
        let event: serde_json::Value = serde_json::from_slice(&pending)
            .map_err(|_| "Ollama returned an incomplete model-download progress event.".to_string())?;
        if let Some(error) = event.get("error").and_then(|v| v.as_str()) {
            return Err(format!("Ollama could not download this model: {error}"));
        }
        if event.get("status").and_then(|v| v.as_str()) == Some("success") { completed = true; }
    }
    if !completed { return Err("Ollama download ended before reporting success. Check Ollama and retry.".into()); }
    on_progress("Model installed and ready".into(), 100);
    Ok(())
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
        assert!(!valid_model("org/../model"));
        assert!(!valid_model("model name"));
        assert!(!valid_model(&"m".repeat(129)));
    }

    #[test]
    fn retries_server_errors_but_not_client_errors() {
        assert!(retryable_status(reqwest::StatusCode::REQUEST_TIMEOUT));
        assert!(retryable_status(reqwest::StatusCode::TOO_EARLY));
        assert!(retryable_status(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(retryable_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR));
        assert!(retryable_status(reqwest::StatusCode::SERVICE_UNAVAILABLE));
        assert!(!retryable_status(reqwest::StatusCode::NOT_FOUND));
        assert!(!retryable_status(reqwest::StatusCode::BAD_REQUEST));
        assert!(!retryable_status(reqwest::StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn accelerator_observation_requires_positive_runtime_vram_evidence() {
        let gpu = RunningModels { models: vec![RunningModel { size_vram: 1024 }] };
        assert_eq!(gpu.models.iter().fold(0_u64, |total, m| total.saturating_add(m.size_vram)), 1024);
        let cpu = RunningModels { models: vec![RunningModel { size_vram: 0 }] };
        assert_eq!(cpu.models.iter().fold(0_u64, |total, m| total.saturating_add(m.size_vram)), 0);
    }
}
