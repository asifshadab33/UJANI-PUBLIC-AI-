mod cloud_gateway;
mod folder_import;
mod history;
mod model_runtime;
mod ollama;
mod task_engine;

use task_engine::LocalTaskEngine;

use model_runtime::{discover_models, run_local_model, InferenceResult, ModelInfo};
use serde::Serialize;
use std::{collections::{BTreeSet, HashMap}, sync::Mutex};
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;
use ujani_core::router::{evaluate_mode, ExecutionMode, ModeRoutingDecision, RouteRequest};
use sysinfo::System;
use ujani_core::{FileAnalysis, ImageInfo};
use ujani_core::capability::{plan_local_task, registered_operations, CapabilityRecord, CapabilityState, OperationDescriptor, TaskPlan};
use ujani_core::task_graph::{plan_task_graph, TaskGraph, TaskGraphPlan, TaskNode};
use ujani_core::model_advisor::{recommend_local_model, InstalledModelSize, ModelRecommendation};


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

/// Advisory model-fit estimates; no model is automatically selected or loaded.
#[tauri::command]
async fn get_model_recommendation() -> Result<ModelRecommendation, String> {
 let mut system=System::new(); system.refresh_memory();
 let available_memory_mb=system.available_memory()/(1024*1024);
 let models=if ollama::is_available().await {ollama::list_models().await?.into_iter().map(|m|InstalledModelSize{name:m.name,size_bytes:m.size}).collect::<Vec<_>>()}else{Vec::new()};
 Ok(recommend_local_model(&models,available_memory_mb))
}

#[derive(Serialize)]
struct WorkflowExecutionResult { plan: TaskGraphPlan, result: task_engine::DocumentPipelineResult }

/// Validates and executes only the fixed local analyse → summarise → optional translate workflow.
#[tauri::command]
async fn execute_document_workflow(file_name:String,content:String,mode:String,model:String,translation_language:Option<String>)->Result<WorkflowExecutionResult,String>{
 if content.len()>256*1024{return Err("Workflow input exceeds the 256 KiB document limit.".into());}
 if mode!="fortress"&&mode!="smart"{return Err("Executable document workflows are local-only (Fortress or Smart).".into());}
 let snapshot=get_capability_snapshot().await?;
 let graph=TaskGraph{schema_version:1,nodes:vec![
 TaskNode{id:"analyse".into(),operation_id:"file.analyze_text".into(),depends_on:vec![]},
 TaskNode{id:"summarise".into(),operation_id:"document.summarize".into(),depends_on:vec!["analyse".into()]},
 TaskNode{id:"translate".into(),operation_id:"translate.text".into(),depends_on:vec!["summarise".into()]}]};
 let plan=plan_task_graph(&graph,&mode,snapshot.available_memory_mb,&snapshot.capabilities);
 if !plan.accepted{return Err(format!("Workflow preflight rejected execution: {}",plan.reason));}
 // Do not interpret arbitrary graph data as executable code. This command runs this fixed sequence only.
 let result=LocalTaskEngine::process_document(file_name,content,mode,model,translation_language).await?;
 Ok(WorkflowExecutionResult{plan,result})
}


/// Sends one explicitly approved prompt to a user-configured OpenAI-compatible provider.
/// The supplied API key is used for this invocation only and is never persisted.
#[tauri::command]
async fn execute_cloud_ai_task(
 app: AppHandle,
 control: State<'_, TaskControlState>,
 task_id: String,
 endpoint: String,
 model: String,
 api_key: String,
 prompt: String,
 mode: String,
 external_data_approved: bool,
) -> Result<String, String> {
 if task_id.is_empty() || task_id.len() > 96 || !task_id.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)) {
  return Err("Invalid task identifier.".into());
 }
 if mode == "fortress" { return Err("Fortress mode is local-only. Cloud transfer is blocked by native policy.".into()); }
 if !matches!(mode.as_str(), "smart" | "worker" | "partner") { return Err("Unknown cloud execution mode.".into()); }
 let token = CancellationToken::new();
 {
  let mut active = control.active.lock().map_err(|_| "Task registry unavailable.".to_string())?;
  if active.contains_key(&task_id) { return Err("Task identifier is already active.".into()); }
  active.insert(task_id.clone(), token.clone());
 }
 emit_task_progress(&app, &task_id, "cloud_starting", "Preparing the user-approved cloud request.", 5);
 emit_task_progress(&app, &task_id, "cloud_sending", "Sending the approved prompt to your configured provider.", 25);
 let request = cloud_gateway::execute(endpoint, model, api_key, prompt, mode, external_data_approved);
 let result = tokio::select! {
  _ = token.cancelled() => Err("Cloud task cancelled by user.".to_string()),
  outcome = tokio::time::timeout(std::time::Duration::from_secs(95), request) => {
   match outcome {
    Ok(value) => value,
    Err(_) => Err("Cloud task exceeded the 95-second time budget.".into()),
   }
  }
 };
 if let Ok(mut active) = control.active.lock() { active.remove(&task_id); }
 match &result {
  Ok(_) => emit_task_progress(&app, &task_id, "completed", "Cloud task completed.", 100),
  Err(error) if error.contains("cancelled") => emit_task_progress(&app, &task_id, "cancelled", "Cloud request cancelled.", 0),
  Err(error) if error.contains("95-second") => emit_task_progress(&app, &task_id, "timed_out", error, 0),
  Err(error) => emit_task_progress(&app, &task_id, "failed", error, 0),
 }
 result
}
/// Checks only the local Ollama service at 127.0.0.1:11434.
#[tauri::command]
async fn get_ollama_status() -> Result<OllamaStatus, String> {
    let available = ollama::is_available().await;
    let models = if available { ollama::list_models().await? } else { Vec::new() };
    Ok(OllamaStatus { available, models })
}


#[derive(Default)]
struct TaskControlState {
    active: Mutex<HashMap<String, CancellationToken>>,
}

#[derive(Clone, Serialize)]
struct TaskProgressEvent {
    task_id: String,
    stage: String,
    message: String,
    progress_percent: u8,
}

fn emit_task_progress(app: &AppHandle, task_id: &str, stage: &str, message: &str, progress_percent: u8) {
    let _ = app.emit("ujani-task-progress", TaskProgressEvent {
        task_id: task_id.to_string(),
        stage: stage.to_string(),
        message: message.to_string(),
        progress_percent,
    });
}

/// Interactive local chat with a cancellation token and an overall time budget.
#[tauri::command]
async fn execute_local_ai_task(
    app: AppHandle,
    control: State<'_, TaskControlState>,
    task_id: String,
    prompt: String,
    mode: String,
    model: String,
) -> Result<task_engine::AiTaskResult, String> {
    if task_id.is_empty() || task_id.len() > 96 || !task_id.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)) {
        return Err("Invalid task identifier.".into());
    }
    let token = CancellationToken::new();
    {
        let mut active = control.active.lock().map_err(|_| "Task registry unavailable.".to_string())?;
        if active.contains_key(&task_id) { return Err("Task identifier is already active.".into()); }
        active.insert(task_id.clone(), token.clone());
    }
    emit_task_progress(&app, &task_id, "starting", "Checking local policy and preparing inference.", 5);
    emit_task_progress(&app, &task_id, "inference", "Local model request sent; waiting for Ollama response.", 25);
    let execution = LocalTaskEngine::execute(prompt, mode, model);
    let result = tokio::select! {
        _ = token.cancelled() => Err("Task cancelled by user.".to_string()),
        outcome = tokio::time::timeout(std::time::Duration::from_secs(140), execution) => {
            match outcome {
                Ok(value) => value,
                Err(_) => Err("Local task exceeded the 140-second overall time budget. Try a smaller prompt or faster model.".into()),
            }
        }
    };
    if let Ok(mut active) = control.active.lock() { active.remove(&task_id); }
    match &result {
        Ok(_) => emit_task_progress(&app, &task_id, "completed", "Local task completed.", 100),
        Err(error) if error.contains("cancelled") => emit_task_progress(&app, &task_id, "cancelled", "Task cancelled.", 0),
        Err(error) if error.contains("140-second") => emit_task_progress(&app, &task_id, "timed_out", error, 0),
        Err(error) => emit_task_progress(&app, &task_id, "failed", error, 0),
    }
    result
}

#[tauri::command]
fn cancel_active_task(control: State<'_, TaskControlState>, task_id: String) -> Result<bool, String> {
    let active = control.active.lock().map_err(|_| "Task registry unavailable.".to_string())?;
    if let Some(token) = active.get(&task_id) { token.cancel(); Ok(true) } else { Ok(false) }
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
        .manage(TaskControlState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            execute_cloud_ai_task,
            get_device_info,
            get_capability_snapshot,
            plan_local_operation,
            plan_task_workflow,
            list_local_models,
            run_local_model_task,
            run_local_task,
            evaluate_task_route,
            get_ollama_status,
            get_model_recommendation,
            execute_document_workflow,
            execute_local_ai_task,
            cancel_active_task,
            analyze_text_file,
            summarize_selected_text,
            process_selected_document,
            process_selected_pdf,
            translate_text_local,
            inspect_image_file,
            folder_import::process_selected_folder,
            history::get_task_history_settings,
            history::set_task_history_enabled,
            history::save_task_history,
            history::list_task_history,
            history::delete_task_history,
            history::clear_task_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running UJANI Public AI");
}
