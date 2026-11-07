mod cloud_gateway;
mod device_identity;
mod everyday_tools;
mod folder_import;
mod local_search;
mod search_cache;
mod history;
mod model_runtime;
mod ollama;
mod resource_runtime;
mod security_store;
pub mod pdf_worker;
mod task_engine;

use task_engine::LocalTaskEngine;

use model_runtime::{discover_models, run_local_model, InferenceResult, ModelInfo};
use serde::Serialize;
use std::{collections::{BTreeSet, HashMap}, sync::Mutex};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio_util::sync::CancellationToken;
use ujani_core::router::{evaluate_mode, ExecutionMode, ModeRoutingDecision, RouteRequest};
use sysinfo::System;
use ujani_core::{FileAnalysis, ImageInfo};
use ujani_core::capability::{plan_local_task, registered_operations, CapabilityRecord, CapabilityState, OperationDescriptor, TaskPlan};
use ujani_core::task_graph::{plan_task_graph, TaskGraph, TaskGraphPlan, TaskNode};
use ujani_core::model_advisor::{recommend_local_model, InstalledModelSize, ModelRecommendation};
use ujani_core::tool_registry::{builtin_tool_manifests, ToolManifest};
use ujani_core::{sha256_hex, validate_model_identifier, NetworkActivity, ReceiptExecutionLocation, ExecutionReceipt};
use std::time::{SystemTime, UNIX_EPOCH};


fn valid_command_identifier(value: &str, max_len: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= max_len
        && value.chars().all(|c| c.is_ascii_alphanumeric() || "._:-".contains(c))
}

/// Evaluates policy only; this command does not execute a task or make network calls.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Preserve the stable named Tauri IPC argument contract.
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
    if !valid_command_identifier(&task_id, 96)
        || !valid_command_identifier(&operation, 96)
        || required_capabilities.len() > 32
        || available_capabilities.len() > 64
        || required_capabilities.iter().chain(available_capabilities.iter())
            .any(|value| !valid_command_identifier(value, 64))
    {
        return Err("Task route contains an invalid identifier or exceeds capability limits.".into());
    }
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
    let accelerator = if ollama_available { ollama::accelerator_observation().await.ok() } else { None };
    let pdf_worker_limits_available = !cfg!(target_os = "macos") || std::path::Path::new("/usr/bin/sandbox-exec").is_file();
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
            state: if accelerator.as_ref().is_some_and(|a| a.gpu_vram_allocated_bytes > 0) { CapabilityState::Available } else { CapabilityState::Unknown },
            source: if accelerator.is_some() { "Ollama /api/ps runtime telemetry".into() } else { "Ollama runtime telemetry unavailable".into() },
            detail: accelerator.as_ref().map(|a| a.evidence.clone()).unwrap_or_else(|| "GPU presence and usable runtime/driver compatibility are not verified; start Ollama and run a model to obtain runtime telemetry".into()),
        },
        CapabilityRecord {
            id: "npu.compute".into(), category: "accelerator".into(),
            state: CapabilityState::Unknown, source: "Not probed".into(),
            detail: "NPU hardware and runtime are not probed; remains Unknown until a supported OS/runtime API confirms a usable NPU".into(),
        },
        CapabilityRecord {
            id: "local.ai.memory_reservation".into(), category: "resource_management".into(),
            state: CapabilityState::Available, source: "UJANI native process-local reservation manager".into(),
            detail: "Interactive local AI pipelines are serialized and require at least 1 GiB available RAM at admission; this is not an OS-level reservation.".into(),
        },
        CapabilityRecord {
            id: "pdf.worker.resource_limits".into(), category: "security".into(),
            state: if pdf_worker_limits_available { CapabilityState::Available } else { CapabilityState::Unavailable }, source: "OS process limits and platform worker sandbox".into(),
            detail: "Unix: 768 MiB address-space, 45-second CPU, 16-file-descriptor, zero file-growth and zero core-dump limits; Linux also enables no_new_privs and seccomp network-syscall denial. macOS additionally launches through sandbox-exec with file writes and network denied. Windows uses 768 MiB process-memory and 45-second CPU Job Object limits with kill-on-close. These constrain the PDF worker, not the complete application.".into(),
        },
        CapabilityRecord {
            id: "fortress.os_isolation".into(), category: "security".into(),
            state: CapabilityState::Unavailable, source: "Full application sandbox adapter not yet installed".into(),
            detail: "Fortress blocks UJANI cloud requests and the PDF worker has OS resource limits, but full application process, filesystem and network isolation is not active on this build.".into(),
        },
    ];
    Ok(CapabilitySnapshot {
        capabilities, operations: registered_operations(),
        available_memory_mb: system.available_memory() / (1024 * 1024),
    })
}

/// Returns the bounded, built-in tool catalogue. Listing metadata never executes a tool or grants permissions.
#[tauri::command]
fn get_tool_catalogue() -> Vec<ToolManifest> {
    builtin_tool_manifests()
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
    if mode.len() > 16 || !matches!(mode.as_str(), "fortress" | "smart") {
        return Err("Workflow mode must be Fortress or Smart.".into());
    }
    if nodes.iter().any(|node| {
        !valid_command_identifier(&node.id, 64)
            || !valid_command_identifier(&node.operation_id, 96)
            || node.depends_on.iter().any(|id| !valid_command_identifier(id, 64))
    }) {
        return Err("Workflow contains an invalid or oversized node identifier.".into());
    }
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
#[allow(clippy::too_many_arguments)] // Preserve the stable named Tauri IPC argument contract.
async fn execute_document_workflow(
 app: AppHandle,
 control: State<'_, TaskControlState>,
 task_id: String,
 file_name: String,
 content: String,
 mode: String,
 model: String,
 translation_language: Option<String>,
) -> Result<WorkflowExecutionResult, String> {
 let language = translation_language.as_deref().unwrap_or("");
 let receipt = local_receipt_context(
  "document.workflow",
  &model,
  &[file_name.as_bytes(), content.as_bytes(), mode.as_bytes(), language.as_bytes()],
 )?;
 run_managed_task(app, &control, task_id, "Document workflow", receipt, NetworkActivity::inferred(vec!["127.0.0.1:11434".into()]), async move {
  if content.len() > 256 * 1024 {
   return Err("Workflow input exceeds the 256 KiB document limit.".into());
  }
  if mode != "fortress" && mode != "smart" {
   return Err("Executable document workflows are local-only (Fortress or Smart).".into());
  }
  let snapshot = get_capability_snapshot().await?;
  let graph = TaskGraph { schema_version: 1, nodes: vec![
   TaskNode { id: "analyse".into(), operation_id: "file.analyze_text".into(), depends_on: vec![] },
   TaskNode { id: "summarise".into(), operation_id: "document.summarize".into(), depends_on: vec!["analyse".into()] },
   TaskNode { id: "translate".into(), operation_id: "translate.text".into(), depends_on: vec!["summarise".into()] },
  ] };
  let plan = plan_task_graph(&graph, &mode, snapshot.available_memory_mb, &snapshot.capabilities);
  if !plan.accepted {
   return Err(format!("Workflow preflight rejected execution: {}", plan.reason));
  }
  // Only the fixed validated sequence is executable; arbitrary graph data is never interpreted as code.
  let result = LocalTaskEngine::process_document(file_name, content, mode, model, translation_language).await?;
  Ok(WorkflowExecutionResult { plan, result })
 }).await
}


/// Sends one explicitly approved prompt to a user-configured OpenAI-compatible provider.
/// The supplied API key is used for this invocation only and is never persisted.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Preserve the stable named Tauri IPC argument contract.
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
 token_parameter: String,
) -> Result<String, String> {
 if task_id.is_empty() || task_id.len() > 96 || !task_id.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)) {
  return Err("Invalid task identifier.".into());
 }
 if !external_data_approved {
  return Err("Cloud transfer was not approved. No request was sent.".into());
 }
 cloud_gateway::validate_request(&endpoint, &model, &api_key, &prompt, &mode, external_data_approved, &token_parameter)?;
 if !validate_model_identifier(&model) {
  return Err("Model identifier cannot be recorded safely in execution provenance.".into());
 }
 let started_at_unix_ms = SystemTime::now().duration_since(UNIX_EPOCH)
  .map_err(|e| e.to_string())?.as_millis() as u64;
 let network_activity = NetworkActivity::inferred(vec![
  cloud_gateway::endpoint_host(&endpoint).unwrap_or_else(|| "configured-provider-endpoint".into())
 ]);
 let input_digest = sha256_hex(prompt.as_bytes());
 let receipt_model_id = model.clone();
 // This reference binds the explicit approval flag to this task, endpoint, model and exact prompt digest.
 let approval_id = sha256_hex(format!("{task_id}:{endpoint}:{model}:{input_digest}:{started_at_unix_ms}").as_bytes());
 let token = CancellationToken::new();
 {
  let mut active = control.active.lock().map_err(|_| "Task registry unavailable.".to_string())?;
  if active.contains_key(&task_id) { return Err("Task identifier is already active.".into()); }
  active.insert(task_id.clone(), token.clone());
 }
 let _active_task_guard = ActiveTaskGuard { control: &control, task_id: task_id.clone() };
 let receipt_slot = match security_store::reserve_execution_receipt_slot(&app, &task_id) {
  Ok(slot) => slot,
  Err(error) => {
   return Err(error);
  }
 };
 // Conservative local daily quota: reserve before sending and never refund failures/cancellations.
 let budget = match security_store::reserve_cloud_tokens(&app, prompt.len(), 8_192) {
  Ok(value) => value,
  Err(error) => {
   return Err(error);
  }
 };
 emit_task_progress(&app, &task_id, "cloud_starting", &format!("Cloud budget reserved; {} tokens remain in today's local allowance.", budget.remaining_tokens), 5);
 emit_task_progress(&app, &task_id, "cloud_sending", "Sending the approved prompt to your configured provider.", 25);
 let request = cloud_gateway::execute(endpoint, model, api_key, prompt, mode, external_data_approved, token_parameter);
 let result = tokio::select! {
  _ = token.cancelled() => Err("Cloud task cancelled by user.".to_string()),
  outcome = tokio::time::timeout(std::time::Duration::from_secs(95), request) => {
   match outcome {
    Ok(value) => value,
    Err(_) => Err("Cloud task exceeded the 95-second time budget.".into()),
   }
  }
 };
 if let Ok(output) = &result {
  let completed_at_unix_ms = SystemTime::now().duration_since(UNIX_EPOCH)
   .map_err(|e| e.to_string())?.as_millis() as u64;
  let receipt = ExecutionReceipt {
   schema_version: 2,
   task_id: task_id.clone(),
   operation_id: "cloud.ai.generate".into(),
   location: ReceiptExecutionLocation::ExternalProvider,
   adapter_id: "byok.cloud-gateway".into(),
   adapter_version: env!("CARGO_PKG_VERSION").into(),
   model_id: Some(receipt_model_id),
    model_version: Some("provider-reported-unverified".into()),
   input_sha256: input_digest,
   output_sha256: sha256_hex(output.as_bytes()),
   started_at_unix_ms,
   completed_at_unix_ms,
   user_approval_id: Some(approval_id),
   network_activity: network_activity.clone(),
  };
  receipt_slot.persist(receipt)
   .map_err(|e| format!("Cloud inference completed, but its provenance receipt could not be stored: {e}"))?;
 }
 match &result {
  Ok(_) => emit_task_progress(&app, &task_id, "completed", "Cloud task completed and provenance recorded.", 100),
  Err(error) if error.contains("cancelled") => emit_task_progress(&app, &task_id, "cancelled", "Cloud request cancelled.", 0),
  Err(error) if error.contains("95-second") => emit_task_progress(&app, &task_id, "timed_out", error, 0),
  Err(error) => emit_task_progress(&app, &task_id, "failed", error, 0),
 }
 result
}
/// Explicitly downloads a named model through the local Ollama service only.
#[tauri::command]
async fn pull_ollama_model(
    app: AppHandle,
    control: State<'_, TaskControlState>,
    task_id: String,
    model: String,
    mode: String,
) -> Result<(), String> {
    if mode == "fortress" {
        return Err("Fortress mode blocks model downloads because they require internet access.".into());
    }
    if mode != "smart" {
        return Err("Model downloads are available only in Smart mode.".into());
    }
    if !ollama::valid_pull_model(&model) {
        return Err("Invalid Ollama model name. Use an official library identifier such as llama3.2.".into());
    }
    let (approval_tx, approval_rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .message(format!("Download {model} from the internet via the local Ollama service?\n\nOllama will contact its model registry."))
        .title("Approve model download")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNo)
        .show(move |approved| { let _ = approval_tx.send(approved); });
    let approved = approval_rx.await.unwrap_or(false);
    if !approved {
        return Err("Model download was not approved. No download was started.".into());
    }
    if task_id.is_empty() || task_id.len() > 96 || !task_id.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)) {
        return Err("Invalid task identifier.".into());
    }
    let token = CancellationToken::new();
    {
        let mut active = control.active.lock().map_err(|_| "Task registry unavailable.".to_string())?;
        if active.contains_key(&task_id) { return Err("Task identifier is already active.".into()); }
        active.insert(task_id.clone(), token.clone());
    }
    let _active_task_guard = ActiveTaskGuard { control: &control, task_id: task_id.clone() };
    emit_task_progress(&app, &task_id, "model_pull_starting", "Starting download from the local Ollama service.", 1);
    let progress_app = app.clone();
    let progress_task = task_id.clone();
    let download = ollama::pull_model(&model, move |message, percent| {
        emit_task_progress(&progress_app, &progress_task, "model_downloading", &message, percent);
    });
    let result = tokio::select! {
        _ = token.cancelled() => Err("Model download cancelled by user. Ollama may need a moment to release the connection.".to_string()),
        outcome = tokio::time::timeout(std::time::Duration::from_secs(20 * 60), download) => {
            match outcome {
                Ok(value) => value,
                Err(_) => Err("Model download exceeded the 20-minute task time budget.".into()),
            }
        },
    };
    match &result {
        Ok(_) => emit_task_progress(&app, &task_id, "completed", "Local model installed successfully.", 100),
        Err(error) if error.contains("cancelled") => emit_task_progress(&app, &task_id, "cancelled", error, 0),
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

/// Removes a task registration even if its command future is cancelled or unwinds.
/// Without this guard, an aborted Tauri command could leave a stale task ID in the
/// process-local registry and make future cancellation/retry behavior inconsistent.
struct ActiveTaskGuard<'a> {
    control: &'a TaskControlState,
    task_id: String,
}

impl Drop for ActiveTaskGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.control.active.lock() {
            active.remove(&self.task_id);
        }
    }
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

#[derive(Debug)]
struct LocalReceiptContext {
    operation_id: &'static str,
    model_id: String,
    input_sha256: String,
}

/// Hash a domain-separated, length-prefixed description of the exact user task
/// inputs. Source bytes are never copied into the receipt ledger.
fn local_receipt_context(
    operation_id: &'static str,
    model_id: &str,
    fields: &[&[u8]],
) -> Result<LocalReceiptContext, String> {
    if !validate_model_identifier(model_id) {
        return Err("Model identifier cannot be recorded safely in execution provenance.".into());
    }
    let mut material = b"UJANI-LOCAL-TASK-INPUT-V1".to_vec();
    for field in [operation_id.as_bytes(), model_id.as_bytes()]
        .into_iter()
        .chain(fields.iter().copied())
    {
        material.extend_from_slice(&(field.len() as u64).to_be_bytes());
        material.extend_from_slice(field);
    }
    Ok(LocalReceiptContext {
        operation_id,
        model_id: model_id.to_string(),
        input_sha256: sha256_hex(&material),
    })
}

/// Runs a native task with cancellation, a hard wall-clock budget and a
/// reserved provenance slot. The ledger stores digests and metadata, never content.
async fn run_managed_task<T, F>(
    app: AppHandle,
    control: &TaskControlState,
    task_id: String,
    label: &'static str,
    receipt_context: LocalReceiptContext,
    network_activity: NetworkActivity,
    future: F,
) -> Result<T, String>
where
    T: Serialize,
    F: std::future::Future<Output = Result<T, String>>,
{
    if !valid_command_identifier(&task_id, 96) {
        return Err("Invalid task identifier.".into());
    }
    // Keep document, PDF, summary and translation tasks under the same native
    // single-task / 1 GiB admission gate as interactive inference.
    let _memory_reservation = resource_runtime::MemoryReservation::acquire(&task_id)?;
    let receipt_slot = security_store::reserve_execution_receipt_slot(&app, &task_id)?;
    let started_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64;
    let token = CancellationToken::new();
    {
        let mut active = control.active.lock().map_err(|_| "Task registry unavailable.".to_string())?;
        if active.contains_key(&task_id) {
            return Err("Task identifier is already active.".into());
        }
        active.insert(task_id.clone(), token.clone());
    }
    let active_task_guard = ActiveTaskGuard { control, task_id: task_id.clone() };
    emit_task_progress(&app, &task_id, "starting", &format!("{label}: starting locally."), 5);
    let result = tokio::select! {
        _ = token.cancelled() => Err("Task cancelled by user.".to_string()),
        outcome = tokio::time::timeout(std::time::Duration::from_secs(240), future) => {
            match outcome {
                Ok(value) => value,
                Err(_) => Err(format!("{label} exceeded the 240-second overall time budget.")),
            }
        }
    };
    drop(active_task_guard);
    if let Ok(value) = &result {
        let output_bytes = serde_json::to_vec(value)
            .map_err(|e| format!("Could not encode local task result for provenance: {e}"))?;
        let completed_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis() as u64;
        let location = match ollama::accelerator_observation().await.ok() {
            Some(observation) if observation.gpu_vram_allocated_bytes > 0 => ReceiptExecutionLocation::LocalGpu,
            _ => ReceiptExecutionLocation::LocalUnverified,
        };
        let receipt = ExecutionReceipt {
            schema_version: 2,
            task_id: task_id.clone(),
            operation_id: receipt_context.operation_id.into(),
            location,
            adapter_id: "ollama.local".into(),
            adapter_version: env!("CARGO_PKG_VERSION").into(),
            model_id: Some(receipt_context.model_id),
            model_version: Some("unknown".into()),
            input_sha256: receipt_context.input_sha256,
            output_sha256: sha256_hex(&output_bytes),
            started_at_unix_ms,
            completed_at_unix_ms,
            user_approval_id: None,
            network_activity,
        };
        if let Err(error) = receipt_slot.persist(receipt) {
            let message = format!("{label} completed, but its provenance receipt could not be stored: {error}");
            emit_task_progress(&app, &task_id, "failed", &message, 0);
            return Err(message);
        }
    }
    match &result {
        Ok(_) => emit_task_progress(&app, &task_id, "completed", &format!("{label}: completed and provenance recorded."), 100),
        Err(error) if error.contains("cancelled") => emit_task_progress(&app, &task_id, "cancelled", &format!("{label}: cancelled."), 0),
        Err(error) if error.contains("240-second") => emit_task_progress(&app, &task_id, "timed_out", error, 0),
        Err(error) => emit_task_progress(&app, &task_id, "failed", error, 0),
    }
    result
}

#[cfg(test)]
mod local_receipt_context_tests {
    use super::local_receipt_context;

    #[test]
    fn active_task_guard_cleans_registry_on_scope_exit() {
        use super::{ActiveTaskGuard, TaskControlState};
        use std::collections::HashMap;
        use std::sync::Mutex;
        use tokio_util::sync::CancellationToken;

        let control = TaskControlState { active: Mutex::new(HashMap::new()) };
        control.active.lock().unwrap().insert("cancel-me".into(), CancellationToken::new());
        {
            let _guard = ActiveTaskGuard { control: &control, task_id: "cancel-me".into() };
            assert!(control.active.lock().unwrap().contains_key("cancel-me"));
        }
        assert!(!control.active.lock().unwrap().contains_key("cancel-me"));
    }

    #[test]
    fn input_digest_is_deterministic_and_bound_to_operation_model_and_fields() {
        let first = local_receipt_context(
            "document.summarize",
            "llama3.2",
            &[&b"report.txt"[..], &b"private source text"[..]],
        ).unwrap();
        let same = local_receipt_context(
            "document.summarize",
            "llama3.2",
            &[&b"report.txt"[..], &b"private source text"[..]],
        ).unwrap();
        let different_operation = local_receipt_context(
            "translate.text",
            "llama3.2",
            &[&b"report.txt"[..], &b"private source text"[..]],
        ).unwrap();
        let different_model = local_receipt_context(
            "document.summarize",
            "qwen2.5",
            &[&b"report.txt"[..], &b"private source text"[..]],
        ).unwrap();

        assert_eq!(first.input_sha256, same.input_sha256);
        assert_ne!(first.input_sha256, different_operation.input_sha256);
        assert_ne!(first.input_sha256, different_model.input_sha256);
        assert_eq!(first.input_sha256.len(), 64);
    }
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
    if !validate_model_identifier(&model) {
        return Err("Model identifier cannot be recorded safely in execution provenance.".into());
    }
    let _memory_reservation = resource_runtime::MemoryReservation::acquire(&task_id)?;
    let receipt_slot = security_store::reserve_execution_receipt_slot(&app, &task_id)?;
    let started_at_unix_ms = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis() as u64;
    let input_digest = sha256_hex(prompt.as_bytes());
    let receipt_model_id = model.clone();
    let token = CancellationToken::new();
    {
        let mut active = control.active.lock().map_err(|_| "Task registry unavailable.".to_string())?;
        if active.contains_key(&task_id) { return Err("Task identifier is already active.".into()); }
        active.insert(task_id.clone(), token.clone());
    }
    let _active_task_guard = ActiveTaskGuard { control: &control, task_id: task_id.clone() };
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
    if let Ok(value) = &result {
        let completed_at_unix_ms = SystemTime::now().duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?.as_millis() as u64;
        let receipt = ExecutionReceipt {
            schema_version: 2,
            task_id: task_id.clone(),
            operation_id: "local.ai.generate".into(),
            location: match ollama::accelerator_observation().await.ok() {
                Some(observation) if observation.gpu_vram_allocated_bytes > 0 => ReceiptExecutionLocation::LocalGpu,
                _ => ReceiptExecutionLocation::LocalUnverified,
            },
            adapter_id: "ollama.local".into(),
            adapter_version: env!("CARGO_PKG_VERSION").into(),
            model_id: Some(receipt_model_id),
            model_version: Some("unknown".into()),
            input_sha256: input_digest,
            output_sha256: sha256_hex(value.output.as_bytes()),
            started_at_unix_ms,
            completed_at_unix_ms,
            user_approval_id: None,
            network_activity: NetworkActivity::inferred(vec!["127.0.0.1:11434".into()]),
        };
        receipt_slot.persist(receipt)
            .map_err(|e| format!("Local inference completed, but its provenance receipt could not be stored: {e}"))?;
    }
    match &result {
        Ok(_) => emit_task_progress(&app, &task_id, "completed", "Local task completed and provenance recorded.", 100),
        Err(error) if error.contains("cancelled") => emit_task_progress(&app, &task_id, "cancelled", "Task cancelled.", 0),
        Err(error) if error.contains("140-second") => emit_task_progress(&app, &task_id, "timed_out", error, 0),
        Err(error) => emit_task_progress(&app, &task_id, "failed", error, 0),
    }
    result
}

#[tauri::command]
fn cancel_active_task(control: State<'_, TaskControlState>, task_id: String) -> Result<bool, String> {
    if !valid_command_identifier(&task_id, 96) {
        return Err("Invalid task identifier.".into());
    }
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
async fn summarize_selected_text(
    app: AppHandle,
    control: State<'_, TaskControlState>,
    task_id: String,
    file_name: String,
    content: String,
    mode: String,
    model: String,
) -> Result<String, String> {
    let receipt = local_receipt_context(
        "document.summarize",
        &model,
        &[file_name.as_bytes(), content.as_bytes(), mode.as_bytes()],
    )?;
    run_managed_task(app, &control, task_id, "Text summary", receipt, NetworkActivity::inferred(vec!["127.0.0.1:11434".into()]),
        LocalTaskEngine::summarize(file_name, content, mode, model)).await
}

/// Runs bounded text analysis followed by local-only Ollama summarisation.
/// The arguments intentionally mirror the stable Tauri IPC payload fields.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn process_selected_document(
    app: AppHandle,
    control: State<'_, TaskControlState>,
    task_id: String,
    file_name: String,
    content: String,
    mode: String,
    model: String,
    translation_language: Option<String>,
) -> Result<task_engine::DocumentPipelineResult, String> {
    let language = translation_language.as_deref().unwrap_or("");
    let receipt = local_receipt_context(
        "document.summarize",
        &model,
        &[file_name.as_bytes(), content.as_bytes(), mode.as_bytes(), language.as_bytes()],
    )?;
    run_managed_task(app, &control, task_id, "Document pipeline", receipt, NetworkActivity::inferred(vec!["127.0.0.1:11434".into()]),
        LocalTaskEngine::process_document(file_name, content, mode, model, translation_language)).await
}

/// Extracts and summarises text from a user-selected PDF, entirely on this computer.
/// The arguments intentionally mirror the stable Tauri IPC payload fields.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn process_selected_pdf(
    app: AppHandle,
    control: State<'_, TaskControlState>,
    task_id: String,
    file_name: String,
    data: Vec<u8>,
    mode: String,
    model: String,
    translation_language: Option<String>,
) -> Result<task_engine::DocumentPipelineResult, String> {
    let language = translation_language.as_deref().unwrap_or("");
    let receipt = local_receipt_context(
        "pdf.extract_and_summarize",
        &model,
        &[file_name.as_bytes(), data.as_slice(), mode.as_bytes(), language.as_bytes()],
    )?;
    run_managed_task(app, &control, task_id, "PDF extraction and summary", receipt, NetworkActivity::inferred(vec!["127.0.0.1:11434".into()]),
        LocalTaskEngine::process_pdf(file_name, data, mode, model, translation_language)).await
}

/// Translates user-provided text through the selected local Ollama model.
#[tauri::command]
async fn translate_text_local(
    app: AppHandle,
    control: State<'_, TaskControlState>,
    task_id: String,
    text: String,
    target_language: String,
    mode: String,
    model: String,
) -> Result<String, String> {
    let receipt = local_receipt_context(
        "translate.text",
        &model,
        &[text.as_bytes(), target_language.as_bytes(), mode.as_bytes()],
    )?;
    run_managed_task(app, &control, task_id, "Local translation", receipt, NetworkActivity::inferred(vec!["127.0.0.1:11434".into()]),
        LocalTaskEngine::translate_local(text, target_language, mode, model)).await
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
    let _reservation = resource_runtime::MemoryReservation::acquire("registered-local-model")?;
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
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            execute_cloud_ai_task,
            get_device_info,
            get_capability_snapshot,
            get_tool_catalogue,
            plan_local_operation,
            plan_task_workflow,
            list_local_models,
            run_local_model_task,
            run_local_task,
            evaluate_task_route,
            get_ollama_status,
            pull_ollama_model,
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
            local_search::find_local_files,
            search_cache::get_local_search_cache_settings,
            search_cache::list_recent_local_searches,
            search_cache::set_local_search_cache_settings,
            search_cache::clear_local_search_cache,
            everyday_tools::get_local_clock,
            everyday_tools::find_duplicate_files,
            history::get_task_history_settings,
            history::set_task_history_enabled,
            history::save_task_history,
            history::list_task_history,
            history::delete_task_history,
            history::clear_task_history,
            security_store::verify_execution_receipt_ledger,
            security_store::list_execution_receipts,
            security_store::get_cloud_budget_status,
            security_store::enroll_trusted_peer,
            security_store::revoke_trusted_peer,
            security_store::list_trusted_peers,
            security_store::verify_and_claim_peer_envelope,
            device_identity::get_or_create_device_identity,
            device_identity::rotate_device_identity,
            device_identity::sign_peer_task_envelope
        ])
        .run(tauri::generate_context!())
        .expect("error while running UJANI Public AI");
}

#[cfg(test)]
mod ipc_input_tests {
    use super::valid_command_identifier;

    #[test]
    fn command_identifiers_are_bounded_and_path_safe() {
        assert!(valid_command_identifier("task-01", 96));
        assert!(!valid_command_identifier("", 96));
        assert!(!valid_command_identifier("../task", 96));
        assert!(!valid_command_identifier("task\nadmin", 96));
        assert!(!valid_command_identifier(&"x".repeat(97), 96));
    }
}
