//! Versioned, privacy-minimised inventory assembled from OS and loopback Ollama evidence.
use crate::ollama::{AcceleratorObservation, OllamaModel, OllamaModelMetadata};
use serde::Serialize;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct ModelRuntimeInventory {
    schema_version: u8,
    generated_at_unix_ms: u64,
    platform: InventoryPlatform,
    hardware: InventoryHardware,
    runtimes: Vec<InventoryRuntime>,
    models: Vec<InventoryModel>,
    privacy: InventoryPrivacy,
}

#[derive(Debug, Clone, Serialize)]
struct InventoryPlatform {
    os: String,
    os_version: Option<String>,
    architecture: String,
}

#[derive(Debug, Clone, Serialize)]
struct InventoryHardware {
    cpu_logical_cores: Option<u32>,
    memory_total_mb: Option<u64>,
    memory_available_mb: Option<u64>,
    storage_free_mb: Option<u64>,
    accelerators: Vec<InventoryAccelerator>,
}

#[derive(Debug, Clone, Serialize)]
struct InventoryAccelerator {
    kind: &'static str,
    vendor: Option<String>,
    device_family: Option<String>,
    memory_mb: Option<u64>,
    runtime_api: Option<String>,
    evidence: InventoryEvidence,
}

#[derive(Debug, Clone, Serialize)]
struct InventoryRuntime {
    runtime_id: &'static str,
    provider: &'static str,
    version: Option<String>,
    availability: &'static str,
    network_scope: &'static str,
    evidence: InventoryEvidence,
}

#[derive(Debug, Clone, Serialize)]
struct InventoryModel {
    model_ref: String,
    runtime_id: &'static str,
    display_name: String,
    family: Option<String>,
    version: Option<String>,
    availability: &'static str,
    size_bytes: Option<u64>,
    context_length_tokens: Option<u64>,
    capabilities: Vec<String>,
    evidence: InventoryEvidence,
}

#[derive(Debug, Clone, Serialize)]
struct InventoryEvidence {
    source: &'static str,
    confidence: &'static str,
    observed_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct InventoryPrivacy {
    contains_prompts: bool,
    contains_user_files: bool,
    contains_local_paths: bool,
    contains_credentials: bool,
}

impl InventoryEvidence {
    fn reported(source: &'static str, observed_at_unix_ms: u64) -> Self {
        Self {
            source,
            confidence: "reported",
            observed_at_unix_ms: Some(observed_at_unix_ms),
        }
    }

    fn unknown() -> Self {
        Self {
            source: "unknown",
            confidence: "unknown",
            observed_at_unix_ms: None,
        }
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(1)
        .max(1)
}

fn bounded_text(value: Option<String>, max_chars: usize) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.chars().take(max_chars).collect())
        }
    })
}

/// Build inventory v1 from already-bounded OS and local-runtime observations.
/// Unknown accelerator, storage, context and capability values remain unknown;
/// this function never probes another endpoint or reads user content.
pub fn build_inventory(
    models: &[OllamaModel],
    metadata: &HashMap<String, OllamaModelMetadata>,
    runtime_available: bool,
    runtime_version: Option<String>,
    os_name: String,
    os_version: Option<String>,
    architecture: String,
    cpu_logical_cores: Option<u32>,
    memory_total_mb: Option<u64>,
    memory_available_mb: Option<u64>,
    accelerator: Option<&AcceleratorObservation>,
) -> ModelRuntimeInventory {
    let observed_at = now_unix_ms();
    let gpu_activity_confirmed = accelerator
        .is_some_and(|observation| observation.gpu_vram_allocated_bytes > 0);

    let gpu = InventoryAccelerator {
        kind: "gpu",
        vendor: None,
        device_family: None,
        // /api/ps reports active model allocation, not total physical VRAM.
        // The v1 schema has no allocated-VRAM field, so do not mislabel it as capacity.
        memory_mb: None,
        runtime_api: accelerator.map(|_| "Ollama /api/ps".into()),
        evidence: if gpu_activity_confirmed {
            InventoryEvidence::reported("runtime_api", observed_at)
        } else {
            InventoryEvidence::unknown()
        },
    };
    let npu = InventoryAccelerator {
        kind: "npu",
        vendor: None,
        device_family: None,
        memory_mb: None,
        runtime_api: None,
        evidence: InventoryEvidence::unknown(),
    };

    let inventory_models = models
        .iter()
        .map(|model| {
            let model_metadata = metadata.get(&model.name);
            let capabilities = model_metadata
                .map(|item| item.capabilities.clone())
                .filter(|items| !items.is_empty())
                .unwrap_or_else(|| vec!["unknown".into()]);
            let evidence = if model_metadata.is_some() {
                InventoryEvidence::reported("runtime_metadata", observed_at)
            } else {
                // /api/tags still confirms the installed identifier and reported disk size.
                InventoryEvidence::reported("runtime_api", observed_at)
            };
            InventoryModel {
                model_ref: model.name.clone(),
                runtime_id: "ollama.local",
                display_name: model.name.clone(),
                family: bounded_text(
                    model.details.as_ref().and_then(|details| details.family.clone()),
                    96,
                ),
                version: None,
                availability: "installed",
                size_bytes: (model.size > 0).then_some(model.size),
                context_length_tokens: model_metadata
                    .and_then(|item| item.context_length_tokens),
                capabilities,
                evidence,
            }
        })
        .collect();

    ModelRuntimeInventory {
        schema_version: 1,
        generated_at_unix_ms: observed_at,
        platform: InventoryPlatform {
            os: bounded_text(Some(os_name), 32)
                .unwrap_or_else(|| std::env::consts::OS.to_string()),
            os_version: bounded_text(os_version, 64),
            architecture: bounded_text(Some(architecture), 32)
                .unwrap_or_else(|| std::env::consts::ARCH.to_string()),
        },
        hardware: InventoryHardware {
            cpu_logical_cores: cpu_logical_cores.filter(|cores| *cores > 0 && *cores <= 65_535),
            memory_total_mb: memory_total_mb.filter(|memory| *memory > 0),
            memory_available_mb,
            // No portable, unambiguous aggregate free-space value is collected yet.
            storage_free_mb: None,
            accelerators: vec![gpu, npu],
        },
        runtimes: vec![InventoryRuntime {
            runtime_id: "ollama.local",
            provider: "Ollama",
            version: bounded_text(runtime_version, 64),
            availability: if runtime_available { "available" } else { "unavailable" },
            network_scope: "loopback_only",
            evidence: InventoryEvidence::reported("runtime_api", observed_at),
        }],
        models: inventory_models,
        privacy: InventoryPrivacy {
            contains_prompts: false,
            contains_user_files: false,
            contains_local_paths: false,
            contains_credentials: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ollama::OllamaModelDetails;

    fn model(name: &str, size: u64) -> OllamaModel {
        OllamaModel {
            name: name.into(),
            size,
            details: Some(OllamaModelDetails {
                format: Some("gguf".into()),
                family: Some("llama".into()),
                parameter_size: Some("3B".into()),
                quantization_level: Some("Q4_K_M".into()),
            }),
        }
    }

    #[test]
    fn inventory_keeps_unknown_hardware_and_model_capabilities_explicit() {
        let inventory = build_inventory(
            &[model("llama3.2:latest", 1_000_000)],
            &HashMap::new(),
            false,
            None,
            "Linux".into(),
            None,
            "x86_64".into(),
            Some(8),
            Some(16_384),
            Some(8_192),
            None,
        );

        assert_eq!(inventory.schema_version, 1);
        assert_eq!(inventory.runtimes[0].availability, "unavailable");
        assert_eq!(inventory.runtimes[0].network_scope, "loopback_only");
        assert_eq!(inventory.models[0].capabilities, vec!["unknown"]);
        assert_eq!(inventory.models[0].context_length_tokens, None);
        assert_eq!(inventory.hardware.storage_free_mb, None);
        assert_eq!(inventory.hardware.accelerators[0].evidence.confidence, "unknown");
        assert_eq!(inventory.hardware.accelerators[1].kind, "npu");
        assert_eq!(inventory.hardware.accelerators[1].evidence.confidence, "unknown");
        assert!(!inventory.privacy.contains_prompts);
        assert!(!inventory.privacy.contains_user_files);
        assert!(!inventory.privacy.contains_local_paths);
        assert!(!inventory.privacy.contains_credentials);
    }

    #[test]
    fn inventory_preserves_only_runtime_reported_capabilities_and_context() {
        let model = model("qwen2.5:3b", 2_000_000);
        let metadata = HashMap::from([(
            model.name.clone(),
            OllamaModelMetadata {
                context_length_tokens: Some(32_768),
                capabilities: vec!["chat".into(), "vision".into(), "tool_use".into()],
            },
        )]);
        let inventory = build_inventory(
            &[model],
            &metadata,
            true,
            Some("0.5.12".into()),
            "Windows 11".into(),
            Some("10.0".into()),
            "x86_64".into(),
            Some(12),
            Some(32_768),
            Some(20_000),
            Some(&AcceleratorObservation {
                gpu_vram_allocated_bytes: 4 * 1024 * 1024 * 1024,
                running_models: 1,
                evidence: "Ollama reports active model VRAM allocation".into(),
            }),
        );

        assert_eq!(inventory.runtimes[0].version.as_deref(), Some("0.5.12"));
        assert_eq!(inventory.models[0].context_length_tokens, Some(32_768));
        assert_eq!(
            inventory.models[0].capabilities,
            vec!["chat", "vision", "tool_use"]
        );
        assert_eq!(inventory.models[0].evidence.source, "runtime_metadata");
        assert_eq!(inventory.hardware.accelerators[0].evidence.confidence, "reported");
        // Active allocation is not mislabeled as total VRAM capacity.
        assert_eq!(inventory.hardware.accelerators[0].memory_mb, None);
    }
}
