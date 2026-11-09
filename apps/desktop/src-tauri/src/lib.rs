mod cloud_gateway;
mod native_consent;
// Peer identity commands are retained for the opt-in peer-preview build only.
#[allow(dead_code)]
mod device_identity;
mod everyday_tools;
mod file_insights;
mod folder_import;
mod local_search;
mod search_cache;
mod history;
mod model_runtime;
mod ollama;
mod resource_runtime;
mod security_providers;
mod safe_fs;
// Peer enrollment/replay commands are not registered in the default IPC surface.
#[allow(dead_code, unused_imports)]
mod security_store;
mod task_engine;

use task_engine::LocalTaskEngine;

use model_runtime::{discover_models, run_local_model, InferenceResult, ModelInfo};
use serde::Serialize;
use std::{collections::{BTreeSet, HashMap}, sync::{Arc, Condvar, Mutex, OnceLock}, time::{Duration, Instant}};
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
  return Err("Cloud transfer was not approved in UJANI. No request was sent.".into());
 }
 cloud_gateway::validate_request(&endpoint, &model, &api_key, &prompt, &mode, external_data_approved, &token_parameter)?;
 let provider_host = cloud_gateway::endpoint_host(&endpoint)
  .ok_or_else(|| "Cloud endpoint host could not be validated. No request was sent.".to_string())?;
 let preview: String = prompt.chars().take(600).collect();
 let truncated = prompt.chars().count() > 600;
 let consent_message = format!(
  "UJANI is about to send this single request to an external provider.\n\nProvider host: {provider_host}\nModel: {model}\nPrompt size: {} characters\n\nPrompt preview:\n{}{}\n\nThe provider will receive the prompt and may process it under its own privacy and retention terms. Continue?",
  prompt.chars().count(), preview, if truncated { "\n[preview truncated]" } else { "" }
 );
 // Register before showing consent so the user can cancel while the native dialog is pending.
 let token = CancellationToken::new();
 {
  let mut active = control.active.lock().map_err(|_| "Task registry unavailable.".to_string())?;
  if active.contains_key(&task_id) { return Err("Task identifier is already active.".into()); }
  active.insert(task_id.clone(), token.clone());
 }
 let _active_task_guard = ActiveTaskGuard { control: &control, task_id: task_id.clone() };
 if !native_consent::confirm_native(
  &app,
  "Approve one-time cloud data transfer",
  consent_message,
  token.clone(),
  Duration::from_secs(120),
 ).await {
  return Err("Cloud transfer consent was declined, cancelled, or timed out. No request was sent.".into());
 }
 if !validate_model_identifier(&model) {
  return Err("Model identifier cannot be recorded safely in execution provenance.".into());
 }
 let started_at_unix_ms = SystemTime::now().duration_since(UNIX_EPOCH)
  .map_err(|e| e.to_string())?.as_millis() as u64;
 let network_activity = NetworkActivity::inferred(vec![provider_host]);
 let input_digest = sha256_hex(prompt.as_bytes());
 let receipt_model_id = model.clone();
 // This reference binds the native approval to this task, endpoint, model and exact prompt digest.
 let approval_id = sha256_hex(format!("{task_id}:{endpoint}:{model}:{input_digest}:{started_at_unix_ms}").as_bytes());
 flush_pending_receipts(&app).await;
 let receipt_slot = security_store::reserve_execution_receipt_slot(&app, &task_id)?;
 if let Some(warning) = receipt_slot.archive_warning() {
  emit_task_progress(&app, &task_id, "receipt_archive_warning", warning, 3);
 }
 let mut budget = security_store::reserve_cloud_tokens(&app, prompt.len(), 8_192)?;
 emit_task_progress(&app, &task_id, "cloud_starting", &format!("Cloud budget reserved; {} tokens remain in today's local allowance.", budget.status().remaining_tokens), 5);
 emit_task_progress(&app, &task_id, "cloud_sending", "Sending the approved prompt to your configured provider.", 25);
 let request = cloud_gateway::execute(endpoint, model, api_key, prompt, mode, external_data_approved, token_parameter, || budget.mark_dispatched());
 let mut receipt_warning = None;
 let result = tokio::select! {
  // If cancellation is already pending, do not poll the gateway or mark the budget dispatched.
  biased;
  _ = token.cancelled() => Err("Cloud task cancelled by user.".to_string()),
  outcome = tokio::time::timeout(Duration::from_secs(95), request) => {
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
   output_sha256: sha256_hex(output.text.as_bytes()),
   started_at_unix_ms,
   completed_at_unix_ms,
   user_approval_id: Some(approval_id),
   network_activity: network_activity.clone(),
  };
  receipt_warning = persist_receipt_or_warn("Cloud inference", receipt_slot, receipt);
 }
 match &result {
  Ok(_) => emit_task_progress(&app, &task_id, "completed", "Cloud task completed.", 100),
  Err(error) if error.contains("cancelled") => emit_task_progress(&app, &task_id, "cancelled", "Cloud request cancelled.", 0),
  Err(error) if error.contains("95-second") => emit_task_progress(&app, &task_id, "timed_out", error, 0),
  Err(error) => emit_task_progress(&app, &task_id, "failed", error, 0),
 }
 if let Some(message) = receipt_warning { emit_task_progress(&app, &task_id, "receipt_warning", &message, 100); }
 let budget_warning = if let Ok(output) = &result {
  match budget.settle(output.total_tokens) {
   Ok(()) => None,
   Err(error) => Some(format!("Cloud usage reconciliation failed; the conservative reservation remains counted: {error}")),
  }
 } else {
  drop(budget); // Unsent requests refund automatically; dispatched requests remain conservatively charged.
  None
 };
 if let Some(message) = budget_warning { emit_task_progress(&app, &task_id, "cloud_budget_warning", &message, 100); }
 result.map(|output| output.text)
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
    /// Bounded, short-lived cancellation requests that arrive before PDF task registration.
    pending_cancellations: Mutex<HashMap<String, Instant>>,
    /// Recently completed PDF IDs reject late cancellation during receipt/history persistence.
    finished_pdf_tasks: Mutex<HashMap<String, Instant>>,
}

/// Removes a task registration even if its command future is cancelled or unwinds.
/// Without this guard, an aborted Tauri command could leave a stale task ID in the
/// process-local registry and make future cancellation/retry behavior inconsistent.
struct ActiveTaskGuard<'a> {
    control: &'a TaskControlState,
    task_id: String,
}

impl Drop for ActiveTaskGuard<'_> {
    /// Removes active registration and remembers completion to reject late PDF cancellation.
    fn drop(&mut self) {
        // Keep the active lock until the finished marker is committed. Otherwise a
        // concurrent cancel/re-register can observe a gap after active removal and
        // before completion is recorded, leaving a stale pending cancellation.
        if let Ok(mut active) = self.control.active.lock() {
            active.remove(&self.task_id);
            if self.task_id.starts_with("pdf-") {
                if let Ok(mut finished) = self.control.finished_pdf_tasks.lock() {
                    let now = Instant::now();
                    finished.retain(|_, completed_at| now.duration_since(*completed_at) <= FINISHED_PDF_TASK_TTL);
                    if !finished.contains_key(&self.task_id) && finished.len() >= MAX_FINISHED_PDF_TASKS {
                        if let Some(oldest) = finished.iter().min_by_key(|(_, at)| **at).map(|(id, _)| id.clone()) {
                            finished.remove(&oldest);
                        }
                    }
                    finished.insert(self.task_id.clone(), now);
                }
            }
        }
    }
}

const MAX_PENDING_CANCELLATIONS: usize = 64;
const PENDING_CANCELLATION_TTL: Duration = Duration::from_secs(30);
const MAX_FINISHED_PDF_TASKS: usize = 256;
const FINISHED_PDF_TASK_TTL: Duration = Duration::from_secs(5 * 60);

/// A task registration that can be created before bounded upload preparation.
struct ActiveTaskRegistration<'a> { task_id: String, token: CancellationToken, _guard: ActiveTaskGuard<'a> }

/// Rejects a task cancelled before an expensive preparation step starts.
fn ensure_task_not_cancelled(token: &CancellationToken) -> Result<(), String> {
    if token.is_cancelled() { Err("Task cancelled by user.".into()) } else { Ok(()) }
}

/// Registers a task, consumes a pending cancellation, and rejects reused completed PDF IDs.
fn register_active_task<'a>(control: &'a TaskControlState, task_id: &str) -> Result<ActiveTaskRegistration<'a>, String> {
    if !valid_command_identifier(task_id, 96) { return Err("Invalid task identifier.".into()); }
    // Preserve the stricter identifier contract required by the resource reservation queue.
    resource_runtime::validate_task_id(task_id)?;
    let token = CancellationToken::new();
    // Keep active -> pending lock order consistent with request_task_cancellation.
    let mut active = control.active.lock().map_err(|_| "Task registry unavailable.".to_string())?;
    if active.contains_key(task_id) { return Err("Task identifier is already active.".into()); }
    let now = Instant::now();
    let mut finished = control.finished_pdf_tasks.lock().map_err(|_| "Finished PDF registry unavailable.".to_string())?;
    finished.retain(|_, completed_at| now.duration_since(*completed_at) <= FINISHED_PDF_TASK_TTL);
    if finished.contains_key(task_id) {
        return Err("PDF task identifier was already completed; use a fresh identifier.".into());
    }
    drop(finished);
    let mut pending = control.pending_cancellations.lock().map_err(|_| "Pending cancellation registry unavailable.".to_string())?;
    pending.retain(|_, requested_at| now.duration_since(*requested_at) <= PENDING_CANCELLATION_TTL);
    let was_cancelled = pending.remove(task_id).is_some();
    active.insert(task_id.to_string(), token.clone());
    drop(pending); drop(active);
    if was_cancelled { token.cancel(); }
    Ok(ActiveTaskRegistration { task_id: task_id.to_string(), token, _guard: ActiveTaskGuard { control, task_id: task_id.to_string() } })
}

#[derive(Clone, Serialize)]
struct TaskProgressEvent {
    task_id: String,
    stage: String,
    message: String,
    progress_percent: u8,
}

#[derive(Clone, Serialize)]
struct ChatDeltaEvent {
    task_id: String,
    delta: String,
}

fn emit_chat_delta(app: &AppHandle, task_id: &str, delta: &str) {
    let _ = app.emit("ujani-chat-delta", ChatDeltaEvent {
        task_id: task_id.to_string(),
        delta: delta.to_string(),
    });
}

fn emit_task_progress(app: &AppHandle, task_id: &str, stage: &str, message: &str, progress_percent: u8) {
    let _ = app.emit("ujani-task-progress", TaskProgressEvent {
        task_id: task_id.to_string(),
        stage: stage.to_string(),
        message: message.to_string(),
        progress_percent,
    });
}

/// A pending receipt retains its original durable capacity reservation so retries
/// do not fail by trying to insert the same task slot again.
struct PendingReceipt {
    receipt: ExecutionReceipt,
    slot: security_store::ReceiptSlotReservation,
    attempts: u8,
}

/// Tracks queued and currently retrying provenance receipts for bounded capacity accounting.
struct PendingReceiptState {
    queued: Vec<PendingReceipt>,
    in_flight: usize,
    flush_in_progress: bool,
}

static PENDING_RECEIPTS: OnceLock<Mutex<PendingReceiptState>> = OnceLock::new();
const MAX_PENDING_RECEIPTS: usize = 100;
const RECEIPT_RETRY_BUDGET: Duration = Duration::from_secs(3);
const MAX_RECEIPT_RETRY_ATTEMPTS: u8 = 5;
static PENDING_RECEIPTS_CHANGED: OnceLock<Condvar> = OnceLock::new();

fn pending_receipts_changed() -> &'static Condvar {
    PENDING_RECEIPTS_CHANGED.get_or_init(Condvar::new)
}

fn next_receipt_retry_attempt(attempts: u8) -> Option<u8> {
    let next = attempts.saturating_add(1);
    (next < MAX_RECEIPT_RETRY_ATTEMPTS).then_some(next)
}

/// Returns the process-local bounded queue and its in-flight capacity accounting.
fn pending_receipts() -> &'static Mutex<PendingReceiptState> {
    PENDING_RECEIPTS.get_or_init(|| Mutex::new(PendingReceiptState { queued: Vec::new(), in_flight: 0, flush_in_progress: false }))
}

/// Adds a failed receipt and its still-live reservation without exceeding the
/// combined queued and in-flight bound.
fn enqueue_pending_receipt(receipt: ExecutionReceipt, slot: security_store::ReceiptSlotReservation) -> bool {
    let mut state = pending_receipts().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.queued.len().saturating_add(state.in_flight) >= MAX_PENDING_RECEIPTS { return false; }
    // The first persistence attempt already failed before the item entered this queue.
    state.queued.push(PendingReceipt { receipt, slot, attempts: 1 });
    true
}

/// Retries receipts on a blocking thread with bounded capacity and a bounded per-entry retry count.
/// Transient failures requeue the batch; a poison receipt is discarded after five attempts.
fn flush_pending_receipts_blocking(app: &AppHandle, budget: Duration, max_receipts: usize) {
    let batch = {
        let mut state = pending_receipts().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        // Only one worker may own the retry batch. Shutdown observes this flag
        // instead of mistaking an temporarily empty queue for completed work.
        if state.flush_in_progress {
            return;
        }
        let batch = std::mem::take(&mut state.queued);
        if batch.is_empty() {
            return;
        }
        state.flush_in_progress = true;
        state.in_flight = state.in_flight.saturating_add(batch.len());
        batch
    };
    let started = Instant::now();
    let mut iter = batch.into_iter();
    let mut processed = 0usize;
    while let Some(pending) = iter.next() {
        if processed >= max_receipts || started.elapsed() >= budget {
            let remaining = std::iter::once(pending).chain(iter).collect::<Vec<_>>();
            let mut state = pending_receipts().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            state.in_flight = state.in_flight.saturating_sub(remaining.len());
            state.queued.extend(remaining);
            break;
        }
        processed += 1;
        match pending.slot.persist_recoverable(pending.receipt.clone()) {
            Ok(_) => {
                let task_id = pending.receipt.task_id.clone();
                let mut state = pending_receipts().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                state.in_flight = state.in_flight.saturating_sub(1);
                drop(state);
                emit_task_progress(
                    app,
                    &task_id,
                    "receipt_recovered",
                    "The previously pending provenance receipt has now been stored.",
                    100,
                );
            }
            Err(error) => {
                let (slot, message) = *error;
                if let Some(attempts) = next_receipt_retry_attempt(pending.attempts) {
                    let failed = PendingReceipt { receipt: pending.receipt, slot, attempts };
                    let remaining = std::iter::once(failed).chain(iter).collect::<Vec<_>>();
                    let mut state = pending_receipts().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    state.in_flight = state.in_flight.saturating_sub(remaining.len());
                    state.queued.extend(remaining);
                    break;
                }

                // A poison receipt must not permanently block unrelated receipts.
                // Dropping the reservation releases its durable slot; the final warning
                // makes the provenance gap visible instead of retrying forever.
                let task_id = pending.receipt.task_id.clone();
                drop(slot);
                let mut state = pending_receipts().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                state.in_flight = state.in_flight.saturating_sub(1);
                drop(state);
                emit_task_progress(
                    app,
                    &task_id,
                    "receipt_warning",
                    &format!("The provenance receipt could not be stored after {MAX_RECEIPT_RETRY_ATTEMPTS} attempts and was discarded: {message}"),
                    100,
                );
            }
        }
    }
    let mut state = pending_receipts().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    state.flush_in_progress = false;
    drop(state);
    pending_receipts_changed().notify_all();
}

/// Wait for an active retry batch, then make one final bounded flush during shutdown.
fn flush_pending_receipts_on_exit(app: &AppHandle) {
    let deadline = Instant::now() + RECEIPT_RETRY_BUDGET;
    {
        let mut state = pending_receipts().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        while state.flush_in_progress {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return;
            }
            let (next, timeout) = pending_receipts_changed()
                .wait_timeout(state, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state = next;
            if timeout.timed_out() && state.flush_in_progress {
                return;
            }
        }
    }
    let remaining = deadline.saturating_duration_since(Instant::now());
    if !remaining.is_zero() {
        flush_pending_receipts_blocking(app, remaining, MAX_PENDING_RECEIPTS);
    }
}

/// Retries pending receipts off the async runtime with a short, bounded time budget.
async fn flush_pending_receipts(app: &AppHandle) {
    let app = app.clone();
    let _ = tokio::task::spawn_blocking(move || flush_pending_receipts_blocking(&app, RECEIPT_RETRY_BUDGET, MAX_PENDING_RECEIPTS)).await;
}

/// Persists a receipt or returns a warning while preserving the completed task result.
fn persist_receipt_or_warn(
    label: &str,
    slot: security_store::ReceiptSlotReservation,
    receipt: ExecutionReceipt,
) -> Option<String> {
    match slot.persist_recoverable(receipt.clone()) {
        Ok(_) => None,
        Err(recovery) => {
            let (slot, error) = *recovery;
            Some(if enqueue_pending_receipt(receipt, slot) {
                format!("{label} completed. Its provenance receipt could not be stored yet and has been queued for retry: {error}")
            } else {
                format!("{label} completed, but receipt storage and the bounded retry queue are unavailable. Preserve this result before closing UJANI: {error}")
            })
        },
    }
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
async fn await_task_with_limits<T, F>(
    token: &CancellationToken,
    budget: std::time::Duration,
    timeout_message: &str,
    future: F,
) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>>,
{
    tokio::select! {
        _ = token.cancelled() => Err("Task cancelled by user.".to_string()),
        outcome = tokio::time::timeout(budget, future) => {
            match outcome {
                Ok(value) => value,
                Err(_) => Err(timeout_message.to_string()),
            }
        }
    }
}

/// Registers and executes a managed task with cancellation, resource admission, and provenance handling.
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
    let registration = register_active_task(control, &task_id)?;
    run_managed_task_registered(app, task_id, label, receipt_context, network_activity, future, registration).await
}

/// Runs a task using an existing cancellation registration through admission, execution, and receipt handling.
async fn run_managed_task_registered<T, F>(app: AppHandle, task_id: String, label: &'static str, receipt_context: LocalReceiptContext, network_activity: NetworkActivity, future: F, registration: ActiveTaskRegistration<'_>) -> Result<T, String>
where
    T: Serialize,
    F: std::future::Future<Output = Result<T, String>>,
{
    if registration.task_id != task_id { return Err("Task registration does not match the requested task identifier.".into()); }
    ensure_task_not_cancelled(&registration.token)?;
    let token = registration.token.clone();
    emit_task_progress(&app, &task_id, "queued", &format!("{label}: waiting for local device capacity (up to four tasks may wait)."), 1);
    let _memory_reservation = resource_runtime::MemoryReservation::acquire_queued(&task_id, &token).await?;
    flush_pending_receipts(&app).await;
    let receipt_slot = security_store::reserve_execution_receipt_slot(&app, &task_id)?;
    if let Some(warning) = receipt_slot.archive_warning() {
        emit_task_progress(&app, &task_id, "receipt_archive_warning", warning, 3);
    }
    let started_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64;
    emit_task_progress(&app, &task_id, "starting", &format!("{label}: starting locally."), 5);
    let result = await_task_with_limits(
        &token,
        std::time::Duration::from_secs(240),
        &format!("{label} exceeded the 240-second overall time budget."),
        future,
    ).await;
    drop(registration);
    let mut receipt_warning = None;
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
        receipt_warning = persist_receipt_or_warn(label, receipt_slot, receipt);
    }
    match &result {
        Ok(_) => emit_task_progress(&app, &task_id, "completed", &format!("{label}: completed."), 100),
        Err(error) if error.contains("cancelled") => emit_task_progress(&app, &task_id, "cancelled", &format!("{label}: cancelled."), 0),
        Err(error) if error.contains("240-second") => emit_task_progress(&app, &task_id, "timed_out", error, 0),
        Err(error) => emit_task_progress(&app, &task_id, "failed", error, 0),
    }
    if let Some(message) = receipt_warning { emit_task_progress(&app, &task_id, "receipt_warning", &message, 100); }
    result
}

#[cfg(test)]
mod managed_task_liveness_tests {
    use super::await_task_with_limits;
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn managed_task_timeout_does_not_block_the_async_executor() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let timer_ran = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&timer_ran);
        let result = runtime.block_on(async {
            let timer = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(1)).await;
                observed.store(true, Ordering::SeqCst);
            });
            let result = await_task_with_limits(
                &CancellationToken::new(),
                Duration::from_millis(10),
                "test task timed out",
                std::future::pending::<Result<(), String>>(),
            ).await;
            timer.await.unwrap();
            result
        });
        assert_eq!(result, Err("test task timed out".to_string()));
        assert!(timer_ran.load(Ordering::SeqCst), "executor timer must run while task is pending");
    }

    #[test]
    fn managed_task_cancellation_interrupts_a_pending_operation() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let result = runtime.block_on(async {
            let token = CancellationToken::new();
            let cancel = token.clone();
            let cancel_task = tokio::spawn(async move {
                tokio::task::yield_now().await;
                cancel.cancel();
            });
            let result = await_task_with_limits(
                &token,
                Duration::from_secs(1),
                "test task timed out",
                std::future::pending::<Result<(), String>>(),
            ).await;
            cancel_task.await.unwrap();
            result
        });
        assert_eq!(result, Err("Task cancelled by user.".to_string()));
    }
}

#[cfg(test)]
mod local_receipt_context_tests {
    use super::local_receipt_context;

    #[test]
    fn active_task_guard_cleans_registry_on_scope_exit() {
        use super::{ActiveTaskGuard, TaskControlState};
        use tokio_util::sync::CancellationToken;

        let control = TaskControlState::default();
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
    resource_runtime::validate_task_id(&task_id)?;
    // Register before any runtime/network preflight so the UI's Cancel action can
    // interrupt slow local-service checks instead of appearing unresponsive.
    let token = CancellationToken::new();
    {
        let mut active = control.active.lock().map_err(|_| "Task registry unavailable.".to_string())?;
        if active.contains_key(&task_id) { return Err("Task identifier is already active.".into()); }
        active.insert(task_id.clone(), token.clone());
    }
    let _active_task_guard = ActiveTaskGuard { control: &control, task_id: task_id.clone() };
    emit_task_progress(&app, &task_id, "preflight", "Checking local device capability and installed model availability.", 2);

    // Keep the route-preview policy check, but make both preflight stages cancellable
    // and bounded. The engine repeats the critical runtime/model check immediately
    // before inference after queue admission.
    let capability_snapshot = await_task_with_limits(
        &token,
        std::time::Duration::from_secs(12),
        "Local capability preflight exceeded its 12-second time budget.",
        get_capability_snapshot(),
    ).await?;
    let local_plan = plan_local_task(
        "local.ai.generate",
        &mode,
        capability_snapshot.available_memory_mb,
        &capability_snapshot.capabilities,
    );
    if !local_plan.accepted {
        return Err(format!("UJANI local-first policy blocked this task: {}", local_plan.reason));
    }
    let installed_models = await_task_with_limits(
        &token,
        std::time::Duration::from_secs(8),
        "Checking installed local models exceeded its 8-second time budget.",
        ollama::list_models(),
    ).await?;
    if !installed_models.iter().any(|installed| installed.name == model) {
        return Err("The selected model is not installed in the local Ollama runtime. Choose an installed model or download it explicitly; UJANI will not silently send this task to the cloud.".into());
    }
    emit_task_progress(&app, &task_id, "queued", "Local chat: waiting for device capacity (up to four tasks may wait).", 1);
    let _memory_reservation = resource_runtime::MemoryReservation::acquire_queued(&task_id, &token).await?;
    flush_pending_receipts(&app).await;
    let receipt_slot = security_store::reserve_execution_receipt_slot(&app, &task_id)?;
    if let Some(warning) = receipt_slot.archive_warning() {
        emit_task_progress(&app, &task_id, "receipt_archive_warning", warning, 3);
    }
    let started_at_unix_ms = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis() as u64;
    let input_digest = sha256_hex(prompt.as_bytes());
    let receipt_model_id = model.clone();
    emit_task_progress(&app, &task_id, "starting", "Checking local policy and preparing inference.", 5);
    emit_task_progress(&app, &task_id, "inference", "Local model request sent; streaming the response from Ollama.", 25);
    let execution = LocalTaskEngine::execute_stream(prompt, mode, model, |chunk| {
        emit_chat_delta(&app, &task_id, chunk);
    });
    let result = tokio::select! {
        _ = token.cancelled() => Err("Task cancelled by user.".to_string()),
        outcome = tokio::time::timeout(std::time::Duration::from_secs(140), execution) => {
            match outcome {
                Ok(value) => value,
                Err(_) => Err("Local task exceeded the 140-second overall time budget. Try a smaller prompt or faster model.".into()),
            }
        }
    };
    let mut receipt_warning = None;
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
        receipt_warning = persist_receipt_or_warn("Local inference", receipt_slot, receipt);
    }
    match &result {
        Ok(_) => emit_task_progress(&app, &task_id, "completed", "Local task completed.", 100),
        Err(error) if error.contains("cancelled") => emit_task_progress(&app, &task_id, "cancelled", "Task cancelled.", 0),
        Err(error) if error.contains("140-second") => emit_task_progress(&app, &task_id, "timed_out", error, 0),
        Err(error) => emit_task_progress(&app, &task_id, "failed", error, 0),
    }
    if let Some(message) = receipt_warning { emit_task_progress(&app, &task_id, "receipt_warning", &message, 100); }
    result
}

/// Requests cancellation for an active task or a bounded PDF registration handoff.
fn request_task_cancellation(control: &TaskControlState, task_id: &str) -> Result<bool, String> {
    if !valid_command_identifier(task_id, 96) { return Err("Invalid task identifier.".into()); }
    resource_runtime::validate_task_id(task_id)?;
    let active = control.active.lock().map_err(|_| "Task registry unavailable.".to_string())?;
    if let Some(token) = active.get(task_id) { token.cancel(); return Ok(true); }
    if !task_id.starts_with("pdf-") { return Ok(false); }
    let now = Instant::now();
    let mut finished = control.finished_pdf_tasks.lock().map_err(|_| "Finished PDF registry unavailable.".to_string())?;
    finished.retain(|_, completed_at| now.duration_since(*completed_at) <= FINISHED_PDF_TASK_TTL);
    if finished.contains_key(task_id) { return Ok(false); }
    drop(finished);
    let mut pending = control.pending_cancellations.lock().map_err(|_| "Pending cancellation registry unavailable.".to_string())?;
    pending.retain(|_, requested_at| now.duration_since(*requested_at) <= PENDING_CANCELLATION_TTL);
    if !pending.contains_key(task_id) && pending.len() >= MAX_PENDING_CANCELLATIONS { return Ok(false); }
    pending.insert(task_id.to_string(), now); Ok(true)
}

/// Cancels a registered task and emits a metadata-only acknowledgement when accepted.
#[tauri::command]
fn cancel_active_task(app: AppHandle, control: State<'_, TaskControlState>, task_id: String) -> Result<bool, String> {
    let accepted = request_task_cancellation(&control, &task_id)?;
    if accepted {
        emit_task_progress(&app, &task_id, "cancel_requested", "Cancellation request accepted.", 0);
    }
    Ok(accepted)
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

const MAX_PDF_UPLOAD_BYTES: usize = 10 * 1024 * 1024;

/// Decode base64 IPC data with an encoded and decoded size bound.
fn decode_bounded_pdf_payload(data_base64: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let max_base64_bytes = max_bytes.div_ceil(3) * 4;
    if data_base64.len() > max_base64_bytes {
        return Err("PDF upload exceeds the 10 MiB limit.".into());
    }
    let data = STANDARD.decode(data_base64.as_bytes())
        .map_err(|_| "PDF upload could not be decoded. Please select the PDF again.".to_string())?;
    if data.is_empty() || data.len() > max_bytes {
        return Err("PDF files must be non-empty and no larger than 10 MiB.".into());
    }
    Ok(data)
}

/// Decodes a non-empty PDF upload after enforcing the encoded and decoded 10 MiB bounds.
fn decode_pdf_upload(data_base64: &str) -> Result<Vec<u8>, String> {
    decode_bounded_pdf_payload(data_base64, MAX_PDF_UPLOAD_BYTES)
}

const MAX_CONCURRENT_PDF_DECODERS: usize = 1;
static PDF_DECODE_SLOTS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();

/// Returns the process-wide semaphore that allows only one PDF decoder at a time.
fn pdf_decode_slots() -> Arc<tokio::sync::Semaphore> {
    Arc::clone(PDF_DECODE_SLOTS.get_or_init(|| Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_PDF_DECODERS))))
}

/// Runs bounded blocking work while retaining its admission permit until the worker exits.
fn spawn_bounded_pdf_worker<T, F, R>(slots: Arc<tokio::sync::Semaphore>, work: F, on_permit_released: R) -> Result<tokio::task::JoinHandle<T>, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
    R: FnOnce() + Send + 'static,
{
    let permit = slots.try_acquire_owned().map_err(|_| "Another PDF upload is still being prepared. Please wait before retrying.".to_string())?;
    Ok(tokio::task::spawn_blocking(move || {
        let result = work();
        drop(permit);
        on_permit_released();
        result
    }))
}

/// Decodes the bounded PDF payload off the async executor and reports decoder lifecycle stages.
async fn decode_pdf_upload_off_thread(app: AppHandle, task_id: String, data_base64: String) -> Result<Vec<u8>, String> {
    let release_app = app.clone();
    let release_task_id = task_id.clone();
    let worker = spawn_bounded_pdf_worker(
        pdf_decode_slots(),
        move || {
            emit_task_progress(&app, &task_id, "decode_started", "PDF upload decoding started in the bounded worker.", 8);
            let result = decode_pdf_upload(&data_base64);
            emit_task_progress(&app, &task_id, "decode_finished", "PDF upload decoder work has finished.", 10);
            result
        },
        move || emit_task_progress(&release_app, &release_task_id, "decode_permit_released", "PDF decoder capacity has been released.", 10),
    )?;
    worker.await.map_err(|error| format!("PDF decode worker failed: {error}"))?
}


#[cfg(test)]
mod pdf_cancellation_registration_tests {
    use super::{register_active_task, request_task_cancellation, TaskControlState, MAX_PENDING_CANCELLATIONS};

    #[test]
    fn registration_preserves_resource_queue_identifier_validation() {
        let control = TaskControlState::default();
        assert!(register_active_task(&control, "pdf-upload.valid").is_err());
        assert!(register_active_task(&control, "pdf-upload:valid").is_err());
        assert!(request_task_cancellation(&control, "pdf-upload.valid").is_err());
        assert!(request_task_cancellation(&control, "pdf-upload:valid").is_err());
        assert!(register_active_task(&control, "pdf-upload-valid").is_ok());
    }

    #[test]
    fn duplicate_active_pdf_registration_is_rejected_and_fresh_id_allows_retry() {
        let control = TaskControlState::default();
        let first = register_active_task(&control, "pdf-upload-duplicate").unwrap();
        assert!(register_active_task(&control, "pdf-upload-duplicate").is_err());
        drop(first);
        assert!(register_active_task(&control, "pdf-upload-duplicate").is_err());
        assert!(register_active_task(&control, "pdf-upload-fresh-retry").is_ok());
    }

    #[test]
    fn cancellation_during_native_pdf_preparation_reaches_registered_task() {
        let control = TaskControlState::default();
        let registration = register_active_task(&control, "pdf-upload-active").unwrap();
        assert!(request_task_cancellation(&control, "pdf-upload-active").unwrap());
        assert!(registration.token.is_cancelled());
    }

    #[test]
    fn cancellation_in_ipc_handoff_is_applied_on_registration() {
        let control = TaskControlState::default();
        assert!(request_task_cancellation(&control, "pdf-upload-pending").unwrap());
        let registration = register_active_task(&control, "pdf-upload-pending").unwrap();
        assert!(registration.token.is_cancelled());
        assert!(!control.pending_cancellations.lock().unwrap().contains_key("pdf-upload-pending"));
    }

    #[test]
    fn late_cancellation_is_rejected_after_pdf_registration_finishes() {
        let control = TaskControlState::default();
        {
            let registration = register_active_task(&control, "pdf-upload-finished").unwrap();
            assert!(!registration.token.is_cancelled());
        }
        assert!(!request_task_cancellation(&control, "pdf-upload-finished").unwrap());
        assert!(control.pending_cancellations.lock().unwrap().is_empty());
        // A fresh ID for a retry is not blocked by the prior completed attempt.
        assert!(request_task_cancellation(&control, "pdf-upload-retry-unique").unwrap());
        let retry = register_active_task(&control, "pdf-upload-retry-unique").unwrap();
        assert!(retry.token.is_cancelled());
    }

    #[test]
    fn concurrent_late_cancel_cannot_leave_stale_pending_registration() {
        use std::sync::Barrier;
        use std::thread;

        for index in 0..32 {
            let control = TaskControlState::default();
            let task_id = format!("pdf-cancel-race-{index}");
            let registration = register_active_task(&control, &task_id).unwrap();
            let barrier = std::sync::Arc::new(Barrier::new(2));
            let cancel_barrier = barrier.clone();

            thread::scope(|scope| {
                let cancel = scope.spawn(|| {
                    cancel_barrier.wait();
                    request_task_cancellation(&control, &task_id).unwrap()
                });
                barrier.wait();
                drop(registration);
                let _ = cancel.join().unwrap();
            });

            assert!(control.pending_cancellations.lock().unwrap().is_empty());
            assert!(control.finished_pdf_tasks.lock().unwrap().contains_key(&task_id));
            assert!(!request_task_cancellation(&control, &task_id).unwrap());
        }
    }

    #[test]
    fn pre_registration_cancellation_is_bounded_and_pdf_only() {
        let control = TaskControlState::default();
        assert!(!request_task_cancellation(&control, "ordinary-task").unwrap());
        for index in 0..MAX_PENDING_CANCELLATIONS {
            assert!(request_task_cancellation(&control, &format!("pdf-pending-{index}")).unwrap());
        }
        assert!(!request_task_cancellation(&control, "pdf-pending-overflow").unwrap());
        assert_eq!(control.pending_cancellations.lock().unwrap().len(), MAX_PENDING_CANCELLATIONS);
    }
}

#[cfg(test)]
mod pdf_upload_tests {
    use super::{decode_bounded_pdf_payload, decode_pdf_upload, MAX_PDF_UPLOAD_BYTES};
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    #[test]
    fn pdf_upload_decoder_accepts_exact_limit_and_rejects_decoded_overflow() {
        let exact = STANDARD.encode(b"12345678");
        assert_eq!(decode_bounded_pdf_payload(&exact, 8).unwrap(), b"12345678");
        // This payload is within the encoded-length ceiling but decodes to 9 bytes.
        let decoded_overflow = STANDARD.encode(b"123456789");
        assert!(decode_bounded_pdf_payload(&decoded_overflow, 8).is_err());
    }

    #[test]
    fn pdf_upload_decoder_rejects_empty_malformed_and_zero_limit_payloads() {
        assert!(decode_bounded_pdf_payload("", 8).is_err());
        assert!(decode_bounded_pdf_payload("not base64!", 8).is_err());
        assert!(decode_bounded_pdf_payload(&STANDARD.encode(b"x"), 0).is_err());
    }

    #[test]
    fn pdf_upload_decoder_rejects_encoded_payloads_over_the_predecode_bound() {
        let too_large = "A".repeat(16);
        assert!(decode_bounded_pdf_payload(&too_large, 8).is_err());
    }

    /// Dropping the async waiter must not release the single-decoder permit before its worker exits.
    #[tokio::test]
    async fn cancelled_pdf_worker_retains_decoder_permit_until_exit() {
        use super::spawn_bounded_pdf_worker;
        use std::sync::{mpsc, Arc};
        use std::time::Duration;

        let slots = Arc::new(tokio::sync::Semaphore::new(1));
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let (permit_released_tx, permit_released_rx) = mpsc::sync_channel(1);
        let callback_slots = slots.clone();
        let worker = spawn_bounded_pdf_worker(slots.clone(), move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        }, move || {
            assert!(callback_slots.try_acquire().is_ok(), "release callback must run after the permit is dropped");
            permit_released_tx.send(()).unwrap();
        }).unwrap();
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        drop(worker);
        assert!(spawn_bounded_pdf_worker(slots.clone(), || (), || {}).is_err());
        assert!(permit_released_rx.try_recv().is_err(), "permit release must not be reported while decode work is blocked");
        release_tx.send(()).unwrap();
        permit_released_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(spawn_bounded_pdf_worker(slots, || (), || {}).unwrap().await.is_ok());
    }

    #[test]
    fn maximum_upload_decode_runs_on_blocking_pool_not_async_executor_thread() {
        use std::thread;
        let caller = thread::current().id();
        let payload = STANDARD.encode(vec![0x5a; MAX_PDF_UPLOAD_BYTES]);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (decoded, worker) = runtime.block_on(async {
            tokio::task::spawn_blocking(move || {
                (decode_pdf_upload(&payload).unwrap(), thread::current().id())
            }).await.unwrap()
        });
        assert_eq!(decoded.len(), MAX_PDF_UPLOAD_BYTES);
        assert_ne!(caller, worker, "PDF decoding must never occupy the async executor thread");
    }
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
    data_base64: String,
    mode: String,
    model: String,
    translation_language: Option<String>,
) -> Result<task_engine::DocumentPipelineResult, String> {
    // Register before decoding so the user can cancel the native preparation phase.
    let registration = register_active_task(&control, &task_id)?;
    emit_task_progress(&app, &task_id, "preparing", "Preparing the selected PDF upload…", 5);
    ensure_task_not_cancelled(&registration.token)?;
    let data = await_task_with_limits(
        &registration.token, Duration::from_secs(30),
        "PDF upload decoding exceeded its 30-second time budget.",
        decode_pdf_upload_off_thread(app.clone(), task_id.clone(), data_base64),
    ).await?;

    let language = translation_language.as_deref().unwrap_or("");
    let receipt = local_receipt_context(
        "pdf.extract_and_summarize",
        &model,
        &[file_name.as_bytes(), data.as_slice(), mode.as_bytes(), language.as_bytes()],
    )?;
    run_managed_task_registered(app, task_id, "PDF extraction and summary", receipt, NetworkActivity::inferred(vec!["127.0.0.1:11434".into()]),
        LocalTaskEngine::process_pdf(file_name, data, mode, model, translation_language), registration).await
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

/// Reads aggregate OS security-provider status after an explicit user request.
#[tauri::command]
fn get_security_provider_status(
    provider_ids: Vec<String>,
) -> Result<Vec<security_providers::SecurityProviderStatus>, String> {
    if provider_ids.len() > 2 {
        return Err("Security provider selection exceeds the supported limit.".into());
    }
    security_providers::read_only_status(&provider_ids)
}

macro_rules! ujani_invoke_handler {
    ($($peer_command:path),* $(,)?) => {
        tauri::generate_handler![
            execute_cloud_ai_task, get_device_info, get_capability_snapshot, get_tool_catalogue,
            get_security_provider_status,
            plan_local_operation, plan_task_workflow, list_local_models, run_local_model_task,
            run_local_task, evaluate_task_route, get_ollama_status, pull_ollama_model,
            get_model_recommendation, execute_document_workflow, execute_local_ai_task,
            cancel_active_task, analyze_text_file, summarize_selected_text, process_selected_document,
            process_selected_pdf, translate_text_local, inspect_image_file,
            folder_import::process_selected_folder, local_search::find_local_files,
            search_cache::get_local_search_cache_settings, search_cache::list_recent_local_searches,
            search_cache::set_local_search_cache_settings, search_cache::clear_local_search_cache,
            everyday_tools::get_local_clock, everyday_tools::find_duplicate_files,
            file_insights::find_large_files, history::get_task_history_settings,
            history::set_task_history_enabled, history::save_task_history, history::list_task_history,
            history::delete_task_history, history::clear_task_history,
            security_store::verify_execution_receipt_ledger, security_store::list_execution_receipts,
            security_store::get_cloud_budget_status,
            security_store::set_cloud_budget_limit,
            $($peer_command),*
        ]
    };
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .manage(TaskControlState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build());

    #[cfg(feature = "peer-preview")]
    let builder = builder.invoke_handler(ujani_invoke_handler!(
        security_store::enroll_trusted_peer,
        security_store::revoke_trusted_peer,
        security_store::list_trusted_peers,
        security_store::verify_and_claim_peer_envelope,
        device_identity::get_or_create_device_identity,
        device_identity::rotate_device_identity,
        device_identity::sign_peer_task_envelope
    ));

    #[cfg(not(feature = "peer-preview"))]
    let builder = builder.invoke_handler(ujani_invoke_handler!());

    let app = builder.build(tauri::generate_context!())
        .expect("error while building UJANI Public AI");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            flush_pending_receipts_on_exit(app);
        }
    });
}

#[cfg(test)]
mod pending_receipt_retry_tests {
    use super::{next_receipt_retry_attempt, MAX_RECEIPT_RETRY_ATTEMPTS};

    #[test]
    fn receipt_retry_attempts_include_the_initial_failure_and_are_bounded() {
        let mut attempts = 1; // initial persist_receipt_or_warn attempt
        for expected in 2..MAX_RECEIPT_RETRY_ATTEMPTS {
            attempts = next_receipt_retry_attempt(attempts).expect("retry remains available");
            assert_eq!(attempts, expected);
        }
        assert_eq!(next_receipt_retry_attempt(attempts), None);
        assert_eq!(next_receipt_retry_attempt(u8::MAX), None);
    }
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
