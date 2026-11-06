//! Declarative, side-effect-free tool catalogue for UJANI Public AI.
//!
//! A manifest describes a capability; it does not grant permission or execute code.
//! Runtime adapters must independently enforce the declared limits and permissions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

pub const MAX_TOOL_INPUT_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_TOOL_OUTPUT_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_TOOL_MEMORY_MB: u32 = 64 * 1024;
pub const MAX_TOOL_RUNTIME_MS: u32 = 5 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    Document,
    Text,
    Image,
    Developer,
    Research,
    Productivity,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionKind {
    DeterministicLocal,
    LocalModel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAccess {
    None,
    Internet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermission {
    ReadSelectedFiles,
    WriteUserSelectedOutput,
    UseLocalModelRuntime,
    ExternalNetwork,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub max_input_bytes: u64,
    pub max_output_bytes: u64,
    pub max_memory_mb: u32,
    pub max_runtime_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolManifest {
    /// Stable operation identifier. Never interpreted as a command or file path.
    pub id: String,
    /// Semver-like manifest version, e.g. "1.0.0".
    pub version: String,
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub execution: ExecutionKind,
    pub network: NetworkAccess,
    pub permissions: Vec<ToolPermission>,
    pub budget: ResourceBudget,
    /// False means the current build exposes metadata only, not an executable adapter.
    pub adapter_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    InvalidId(String),
    InvalidVersion(String),
    InvalidText(String),
    InvalidBudget(String),
    MissingNetworkPermission(String),
    MissingRuntimePermission(String),
    DuplicateId(String),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(id) => write!(f, "invalid tool identifier: {id}"),
            Self::InvalidVersion(id) => write!(f, "invalid manifest version for tool: {id}"),
            Self::InvalidText(id) => write!(f, "invalid name or description for tool: {id}"),
            Self::InvalidBudget(id) => write!(f, "invalid or excessive resource budget for tool: {id}"),
            Self::MissingNetworkPermission(id) => write!(f, "network tool lacks explicit network permission: {id}"),
            Self::MissingRuntimePermission(id) => write!(f, "local-model tool lacks runtime permission: {id}"),
            Self::DuplicateId(id) => write!(f, "duplicate tool identifier: {id}"),
        }
    }
}

impl std::error::Error for RegistryError {}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'_' || b == b'-')
        && !value.contains("..")
}

fn valid_version(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        && value.len() <= 24
}

impl ToolManifest {
    /// Validate metadata before it is displayed or considered by a trusted adapter.
    /// This is not a runtime sandbox or a permission grant.
    pub fn validate(&self) -> Result<(), RegistryError> {
        if !valid_id(&self.id) {
            return Err(RegistryError::InvalidId(self.id.clone()));
        }
        if !valid_version(&self.version) {
            return Err(RegistryError::InvalidVersion(self.id.clone()));
        }
        if self.name.trim().is_empty()
            || self.name.len() > 80
            || self.description.trim().is_empty()
            || self.description.len() > 512
        {
            return Err(RegistryError::InvalidText(self.id.clone()));
        }
        let b = &self.budget;
        if b.max_input_bytes == 0
            || b.max_input_bytes > MAX_TOOL_INPUT_BYTES
            || b.max_output_bytes == 0
            || b.max_output_bytes > MAX_TOOL_OUTPUT_BYTES
            || b.max_memory_mb == 0
            || b.max_memory_mb > MAX_TOOL_MEMORY_MB
            || b.max_runtime_ms == 0
            || b.max_runtime_ms > MAX_TOOL_RUNTIME_MS
        {
            return Err(RegistryError::InvalidBudget(self.id.clone()));
        }
        let permissions: BTreeSet<ToolPermission> = self.permissions.iter().copied().collect();
        if self.network == NetworkAccess::Internet
            && !permissions.contains(&ToolPermission::ExternalNetwork)
        {
            return Err(RegistryError::MissingNetworkPermission(self.id.clone()));
        }
        if self.execution == ExecutionKind::LocalModel
            && !permissions.contains(&ToolPermission::UseLocalModelRuntime)
        {
            return Err(RegistryError::MissingRuntimePermission(self.id.clone()));
        }
        Ok(())
    }
}

/// Curated metadata for operations that already exist in the trusted core catalogue.
/// Adapter availability is descriptive only; execution still goes through native policy.
pub fn builtin_tool_manifests() -> Vec<ToolManifest> {
    let deterministic_budget = ResourceBudget {
        max_input_bytes: 1 * 1024 * 1024,
        max_output_bytes: 1 * 1024 * 1024,
        max_memory_mb: 256,
        max_runtime_ms: 15_000,
    };
    let model_budget = ResourceBudget {
        max_input_bytes: 256 * 1024,
        max_output_bytes: 64 * 1024,
        max_memory_mb: 8 * 1024,
        max_runtime_ms: 140_000,
    };
    vec![
        ToolManifest {
            id: "file.analyze_text".into(), version: "1.0.0".into(),
            name: "Analyze text".into(), description: "Count and inspect user-supplied text locally.".into(),
            category: ToolCategory::Text, execution: ExecutionKind::DeterministicLocal,
            network: NetworkAccess::None, permissions: vec![], budget: deterministic_budget.clone(),
            adapter_available: true,
        },
        ToolManifest {
            id: "pdf.extract_and_summarize".into(), version: "1.0.0".into(),
            name: "Extract and summarize PDF".into(), description: "Extract text from a user-selected PDF and summarize it with a local model.".into(),
            category: ToolCategory::Document, execution: ExecutionKind::LocalModel,
            network: NetworkAccess::None,
            permissions: vec![ToolPermission::ReadSelectedFiles, ToolPermission::UseLocalModelRuntime],
            budget: ResourceBudget { max_input_bytes: 10 * 1024 * 1024, ..model_budget.clone() },
            adapter_available: true,
        },
        ToolManifest {
            id: "document.summarize".into(), version: "1.0.0".into(),
            name: "Summarize document".into(), description: "Summarize user-selected text with an installed local model.".into(),
            category: ToolCategory::Document, execution: ExecutionKind::LocalModel,
            network: NetworkAccess::None, permissions: vec![ToolPermission::UseLocalModelRuntime],
            budget: model_budget.clone(), adapter_available: true,
        },
        ToolManifest {
            id: "image.inspect".into(), version: "1.0.0".into(),
            name: "Inspect image metadata".into(), description: "Inspect supported image headers without decoding image pixels.".into(),
            category: ToolCategory::Image, execution: ExecutionKind::DeterministicLocal,
            network: NetworkAccess::None, permissions: vec![ToolPermission::ReadSelectedFiles],
            budget: deterministic_budget.clone(), adapter_available: true,
        },
        ToolManifest {
            id: "local.ai.generate".into(), version: "1.0.0".into(),
            name: "Local AI chat".into(), description: "Generate a response using an installed local model runtime.".into(),
            category: ToolCategory::Productivity, execution: ExecutionKind::LocalModel,
            network: NetworkAccess::None, permissions: vec![ToolPermission::UseLocalModelRuntime],
            budget: model_budget.clone(), adapter_available: true,
        },
        ToolManifest {
            id: "translate.text".into(), version: "1.0.0".into(),
            name: "Translate text".into(), description: "Translate user-provided text with an installed local model.".into(),
            category: ToolCategory::Text, execution: ExecutionKind::LocalModel,
            network: NetworkAccess::None, permissions: vec![ToolPermission::UseLocalModelRuntime],
            budget: model_budget, adapter_available: true,
        },
    ]
}

/// Validate every manifest and reject duplicate identifiers deterministically.
pub fn validate_tool_registry(manifests: &[ToolManifest]) -> Result<(), RegistryError> {
    let mut ids = BTreeSet::new();
    for manifest in manifests {
        manifest.validate()?;
        if !ids.insert(manifest.id.as_str()) {
            return Err(RegistryError::DuplicateId(manifest.id.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_builtin_tool_is_in_the_canonical_operation_registry() {
        let operations = crate::capability::registered_operations();
        for tool in builtin_tool_manifests() {
            assert!(operations.iter().any(|operation| operation.id == tool.id), "unregistered tool: {}", tool.id);
        }
    }

    use super::*;

    #[test]
    fn builtin_registry_is_valid_and_has_unique_ids() {
        let manifests = builtin_tool_manifests();
        assert!(manifests.len() >= 5);
        assert!(manifests.iter().all(|manifest| manifest.adapter_available));
        assert_eq!(validate_tool_registry(&manifests), Ok(()));
    }

    #[test]
    fn rejects_command_like_or_path_like_identifiers() {
        let mut manifest = builtin_tool_manifests().remove(0);
        manifest.id = "../shell".into();
        assert!(matches!(manifest.validate(), Err(RegistryError::InvalidId(_))));
    }

    #[test]
    fn rejects_unbounded_resource_budgets() {
        let mut manifest = builtin_tool_manifests().remove(0);
        manifest.budget.max_input_bytes = MAX_TOOL_INPUT_BYTES + 1;
        assert!(matches!(manifest.validate(), Err(RegistryError::InvalidBudget(_))));
    }

    #[test]
    fn internet_access_requires_explicit_permission() {
        let mut manifest = builtin_tool_manifests().remove(0);
        manifest.network = NetworkAccess::Internet;
        assert!(matches!(manifest.validate(), Err(RegistryError::MissingNetworkPermission(_))));
    }

    #[test]
    fn local_model_requires_runtime_permission() {
        let mut manifest = builtin_tool_manifests().remove(0);
        manifest.execution = ExecutionKind::LocalModel;
        assert!(matches!(manifest.validate(), Err(RegistryError::MissingRuntimePermission(_))));
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let manifests = builtin_tool_manifests();
        let mut duplicated = manifests.clone();
        duplicated.push(manifests[0].clone());
        assert!(matches!(validate_tool_registry(&duplicated), Err(RegistryError::DuplicateId(_))));
    }

    #[test]
    fn manifest_is_serializable_for_versioned_ui_contracts() {
        let manifest = builtin_tool_manifests().remove(0);
        let json = serde_json::to_string(&manifest).expect("manifest should serialize");
        assert!(json.contains("file.analyze_text"));
        assert!(json.contains("deterministic_local"));
    }
}
