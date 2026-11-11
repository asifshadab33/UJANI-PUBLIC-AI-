//! Strictly local Ollama adapter. It contacts only the loopback address and never a cloud endpoint.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::{HashMap, HashSet}, sync::OnceLock, time::Duration};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

const BASE_URL: &str = "http://127.0.0.1:11434";
const MAX_PROMPT_BYTES: usize = 32 * 1024;
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_ATTEMPTS: u8 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<OllamaModelDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OllamaModelDetails {
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub families: Option<Vec<String>>,
    #[serde(default)]
    pub parameter_size: Option<String>,
    #[serde(default)]
    pub quantization_level: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OllamaModelMetadata {
    pub context_length_tokens: Option<u64>,
    pub capabilities: Vec<String>,
}

#[derive(Deserialize)]
struct ModelList {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct VersionResponse {
    version: String,
}

#[derive(Serialize)]
struct ShowModelRequest<'a> {
    model: &'a str,
    verbose: bool,
}

#[derive(Deserialize, Default)]
struct ShowModelResponse {
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    model_info: HashMap<String, Value>,
}

const MAX_INSTALLED_MODELS: usize = 512;
const MAX_MODEL_METADATA_INSPECTIONS: usize = 32;
const MAX_MODEL_METADATA_CONCURRENCY: usize = 4;

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
fn validate_model_list(models: &[OllamaModel]) -> Result<(), String> {
    if models.len() > MAX_INSTALLED_MODELS {
        return Err(format!(
            "Local Ollama reports more than the {MAX_INSTALLED_MODELS}-model inventory limit."
        ));
    }
    let mut seen = HashSet::with_capacity(models.len());
    for model in models {
        if !valid_model(&model.name) || !seen.insert(model.name.as_str()) {
            return Err(
                "Local Ollama returned an invalid or duplicate model identifier.".into(),
            );
        }
    }
    Ok(())
}

pub async fn list_models() -> Result<Vec<OllamaModel>, String> {
    let response = client()?
        .get(format!("{BASE_URL}/api/tags"))
        .timeout(Duration::from_secs(6))
        .send()
        .await
        .map_err(|e| format!("Cannot connect to Ollama at 127.0.0.1:11434 within 6 seconds: {e}"))?;
    let list: ModelList = parse_limited(response).await?;
    validate_model_list(&list.models)?;
    Ok(list.models)
}

pub async fn runtime_version() -> Result<String, String> {
    let response = client()?
        .get(format!("{BASE_URL}/api/version"))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .map_err(|error| format!("Could not read local Ollama version: {error}"))?;
    let version: VersionResponse = parse_limited(response).await?;
    let version = version.version.trim();
    if version.is_empty()
        || version.len() > 64
        || !version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-_+".contains(c))
    {
        return Err("Local Ollama returned an invalid version identifier.".into());
    }
    Ok(version.to_string())
}

fn context_length_tokens(model_info: &HashMap<String, Value>) -> Option<u64> {
    model_info
        .iter()
        .filter(|(key, _)| key.ends_with(".context_length"))
        .filter_map(|(_, value)| value.as_u64())
        .find(|value| *value > 0 && *value <= 1_000_000)
}

fn canonical_model_capabilities(reported: &[String]) -> Vec<String> {
    let mut capabilities = Vec::new();
    if reported.iter().any(|value| value == "completion") {
        capabilities.push("chat".to_string());
    }
    if reported.iter().any(|value| value == "vision") {
        capabilities.push("vision".to_string());
    }
    if reported.iter().any(|value| value == "tools") {
        capabilities.push("tool_use".to_string());
    }
    if reported.iter().any(|value| value == "embedding" || value == "embeddings") {
        capabilities.push("embeddings".to_string());
    }
    if capabilities.is_empty() {
        capabilities.push("unknown".to_string());
    }
    capabilities
}

/// Read model capabilities and context only from Ollama's bounded local metadata API.
/// Unsupported/older runtime responses are treated as unknown, never guessed from names.
pub async fn inspect_model_metadata(model: &str) -> Result<OllamaModelMetadata, String> {
    if !valid_model(model) {
        return Err("Invalid local model identifier.".into());
    }
    let response = client()?
        .post(format!("{BASE_URL}/api/show"))
        .json(&ShowModelRequest { model, verbose: false })
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map_err(|error| format!("Could not inspect local Ollama model metadata: {error}"))?;
    let details: ShowModelResponse = parse_limited(response).await?;
    Ok(OllamaModelMetadata {
        context_length_tokens: context_length_tokens(&details.model_info),
        capabilities: canonical_model_capabilities(&details.capabilities),
    })
}

/// Inspect at most 32 installed models, with four concurrent loopback requests and
/// an eight-second aggregate deadline. Failed, unsupported or uninspected models
/// are intentionally absent from this map so callers can report unknown metadata.
pub async fn inspect_models(models: &[OllamaModel]) -> HashMap<String, OllamaModelMetadata> {
    let mut pending = models
        .iter()
        .take(MAX_MODEL_METADATA_INSPECTIONS)
        .filter(|model| valid_model(&model.name))
        .map(|model| model.name.clone());
    let mut tasks = JoinSet::new();
    let mut inspected = HashMap::new();

    for _ in 0..MAX_MODEL_METADATA_CONCURRENCY {
        if let Some(name) = pending.next() {
            tasks.spawn(async move {
                let result = inspect_model_metadata(&name).await;
                (name, result)
            });
        }
    }

    let timed_out = tokio::time::timeout(Duration::from_secs(8), async {
        while let Some(joined) = tasks.join_next().await {
            if let Ok((name, Ok(metadata))) = joined {
                inspected.insert(name, metadata);
            }
            if let Some(name) = pending.next() {
                tasks.spawn(async move {
                    let result = inspect_model_metadata(&name).await;
                    (name, result)
                });
            }
        }
    })
    .await
    .is_err();

    if timed_out {
        tasks.abort_all();
    }
    inspected
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
/// Streams bounded local Ollama chat deltas with per-event, total-output, and idle-time limits.
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
    loop {
        let chunk = match tokio::time::timeout(Duration::from_secs(35), response.chunk()).await {
            Err(_) => return Err("Local Ollama chat stream stalled for 35 seconds.".into()),
            Ok(Err(error)) => return Err(format!("Local Ollama chat stream failed: {error}")),
            Ok(Ok(None)) => break,
            Ok(Ok(Some(chunk))) => chunk,
        };
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

const MAX_PULL_EVENT_BYTES: usize = 16 * 1024;
const MAX_PULL_STREAM_BYTES: usize = 8 * 1024 * 1024;
const MAX_PULL_STATUS_BYTES: usize = 256;
const MAX_PULL_ERROR_BYTES: usize = 512;

/// Bound and neutralise untrusted status text received from the local Ollama HTTP service.
fn bounded_progress_text(value: &str, max_bytes: usize) -> String {
    let mut output = String::new();
    for character in value.chars().filter(|character| !character.is_control()) {
        if output.len().saturating_add(character.len_utf8()) > max_bytes {
            break;
        }
        output.push(character);
    }
    output
}

#[derive(Debug, PartialEq, Eq)]
struct PullProgress {
    status: String,
    percent: u8,
    success: bool,
}

/// Parse one bounded Ollama NDJSON progress event into a safe progress update.
fn parse_pull_progress_line(line: &[u8]) -> Result<PullProgress, String> {
    let event: serde_json::Value = serde_json::from_slice(line)
        .map_err(|_| "Ollama returned an invalid model-download progress event.".to_string())?;
    if let Some(error) = event.get("error").and_then(|value| value.as_str()) {
        let error = bounded_progress_text(error, MAX_PULL_ERROR_BYTES);
        return Err(format!("Ollama could not download this model: {error}"));
    }
    let status = bounded_progress_text(
        event.get("status").and_then(|value| value.as_str())
            .unwrap_or("Downloading model"),
        MAX_PULL_STATUS_BYTES,
    );
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

/// Pulls one model from the fixed loopback Ollama service. Cancellation drops the
/// active request or response stream instead of waiting for the next progress event.
pub async fn pull_model<F>(
    model: &str,
    cancellation: &CancellationToken,
    on_progress: F,
) -> Result<(), String>
where
    F: FnMut(String, u8),
{
    if !valid_pull_model(model) {
        return Err("Invalid Ollama model name. Use an official library identifier such as llama3.2 or qwen2.5:3b.".into());
    }
    if cancellation.is_cancelled() {
        return Err("Model download cancelled by user.".into());
    }
    // Bound the complete HTTP operation as well as the per-chunk idle interval below.
    let client = build_client(Some(Duration::from_secs(20 * 60)))?;
    pull_model_with_client(&client, BASE_URL, model, cancellation, on_progress).await
}

/// Internal transport seam for loopback-only production use and deterministic local HTTP tests.
/// The production caller supplies BASE_URL; the endpoint is never accepted from IPC or UI input.
async fn pull_model_with_client<F>(
    client: &reqwest::Client,
    base_url: &str,
    model: &str,
    cancellation: &CancellationToken,
    mut on_progress: F,
) -> Result<(), String>
where
    F: FnMut(String, u8),
{
    if !valid_pull_model(model) {
        return Err("Invalid Ollama model name.".into());
    }
    if cancellation.is_cancelled() {
        return Err("Model download cancelled by user.".into());
    }

    // Selecting on the token drops reqwest's in-flight send future immediately.
    let response = tokio::select! {
        biased;
        _ = cancellation.cancelled() => {
            return Err("Model download cancelled by user.".into());
        }
        result = client
            .post(format!("{base_url}/api/pull"))
            .json(&PullModelRequest { name: model, stream: true })
            .send() => {
                result.map_err(|error| format!("Could not start local model download: {error}"))?
            }
    };
    if !response.status().is_success() {
        return Err(format!("Local Ollama returned HTTP {} while starting model download.", response.status()));
    }

    let mut response = response;
    let mut pending = Vec::<u8>::new();
    let mut last_percent: i32 = -1;
    let mut last_status = String::new();
    let mut total_stream_bytes = 0usize;
    let mut completed = false;
    loop {
        // Dropping the response on cancellation closes the active HTTP body stream.
        let chunk = match tokio::select! {
            biased;
            _ = cancellation.cancelled() => {
                return Err("Model download cancelled by user.".into());
            }
            result = tokio::time::timeout(Duration::from_secs(120), response.chunk()) => result,
        } {
            Err(_) => return Err("Model download stalled for 2 minutes. Check your connection and retry.".into()),
            Ok(Err(error)) => return Err(format!("Model download stream failed: {error}")),
            Ok(Ok(None)) => break,
            Ok(Ok(Some(chunk))) => chunk,
        };
        total_stream_bytes = total_stream_bytes.checked_add(chunk.len())
            .ok_or_else(|| "Model download progress size overflow.".to_string())?;
        if total_stream_bytes > MAX_PULL_STREAM_BYTES {
            return Err("Model download progress exceeded the 8 MiB safety limit.".into());
        }
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
                if pending.len() > MAX_PULL_EVENT_BYTES {
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
    if cancellation.is_cancelled() {
        return Err("Model download cancelled by user.".into());
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
    fn model_list_validation_rejects_duplicate_and_unsafe_identifiers() {
        let duplicate = vec![
            OllamaModel { name: "llama3.2:latest".into(), size: 10, details: None },
            OllamaModel { name: "llama3.2:latest".into(), size: 20, details: None },
        ];
        assert!(validate_model_list(&duplicate).is_err());

        let unsafe_name = vec![
            OllamaModel { name: "../outside".into(), size: 10, details: None },
        ];
        assert!(validate_model_list(&unsafe_name).is_err());

        let valid = vec![
            OllamaModel { name: "llama3.2:latest".into(), size: 10, details: None },
            OllamaModel { name: "qwen2.5:3b".into(), size: 20, details: None },
        ];
        assert!(validate_model_list(&valid).is_ok());
    }

    #[test]
    fn model_capability_mapping_is_allowlisted_and_unknown_is_preserved() {
        assert_eq!(
            canonical_model_capabilities(&[
                "completion".into(),
                "vision".into(),
                "tools".into(),
                "unrecognised".into(),
            ]),
            vec!["chat".to_string(), "vision".to_string(), "tool_use".to_string()]
        );
        assert_eq!(
            canonical_model_capabilities(&["unrecognised".into()]),
            vec!["unknown".to_string()]
        );
        assert_eq!(canonical_model_capabilities(&[]), vec!["unknown".to_string()]);
    }

    #[test]
    fn context_length_requires_a_bounded_positive_integer() {
        let valid = HashMap::from([(
            "llama.context_length".into(),
            Value::from(32_768_u64),
        )]);
        assert_eq!(context_length_tokens(&valid), Some(32_768));

        let zero = HashMap::from([("llama.context_length".into(), Value::from(0_u64))]);
        assert_eq!(context_length_tokens(&zero), None);

        let oversized = HashMap::from([(
            "llama.context_length".into(),
            Value::from(2_000_000_u64),
        )]);
        assert_eq!(context_length_tokens(&oversized), None);

        let untrusted_string = HashMap::from([(
            "llama.context_length".into(),
            Value::from("32768"),
        )]);
        assert_eq!(context_length_tokens(&untrusted_string), None);
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
    fn pull_progress_text_is_bounded_and_strips_control_characters() {
        assert_eq!(bounded_progress_text("Downloading\nmodel\u{0007}", 64), "Downloadingmodel");
        assert_eq!(bounded_progress_text("abcdef", 3), "abc");
        assert_eq!(bounded_progress_text("م".repeat(8).as_str(), 2), "م");
    }

    #[test]
    fn pull_progress_status_and_errors_are_bounded() {
        let status = parse_pull_progress_line(
            format!(r#"{{"status":"{}"}} "#, "s".repeat(MAX_PULL_STATUS_BYTES + 20)).as_bytes()
        ).unwrap();
        assert_eq!(status.status.len(), MAX_PULL_STATUS_BYTES);
        let error = parse_pull_progress_line(
            format!(r#"{{"error":"{}"}} "#, "e".repeat(MAX_PULL_ERROR_BYTES + 20)).as_bytes()
        ).unwrap_err();
        assert!(error.len() <= MAX_PULL_ERROR_BYTES + "Ollama could not download this model: ".len());
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
    #[tokio::test]
    async fn pull_cancellation_aborts_a_pending_http_response() {
        use tokio::io::AsyncReadExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (accepted_tx, accepted_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 2048];
            let _ = stream.read(&mut request).await;
            let _ = accepted_tx.send(());
            tokio::time::sleep(Duration::from_secs(5)).await;
        });

        let client = build_client(Some(Duration::from_secs(30))).unwrap();
        let cancellation = CancellationToken::new();
        let worker_cancellation = cancellation.clone();
        let base_url = format!("http://{address}");
        let pull = tokio::spawn(async move {
            pull_model_with_client(
                &client,
                &base_url,
                "llama3.2",
                &worker_cancellation,
                |_, _| {},
            ).await
        });

        tokio::time::timeout(Duration::from_secs(3), accepted_rx).await.unwrap().unwrap();
        cancellation.cancel();
        let result = tokio::time::timeout(Duration::from_secs(3), pull).await.unwrap().unwrap();
        assert!(result.unwrap_err().contains("cancelled"));
        server.abort();
    }

    #[tokio::test]
    async fn pull_cancellation_aborts_an_active_progress_stream() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 2048];
            let _ = stream.read(&mut request).await;
            let event = b"{\"status\":\"downloading\",\"total\":100,\"completed\":10}\n";
            let header = format!(
                "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: keep-alive\r\n\r\n{:X}\r\n",
                event.len()
            );
            stream.write_all(header.as_bytes()).await.unwrap();
            stream.write_all(event).await.unwrap();
            stream.write_all(b"\r\n").await.unwrap();
            stream.flush().await.unwrap();
            tokio::time::sleep(Duration::from_secs(5)).await;
        });

        let client = build_client(Some(Duration::from_secs(30))).unwrap();
        let cancellation = CancellationToken::new();
        let worker_cancellation = cancellation.clone();
        let base_url = format!("http://{address}");
        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();
        let pull = tokio::spawn(async move {
            pull_model_with_client(
                &client,
                &base_url,
                "llama3.2",
                &worker_cancellation,
                move |status, percent| {
                    let _ = progress_tx.send((status, percent));
                },
            ).await
        });

        let progress = tokio::time::timeout(Duration::from_secs(3), progress_rx.recv())
            .await.unwrap().unwrap();
        assert_eq!(progress, ("downloading".to_string(), 10));
        cancellation.cancel();
        let result = tokio::time::timeout(Duration::from_secs(3), pull).await.unwrap().unwrap();
        assert!(result.unwrap_err().contains("cancelled"));
        server.abort();
    }


    #[tokio::test]
    async fn local_client_does_not_follow_redirect_responses() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).await;
            stream.write_all(
                b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/redirect-target\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ).await.unwrap();
        });

        let response = build_client(Some(Duration::from_secs(2))).unwrap()
            .get(format!("http://{address}/api/tags"))
            .send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FOUND);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn bounded_json_reader_rejects_oversized_content_length_before_body_read() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).await;
            stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2097153\r\nConnection: close\r\n\r\n"
            ).await.unwrap();
        });

        let response = build_client(Some(Duration::from_secs(2))).unwrap()
            .get(format!("http://{address}/api/tags"))
            .send().await.unwrap();
        let error = parse_limited::<serde_json::Value>(response).await.unwrap_err();
        assert!(error.contains("2 MiB limit"));
        server.await.unwrap();
    }

}
