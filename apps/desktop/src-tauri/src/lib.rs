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
use ujani_core::capability::{plan_local_task, registered_operations, CapabilityRecord, CapabilityState, OperationDescriptor, TaskPlan};
use ujani_core::task_graph::{plan_task_graph, TaskGraph, TaskGraphPlan, TaskNode};


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
struct CapabilitySnapshot {
    capabilities: Vec<CapabilityRecord>,
    operations: Vec<OperationDescriptor>,
    available_memory_mb: u64,
}

/// Read-only capability inventory. Unknown GPU/NPU capability is never treated as available.
#[tauri::command]
async fn get_capability_snapshot() -> Result<CapabilitySnapshot, String> {
    let mut system = System::new();
    system.refresh_cpu();
    system.refresh_memory();
    let cpu_available = std::thread::available_parallelism().is_ok();
    let ollama_available = ollama::is_available().await;
    let capabilities = vec![
        CapabilityRecord {
            id: "cpu".into(), category: "compute".into(),
            state: if cpu_available { CapabilityState::Available } else { CapabilityState::Unavailable },
            source: "Rust standard library / OS".into(),
            detail: format!("{} logical CPU threads reported by the OS", system.cpus().len()),
        },
        CapabilityRecord {
            id: "memory".into(), category: "system".into(),
            state: if system.available_memory() > 0 { CapabilityState::Available } else { CapabilityState::Unknown },
            source: "sysinfo / OS".into(),
            detail: format!("{} MiB currently available; varies with other applications", system.available_memory() / (1024 * 1024)),
        },
        CapabilityRecord {
            id: "ollama.local".into(), category: "model_runtime".into(),
            state: if ollama_available { CapabilityState::Available } else { CapabilityState::Unavailable },
            source: "HTTP health check to 127.0.0.1:11434".into(),
            detail: if ollama_available { "Local Ollama service responded".into() } else { "Local Ollama service not detected".into() },
        },
        CapabilityRecord {
            id: "pdf.text_extraction".into(), category: "document".into(),
            state: CapabilityState::Available, source: "Bundled PDF text parser".into(),
            detail: "Text-based PDFs only; scanned-page OCR is not implemented".into(),
        },
        CapabilityRecord {
            id: "image.header_inspection".into(), category: "image".into(),
            state: CapabilityState::Available, source: "UJANI bounded image header parser".into(),
            detail: "Metadata/header inspection only; no OCR or visual model inference".into(),
        },
        CapabilityRecord {
            id: "gpu.compute".into(), category: "accelerator".into(),
            state: CapabilityState::Unknown, source: "Not probed".into(),
            detail: "GPU presence and usable runtime/driver compatibility are not verified".into(),
        },
        CapabilityRecord {
            id: "npu.compute".into(), category: "accelerator".into(),
            state: CapabilityState::Unknown, source: "Not probed".into(),
            detail: "NPU presence and supported runtime are not verified".into(),
        },
    ];
    Ok(CapabilitySnapshot {
        capabilities, operations: registered_operations(),
        available_memory_mb: system.available_memory() / (1024 * 1024),
    })
}

/// Plans an allow-listed local operation. This command does not execute or reserve resources.
#[tauri::command]
async fn plan_local_operation(operation_id: String, mode: String) -> Result<TaskPlan, String> {
    let snapshot = get_capability_snapshot().await?;
    Ok(plan_local_task(&operation_id, &mode, snapshot.available_memory_mb, &snapshot.capabilities))
}


/// Validates a bounded, versioned local workflow graph. This command never executes it.
#[tauri::command]
async fn plan_task_workflow(nodes: Vec<TaskNode>, mode: String) -> Result<TaskGraphPlan, String> {
    if nodes.len() > 32 {
        return Err("Workflow is limited to 32 steps.".into());
    }
    if nodes.iter().any(|node| node.depends_on.len() > 16) {
        return Err("Each workflow step is limited to 16 dependencies.".into());
    }
    let snapshot = get_capability_snapshot().await?;
    let graph = TaskGraph { schema_version: 1, nodes };
    Ok(plan_task_graph(&graph, &mode, snapshot.available_memory_mb, &snapshot.capabilities))
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
    translation_language: Option<String>,
) -> Result<task_engine::DocumentPipelineResult, String> {
    LocalTaskEngine::process_document(file_name, content, mode, model, translation_language).await
}

/// Extracts and summarises text from a user-selected PDF, entirely on this computer.
#[tauri::command]
async fn process_selected_pdf(
    file_name: String,
    data: Vec<u8>,
    mode: String,
    model: String,
    translation_language: Option<String>,
) -> Result<task_engine::DocumentPipelineResult, String> {
    LocalTaskEngine::process_pdf(file_name, data, mode, model, translation_language).await
}

/// Translates user-provided text through the selected local Ollama model.
#[tauri::command]
async fn translate_text_local(
    text: String,
    target_language: String,
    mode: String,
    model: String,
) -> Result<String, String> {
    LocalTaskEngine::translate_local(text, target_language, mode, model).await
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
    system_locale: String,
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
        system_locale: sys_locale::get_locale().unwrap_or_else(|| "Unknown".into()),
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
            get_capability_snapshot,
            plan_local_operation,
            plan_task_workflow,
            list_local_models,
            run_local_model_task,
            run_local_task,
            evaluate_task_route,
            get_ollama_status,
            execute_local_ai_task,
            analyze_text_file,
            summarize_selected_text,
            process_selected_document,
            process_selected_pdf,
            translate_text_local,
            inspect_image_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running UJANI Public AI");
}
