//! Read-only capability registry and conservative local task planning.
//! This module reports declared capabilities; it never executes a task or allocates resources.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Available,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRecord {
    pub id: String,
    pub category: String,
    pub state: CapabilityState,
    pub source: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationDescriptor {
    pub id: String,
    pub description: String,
    pub required_capabilities: Vec<String>,
    pub minimum_available_memory_mb: u64,
    /// Maximum wall-clock budget enforced by the native command or its managed worker.
    pub max_runtime_seconds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPlan {
    pub operation_id: String,
    pub accepted: bool,
    pub execution_location: String,
    pub required_capabilities: Vec<String>,
    pub missing_capabilities: Vec<String>,
    pub minimum_available_memory_mb: u64,
    pub available_memory_mb: u64,
    pub reason: String,
    pub execution_started: bool,
}

/// Trusted built-in operation catalogue. No arbitrary commands or model-authored tools.
pub fn registered_operations() -> Vec<OperationDescriptor> {
    vec![
        OperationDescriptor { id: "local.ai.generate".into(), description: "Generate a response with an installed local Ollama model".into(), required_capabilities: vec!["cpu".into(), "ollama.local".into()], minimum_available_memory_mb: 1024, max_runtime_seconds: 240 },
        OperationDescriptor { id: "document.summarize".into(), description: "Summarise user-selected text with a local model".into(), required_capabilities: vec!["cpu".into(), "ollama.local".into()], minimum_available_memory_mb: 1024, max_runtime_seconds: 240 },
        OperationDescriptor { id: "pdf.extract_and_summarize".into(), description: "Extract text from a selected PDF and summarise it locally".into(), required_capabilities: vec!["cpu".into(), "pdf.text_extraction".into(), "ollama.local".into(), "pdf.worker.resource_limits".into(), "pdf.worker.filesystem_isolation".into(), "pdf.worker.network_isolation".into()], minimum_available_memory_mb: 1024, max_runtime_seconds: 240 },
        OperationDescriptor { id: "translate.text".into(), description: "Translate supplied text with a local Ollama model".into(), required_capabilities: vec!["cpu".into(), "ollama.local".into()], minimum_available_memory_mb: 1024, max_runtime_seconds: 240 },
        OperationDescriptor { id: "local.word_count".into(), description: "Count words in user-supplied text".into(), required_capabilities: vec!["cpu".into()], minimum_available_memory_mb: 64, max_runtime_seconds: 60 },
        OperationDescriptor { id: "file.analyze_text".into(), description: "Analyse user-supplied text without a model".into(), required_capabilities: vec!["cpu".into()], minimum_available_memory_mb: 64, max_runtime_seconds: 60 },
        OperationDescriptor { id: "image.inspect".into(), description: "Inspect supported image headers and metadata".into(), required_capabilities: vec!["cpu".into(), "image.header_inspection".into()], minimum_available_memory_mb: 64, max_runtime_seconds: 60 },
        OperationDescriptor { id: "file.search.local".into(), description: "Search metadata and supported text in one user-selected local folder".into(), required_capabilities: vec!["cpu".into()], minimum_available_memory_mb: 128, max_runtime_seconds: 600 },
        OperationDescriptor { id: "file.search.pdf_text".into(), description: "Search text extracted from PDFs in one user-selected local folder".into(), required_capabilities: vec!["cpu".into(), "pdf.text_extraction".into(), "pdf.worker.resource_limits".into(), "pdf.worker.filesystem_isolation".into(), "pdf.worker.network_isolation".into()], minimum_available_memory_mb: 256, max_runtime_seconds: 600 },
        OperationDescriptor { id: "file.duplicates.preview".into(), description: "Preview duplicate files by bounded local content hashing in one user-selected folder".into(), required_capabilities: vec!["cpu".into()], minimum_available_memory_mb: 128, max_runtime_seconds: 60 },
        OperationDescriptor { id: "file.large_files.preview".into(), description: "Preview large-file metadata in one user-selected local folder".into(), required_capabilities: vec!["cpu".into()], minimum_available_memory_mb: 64, max_runtime_seconds: 60 },
    ]
}

/// Plan only. A successful plan is not an execution grant and does not reserve resources.
pub fn plan_local_task(
    operation_id: &str,
    mode: &str,
    available_memory_mb: u64,
    capabilities: &[CapabilityRecord],
) -> TaskPlan {
    let operation = registered_operations().into_iter().find(|op| op.id == operation_id);
    let Some(operation) = operation else {
        return TaskPlan {
            operation_id: operation_id.to_string(), accepted: false,
            execution_location: "unsupported".into(), required_capabilities: vec![],
            missing_capabilities: vec![], minimum_available_memory_mb: 0,
            available_memory_mb, reason: "Operation is not in UJANI's trusted local catalogue.".into(),
            execution_started: false,
        };
    };

    let base = |accepted: bool, location: &str, missing: Vec<String>, reason: String| TaskPlan {
        operation_id: operation.id.clone(), accepted, execution_location: location.into(),
        required_capabilities: operation.required_capabilities.clone(),
        missing_capabilities: missing, minimum_available_memory_mb: operation.minimum_available_memory_mb,
        available_memory_mb, reason, execution_started: false,
    };

    if mode != "fortress" && mode != "smart" {
        return base(false, "unsupported", vec![], "This planner currently schedules local operations only. Worker and Partner require a separately implemented secure gateway.".into());
    }

    let missing: Vec<String> = operation.required_capabilities.iter().filter(|required| {
        !capabilities.iter().any(|cap| cap.id == **required && cap.state == CapabilityState::Available)
    }).cloned().collect();

    if !missing.is_empty() {
        return base(false, "unavailable", missing, "One or more required capabilities are unavailable or unverified on this device.".into());
    }
    if available_memory_mb < operation.minimum_available_memory_mb {
        return base(false, "unavailable", vec![], "Available memory is below the operation's conservative minimum. Close other applications or retry later.".into());
    }

    // Keep the shared policy evaluator on the same path as the local capability
    // planner so a registered operation cannot bypass its policy-version,
    // operation-registration, memory and runtime-budget checks.
    let policy = crate::policy_gate::evaluate_local_registered_operation(
        &operation.id,
        available_memory_mb,
    );
    if policy.decision != crate::policy_gate::GateDecision::Allow || policy.requires_isolation {
        return base(
            false,
            "blocked",
            vec![],
            format!("Shared local execution policy blocked this operation: {}", policy.reason),
        );
    }

    base(true, "local", vec![], "Local plan is ready. The memory threshold is a coarse preflight only; it does not predict model-specific memory use or reserve CPU/RAM.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities(ollama: bool) -> Vec<CapabilityRecord> {
        vec![
            CapabilityRecord { id: "cpu".into(), category: "compute".into(), state: CapabilityState::Available, source: "test".into(), detail: "test CPU".into() },
            CapabilityRecord { id: "ollama.local".into(), category: "model_runtime".into(), state: if ollama { CapabilityState::Available } else { CapabilityState::Unavailable }, source: "test".into(), detail: "test Ollama".into() },
            CapabilityRecord { id: "pdf.text_extraction".into(), category: "document".into(), state: CapabilityState::Available, source: "test".into(), detail: "bundled parser".into() },
            CapabilityRecord { id: "pdf.worker.resource_limits".into(), category: "security".into(), state: CapabilityState::Available, source: "test".into(), detail: "verified worker limits".into() },
            CapabilityRecord { id: "pdf.worker.filesystem_isolation".into(), category: "security".into(), state: CapabilityState::Available, source: "test".into(), detail: "verified filesystem isolation".into() },
            CapabilityRecord { id: "pdf.worker.network_isolation".into(), category: "security".into(), state: CapabilityState::Available, source: "test".into(), detail: "verified network isolation".into() },
            CapabilityRecord { id: "image.header_inspection".into(), category: "image".into(), state: CapabilityState::Available, source: "test".into(), detail: "header parser".into() },
        ]
    }

    #[test]
    fn word_count_is_registered() {
        assert!(registered_operations().iter().any(|op| op.id == "local.word_count"));
    }

    #[test]
    fn only_registered_operations_can_be_planned() {
        let p = plan_local_task("shell.execute", "smart", 4096, &capabilities(true));
        assert!(!p.accepted);
        assert_eq!(p.execution_location, "unsupported");
    }

    #[test]
    fn local_model_task_requires_real_ollama_capability() {
        let p = plan_local_task("translate.text", "smart", 4096, &capabilities(false));
        assert!(!p.accepted);
        assert_eq!(p.missing_capabilities, vec!["ollama.local"]);
    }

    #[test]
    fn pdf_pipeline_requires_parser_and_local_model() {
        let p = plan_local_task("pdf.extract_and_summarize", "fortress", 4096, &capabilities(true));
        assert!(p.accepted);
        assert_eq!(p.execution_location, "local");
        assert!(!p.execution_started);
    }

    #[test]
    fn pdf_operations_fail_closed_without_verified_worker_security() {
        for capability in [
            "pdf.worker.resource_limits",
            "pdf.worker.filesystem_isolation",
            "pdf.worker.network_isolation",
        ] {
            let mut records = capabilities(true);
            records.iter_mut().find(|record| record.id == capability).unwrap().state = CapabilityState::Unknown;
            let summarize = plan_local_task("pdf.extract_and_summarize", "fortress", 4096, &records);
            assert!(!summarize.accepted, "PDF summary must fail closed without {capability}");
            assert!(summarize.missing_capabilities.contains(&capability.to_string()));

            let search = plan_local_task("file.search.pdf_text", "fortress", 4096, &records);
            assert!(!search.accepted, "PDF folder search must fail closed without {capability}");
            assert!(search.missing_capabilities.contains(&capability.to_string()));
        }
    }

    #[test]
    fn low_memory_blocks_plan_before_execution() {
        let p = plan_local_task("local.ai.generate", "smart", 128, &capabilities(true));
        assert!(!p.accepted);
        assert!(p.reason.contains("memory"));
    }

    #[test]
    fn cloud_modes_are_not_scheduled_by_local_planner() {
        let p = plan_local_task("local.ai.generate", "worker", 4096, &capabilities(true));
        assert!(!p.accepted);
        assert_eq!(p.execution_location, "unsupported");
    }

    #[test]
    fn deterministic_text_analysis_needs_no_model() {
        let p = plan_local_task("file.analyze_text", "fortress", 1024, &capabilities(false));
        assert!(p.accepted);
    }

    #[test]
    fn selected_folder_operations_are_registered_and_resource_bounded() {
        let cases = [
            ("file.search.local", 128, 600),
            ("file.search.pdf_text", 256, 600),
            ("file.duplicates.preview", 128, 60),
            ("file.large_files.preview", 64, 60),
        ];

        for (operation_id, minimum_memory_mb, max_runtime_seconds) in cases {
            let operation = registered_operations()
                .into_iter()
                .find(|candidate| candidate.id == operation_id)
                .expect("selected-folder operation is registered");
            assert_eq!(operation.minimum_available_memory_mb, minimum_memory_mb);
            assert_eq!(operation.max_runtime_seconds, max_runtime_seconds);
            assert!(operation.required_capabilities.contains(&"cpu".to_string()));

            let accepted = plan_local_task(operation_id, "fortress", 4096, &capabilities(false));
            assert!(accepted.accepted, "{operation_id}: {}", accepted.reason);
            let low_memory = plan_local_task(operation_id, "fortress", minimum_memory_mb - 1, &capabilities(false));
            assert!(!low_memory.accepted, "{operation_id} must fail closed below its memory floor");
            let remote = plan_local_task(operation_id, "worker", 4096, &capabilities(false));
            assert!(!remote.accepted, "{operation_id} must remain local-only");
        }
    }
}
