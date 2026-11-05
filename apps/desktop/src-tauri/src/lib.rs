mod model_runtime;

use model_runtime::{discover_models, run_local_model, InferenceResult, ModelInfo};
use serde::Serialize;
use std::collections::BTreeSet;
use ujani_core::router::{evaluate_mode, ExecutionMode, ModeRoutingDecision, RouteRequest};
use sysinfo::System;


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
    network_capability_available: bool,
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
        external_data_approved, network_capability_available,
        requested_collaboration_rounds,
    };
    Ok(evaluate_mode(&request))
}

#[derive(Serialize)]
struct DeviceInfo {
    os: String,
    architecture: String,
    logical_cpus: usize,
    total_memory_mb: u64,
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
            evaluate_task_route
        ])
        .run(tauri::generate_context!())
        .expect("error while running UJANI Public AI");
}
