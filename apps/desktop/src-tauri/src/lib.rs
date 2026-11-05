mod model_runtime;
mod ollama;
mod task_engine;

use task_engine::LocalTaskEngine;

use model_runtime::{discover_models, run_local_model, InferenceResult, ModelInfo};
use serde::Serialize;
use std::collections::BTreeSet;
use ujani_core::router::{evaluate_mode, ExecutionMode, ModeRoutingDecision, RouteRequest};
use sysinfo::System;
use ujani_core::{FileAnalysis, ImageInfo};


/// Evaluates policy only; this command does not execute a task or make network calls.
#[tauri::command]
fn evaluate_task_route(
    task_id: String,
    operation: String,
    mode: String,
    required_capabilities: Vec<String>,
    available_capabilities: Vec<String>,
    local_memory_required_mb: u64,
    available_memory_mb: u64,
    external_data_approved: bool,
    requested_collaboration_rounds: u8,
) -> Result<ModeRoutingDecision, String> {
    let mode = match mode.as_str() {
        "fortress" => ExecutionMode::Fortress,
        "worker" => ExecutionMode::Worker,
        "smart" => ExecutionMode::Smart,
        "partner" => ExecutionMode::Partner,
        _ => return Err("Unknown execution mode.".into()),
    };
    let request = RouteRequest {
        task_id, operation, mode,
        required_capabilities: required_capabilities.into_iter().collect::<BTreeSet<_>>(),
        available_capabilities: available_capabilities.into_iter().collect::<BTreeSet<_>>(),
        local_memory_required_mb, available_memory_mb,
        external_data_approved, network_capability_available: false,
        requested_collaboration_rounds,
    };
    Ok(evaluate_mode(&request))
}


#[derive(Serialize)]
struct OllamaStatus {
    available: bool,
    models: Vec<ollama::OllamaModel>,
}

/// Checks only the local Ollama service at 127.0.0.1:11434.
#[tauri::command]
async fn get_ollama_status() -> Result<OllamaStatus, String> {
    let available = ollama::is_available().await;
    let models = if available { ollama::list_models().await? } else { Vec::new() };
    Ok(OllamaStatus { available, models })
}

/// Real local inference only. Smart and Fortress never fall back to cloud.
#[tauri::command]
async fn execute_local_ai_task(prompt: String, mode: String, model: String) -> Result<task_engine::AiTaskResult, String> {
    LocalTaskEngine::execute(prompt, mode, model).await
}

/// Processes text supplied by the user through the UI; never opens a path.
#[tauri::command]
fn analyze_text_file(file_name: String, content: String) -> Result<FileAnalysis, String> {
    LocalTaskEngine::analyze_text(&file_name, &content)
}


/// Summarises explicitly selected text with Ollama on loopback only.
#[tauri::command]
async fn summarize_selected_text(file_name: String, content: String, mode: String, model: String) -> Result<String, String> {
    LocalTaskEngine::summarize(file_name, content, mode, model).await
}

/// Runs bounded text analysis followed by local-only Ollama summarisation.
#[tauri::command]
async fn process_selected_document(
    file_name: String,
    content: String,
    mode: String,
    model: String,
) -> Result<task_engine::DocumentPipelineResult, String> {
    LocalTaskEngine::process_document(file_name, content, mode, model).await
}

/// Inspects image bytes supplied by the user through the UI; never opens a path.
#[tauri::command]
fn inspect_image_file(file_name: String, data: Vec<u8>, file_size_bytes: u64) -> Result<ImageInfo, String> {
    LocalTaskEngine::inspect_image(&file_name, &data, file_size_bytes)
}

#[derive(Serialize)]
struct DeviceInfo {
    os: String,
    architecture: String,
    logical_cpus: usize,
    total_memory_mb: u64,
    available_memory_mb: u64,
}

#[derive(Serialize)]
struct RunResult {
    operation: String,
    output: String,
    execution: String,
}

#[tauri::command]
fn get_device_info() -> DeviceInfo {
    let mut system = System::new();
    system.refresh_cpu();
    system.refresh_memory();
    DeviceInfo {
        os: System::name().unwrap_or_else(|| std::env::consts::OS.to_string()),
        architecture: std::env::consts::ARCH.to_string(),
        logical_cpus: system.cpus().len(),
        total_memory_mb: system.total_memory() / (1024 * 1024),
        available_memory_mb: system.available_memory() / (1024 * 1024),
    }
}

/// Returns only models registered by the application. No model files are scanned.
#[tauri::command]
fn list_local_models() -> Vec<ModelInfo> {
    let mut system = System::new();
    system.refresh_cpu();
    discover_models(!system.cpus().is_empty())
}

/// Runs an explicitly selected local adapter. No network or cloud fallback exists.
#[tauri::command]
fn run_local_model_task(model_id: String, prompt: String) -> Result<InferenceResult, String> {
    let mut system = System::new();
    system.refresh_cpu();
    run_local_model(&model_id, &prompt, !system.cpus().is_empty()).map_err(|error| error.to_string())
}

/// Narrow demonstration operation: no shell, scripts, filesystem or network access.
#[tauri::command]
fn run_local_task(request: String) -> Result<RunResult, String> {
    let trimmed = request.trim();
    if trimmed.is_empty() {
        return Err("Request cannot be empty.".into());
    }
    if trimmed.len() > 20_000 {
        return Err("Request is too long for the local demonstration tool.".into());
    }
    let word_count = trimmed.split_whitespace().count();
    Ok(RunResult {
        operation: "local.word_count".into(),
        output: format!("Your request contains {word_count} words. This is a deterministic local demonstration, not an AI-generated answer."),
        execution: "Native local runtime".into(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_device_info,
            list_local_models,
            run_local_model_task,
            run_local_task,
            evaluate_task_route,
            get_ollama_status,
            execute_local_ai_task,
            analyze_text_file,
            summarize_selected_text,
            process_selected_document,
            inspect_image_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running UJANI Public AI");
}
