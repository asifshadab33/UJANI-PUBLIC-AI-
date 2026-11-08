//! Strictly local Ollama adapter. It contacts only the loopback address and never a cloud endpoint.
use serde::{Deserialize, Serialize};
use std::{sync::OnceLock, time::Duration};

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

const NUM_CTX: u32 = 8192;
#[derive(Serialize)] struct ChatMsg<'a> { role: &'a str, content: &'a str }
#[derive(Serialize)] struct ChatOptions { num_ctx: u32, temperature: f32 }
#[derive(Serialize)] struct ChatBody<'a> { model: &'a str, messages: Vec<ChatMsg<'a>>, stream: bool, options: ChatOptions, keep_alive: &'a str }
#[derive(Deserialize)] struct ChatReply { message: ChatReplyMsg }
#[derive(Deserialize)] struct ChatReplyMsg { content: String }
pub struct Turn<'a> { pub role: &'a str, pub content: &'a str }

fn build_client(total_timeout: Option<Duration>) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy();
    if let Some(timeout) = total_timeout {
        builder = builder.timeout(timeout);
    }
    builder.build().map_err(|e| format!("Could not initialise local Ollama client: {e}"))
}

fn client() -> Result<&'static reqwest::Client, String> {
    static CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();
    CLIENT.get_or_init(|| build_client(Some(Duration::from_secs(180))))
        .as_ref().map_err(Clone::clone)
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

pub fn valid_pull_model(model: &str) -> bool {
    if !valid_model(model) { return false; }
    let segments: Vec<&str> = model.split('/').collect();
    segments.len() <= 2 && !(segments.len() == 2 && (segments[0].contains('.') || segments[0].contains(':')))
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
    let response = client()?
        .get(format!("{BASE_URL}/api/tags"))
        .timeout(Duration::from_secs(6))
        .send()
        .await
        .map_err(|e| format!("Cannot connect to Ollama at 127.0.0.1:11434 within 6 seconds: {e}"))?;
    let list: ModelList = parse_limited(response).await?;
    Ok(list.models)
}

fn build_chat_body<'a>(
    model: &'a str,
    system: Option<&'a str>,
    turns: &[Turn<'a>],
    stream: bool,
) -> Result<ChatBody<'a>, String> {
    if !valid_model(model) {
        return Err("Invalid local model identifier.".into());
    }
    let total = turns
        .iter()
        .try_fold(system.map_or(0usize, str::len), |n, turn| n.checked_add(turn.content.len()))
        .ok_or_else(|| "Prompt size overflow.".to_string())?;
    if turns.is_empty() || total == 0 {
        return Err("Prompt cannot be empty.".into());
    }
    if total > MAX_PROMPT_BYTES {
        return Err("Prompt exceeds the 32 KiB local limit.".into());
    }
    let mut messages = Vec::with_capacity(turns.len() + usize::from(system.is_some()));
    if let Some(content) = system {
        messages.push(ChatMsg { role: "system", content });
    }
    for turn in turns {
        if turn.role != "user" && turn.role != "assistant" {
            return Err("Invalid chat role.".into());
        }
        messages.push(ChatMsg { role: turn.role, content: turn.content });
    }
    Ok(ChatBody {
        model,
        messages,
        stream,
        options: ChatOptions { num_ctx: NUM_CTX, temperature: 0.3 },
        keep_alive: "5m",
    })
}

#[derive(Deserialize)]
struct ChatStreamEvent {
    #[serde(default)] message: Option<ChatReplyMsg>,
    #[serde(default)] done: bool,
    #[serde(default)] error: Option<String>,
}

fn parse_chat_stream_line(line: &[u8]) -> Result<(String, bool), String> {
    let event: ChatStreamEvent = serde_json::from_slice(line)
        .map_err(|_| "Local Ollama returned an invalid streaming chat event.".to_string())?;
    if let Some(error) = event.error {
        return Err(format!("Local Ollama streaming chat failed: {error}"));
    }
    Ok((event.message.map(|message| message.content).unwrap_or_default(), event.done))
}

pub async fn chat(
    model: &str,
    system: Option<&str>,
    turns: &[Turn<'_>],
) -> Result<String, String> {
    let body = build_chat_body(model, system, turns, false)?;
    let c = client()?;
    let mut last_error = String::new();
    for attempt in 0..MAX_ATTEMPTS {
        match c.post(format!("{BASE_URL}/api/chat")).json(&body).send().await {
            Ok(response) if retryable_status(response.status()) => {
                last_error = format!("Local Ollama returned HTTP {}.", response.status());
            }
            Ok(response) => {
                let parsed: ChatReply = parse_limited(response).await?;
                if parsed.message.content.trim().is_empty() {
                    return Err("Local Ollama returned an empty response.".into());
                }
                return Ok(parsed.message.content);
            }
            Err(error) if error.is_connect() => {
                last_error = format!("Local Ollama request failed: {error}");
            }
            Err(error) => return Err(format!("Local Ollama request failed: {error}")),
        }
        if attempt + 1 < MAX_ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(250 * (1u64 << attempt))).await;
        }
    }
    Err(format!("Local Ollama request failed after {MAX_ATTEMPTS} attempts: {last_error}"))
}

/// Stream local chat deltas from Ollama's NDJSON response. The callback receives
/// plain text only; output and per-event sizes are bounded before forwarding.
pub async fn chat_stream<F>(
    model: &str,
    system: Option<&str>,
    turns: &[Turn<'_>],
    mut on_chunk: F,
) -> Result<String, String>
where
    F: FnMut(&str),
{
    const MAX_STREAM_EVENT_BYTES: usize = 64 * 1024;
    let body = build_chat_body(model, system, turns, true)?;
    let response = client()?.post(format!("{BASE_URL}/api/chat")).json(&body).send().await
        .map_err(|error| format!("Local Ollama streaming request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Local Ollama returned HTTP {} while starting chat.", response.status()));
    }
    let mut response = response;
    let mut pending = Vec::<u8>::new();
    let mut output = String::new();
    let mut received_bytes = 0usize;
    let mut done = false;
    while let Some(chunk) = response.chunk().await
        .map_err(|error| format!("Local Ollama chat stream failed: {error}"))?
    {
        received_bytes = received_bytes.checked_add(chunk.len())
            .ok_or_else(|| "Local Ollama stream size overflow.".to_string())?;
        if received_bytes > MAX_RESPONSE_BYTES {
            return Err("Local Ollama streaming response exceeded the 2 MiB output limit.".into());
        }
        for byte in chunk {
            if byte == b'\n' {
                if pending.is_empty() { continue; }
                let (delta, finished) = parse_chat_stream_line(&pending)?;
                pending.clear();
                if !delta.is_empty() {
                    if output.len().saturating_add(delta.len()) > MAX_RESPONSE_BYTES {
                        return Err("Local Ollama streaming output exceeded the 2 MiB limit.".into());
                    }
                    output.push_str(&delta);
                    on_chunk(&delta);
                }
                if finished { done = true; break; }
            } else {
                pending.push(byte);
                if pending.len() > MAX_STREAM_EVENT_BYTES {
                    return Err("Local Ollama streaming event exceeded the 64 KiB safety limit.".into());
                }
            }
        }
        if done { break; }
    }
    if !done && !pending.is_empty() {
        let (delta, finished) = parse_chat_stream_line(&pending)?;
        if !delta.is_empty() {
            if output.len().saturating_add(delta.len()) > MAX_RESPONSE_BYTES {
                return Err("Local Ollama streaming output exceeded the 2 MiB limit.".into());
            }
            output.push_str(&delta);
            on_chunk(&delta);
        }
        done = finished;
    }
    if !done {
        return Err("Local Ollama chat stream ended before completion.".into());
    }
    if output.trim().is_empty() {
        return Err("Local Ollama returned an empty response.".into());
    }
    Ok(output)
}

pub async fn generate(model: &str, prompt: &str) -> Result<String, String> {
    chat(model, None, &[Turn { role: "user", content: prompt }]).await
}

#[derive(Debug, PartialEq, Eq)]
struct PullProgress {
    status: String,
    percent: u8,
    success: bool,
}

fn parse_pull_progress_line(line: &[u8]) -> Result<PullProgress, String> {
    let event: serde_json::Value = serde_json::from_slice(line)
        .map_err(|_| "Ollama returned an invalid model-download progress event.".to_string())?;
    if let Some(error) = event.get("error").and_then(|value| value.as_str()) {
        return Err(format!("Ollama could not download this model: {error}"));
    }
    let status = event.get("status").and_then(|value| value.as_str())
        .unwrap_or("Downloading model").to_string();
    let total = event.get("total").and_then(|value| value.as_u64()).unwrap_or(0);
    let completed = event.get("completed").and_then(|value| value.as_u64()).unwrap_or(0);
    let percent = if total > 0 {
        ((completed as u128 * 100 / total as u128).min(99)) as u8
    } else { 0 };
    let success = status.eq_ignore_ascii_case("success");
    Ok(PullProgress { status, percent, success })
}

#[derive(Serialize)]
struct PullModelRequest<'a> { name: &'a str, stream: bool }

/// Pulls one model from the local Ollama service. Progress is parsed from its NDJSON stream.
pub async fn pull_model<F>(model: &str, mut on_progress: F) -> Result<(), String>
where F: FnMut(String, u8) {
    if !valid_pull_model(model) {
        return Err("Invalid Ollama model name. Use an official library identifier such as llama3.2 or qwen2.5:3b.".into());
    }
    let client = build_client(None)?;
    let response = client.post(format!("{BASE_URL}/api/pull"))
        .json(&PullModelRequest { name: model, stream: true })
        .send().await
        .map_err(|e| format!("Could not start local model download: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Local Ollama returned HTTP {} while starting model download.", response.status()));
    }
    let mut response = response;
    let mut pending = Vec::<u8>::new();
    let mut last_percent: i32 = -1;
    let mut last_status = String::new();
    let mut completed = false;
    loop {
        let chunk = match tokio::time::timeout(Duration::from_secs(120), response.chunk()).await {
            Err(_) => return Err("Model download stalled for 2 minutes. Check your connection and retry.".into()),
            Ok(Err(e)) => return Err(format!("Model download stream failed: {e}")),
            Ok(Ok(None)) => break,
            Ok(Ok(Some(chunk))) => chunk,
        };
        for byte in chunk {
            if byte == b'\n' {
                if pending.is_empty() { continue; }
                let progress = parse_pull_progress_line(&pending)?;
                pending.clear();
                if progress.percent as i32 != last_percent || progress.status != last_status {
                    last_percent = progress.percent as i32;
                    last_status = progress.status.clone();
                    on_progress(progress.status.clone(), progress.percent);
                }
                if progress.success { completed = true; }
            } else {
                pending.push(byte);
                if pending.len() > 16 * 1024 {
                    return Err("Ollama progress event exceeded the 16 KiB safety limit.".into());
                }
            }
        }
    }
    if !pending.is_empty() {
        let progress = parse_pull_progress_line(&pending)
            .map_err(|_| "Ollama returned an incomplete or invalid model-download progress event.".to_string())?;
        on_progress(progress.status.clone(), progress.percent);
        if progress.success { completed = true; }
    }
    if !completed { return Err("Ollama download ended before reporting success. Check Ollama and retry.".into()); }
    on_progress("Model installed and ready".into(), 100);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_body_shares_validation_and_can_enable_streaming() {
        let turn = Turn { role: "user", content: "hello" };
        let body = build_chat_body("llama3.2", None, &[turn], true).unwrap();
        assert!(body.stream);
        assert_eq!(body.messages.len(), 1);
        assert_eq!(body.messages[0].role, "user");
        let invalid = Turn { role: "system", content: "not permitted as a turn" };
        assert!(build_chat_body("llama3.2", None, &[invalid], true).is_err());
    }

    #[test]
    fn parses_streaming_chat_deltas_and_completion() {
        let (delta, done) = parse_chat_stream_line(br#"{"message":{"role":"assistant","content":"hello"},"done":false}"#).unwrap();
        assert_eq!(delta, "hello");
        assert!(!done);
        let (delta, done) = parse_chat_stream_line(br#"{"message":{"role":"assistant","content":""},"done":true}"#).unwrap();
        assert!(delta.is_empty());
        assert!(done);
    }

    #[test]
    fn rejects_stream_errors_and_malformed_events() {
        assert!(parse_chat_stream_line(br#"{"error":"model unavailable"}"#).unwrap_err().contains("model unavailable"));
        assert!(parse_chat_stream_line(b"not-json").is_err());
    }

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
    fn pull_rejects_custom_registry_hosts() {
        assert!(valid_pull_model("llama3.2"));
        assert!(valid_pull_model("qwen2.5:3b"));
        assert!(valid_pull_model("library/llama3.2"));
        assert!(!valid_pull_model("evil.example.com/ns/model"));
        assert!(!valid_pull_model("localhost:5000/m"));
        assert!(!valid_pull_model("hf.co/org/repo"));
        assert!(!valid_pull_model("a/b/c"));
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
    fn parses_download_progress_and_never_reports_complete_before_success() {
        let progress = parse_pull_progress_line(br#"{"status":"downloading","total":1000,"completed":425}"#).unwrap();
        assert_eq!(progress.status, "downloading");
        assert_eq!(progress.percent, 42);
        assert!(!progress.success);
        let done = parse_pull_progress_line(br#"{"status":"success"}"#).unwrap();
        assert_eq!(done.percent, 0);
        assert!(done.success);
    }

    #[test]
    fn caps_download_progress_and_rejects_provider_errors_or_bad_json() {
        let progress = parse_pull_progress_line(br#"{"status":"downloading","total":10,"completed":100}"#).unwrap();
        assert_eq!(progress.percent, 99);
        assert!(parse_pull_progress_line(br#"{"error":"model not found"}"#).unwrap_err().contains("model not found"));
        assert!(parse_pull_progress_line(b"not-json").is_err());
    }

    #[test]
    fn accelerator_observation_requires_positive_runtime_vram_evidence() {
        let gpu = RunningModels { models: vec![RunningModel { size_vram: 1024 }] };
        assert_eq!(gpu.models.iter().fold(0_u64, |total, m| total.saturating_add(m.size_vram)), 1024);
        let cpu = RunningModels { models: vec![RunningModel { size_vram: 0 }] };
        assert_eq!(cpu.models.iter().fold(0_u64, |total, m| total.saturating_add(m.size_vram)), 0);
    }
}
