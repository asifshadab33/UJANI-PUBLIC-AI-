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

/// Inputs are observations already collected by the bounded OS/runtime adapters.
/// Keeping them in a named struct avoids positional argument mistakes as the
/// inventory contract grows.
pub struct ModelInventoryInput<'a> {
    pub models: &'a [OllamaModel],
    pub metadata: &'a HashMap<String, OllamaModelMetadata>,
    pub runtime_available: bool,
    pub runtime_version: Option<String>,
    pub os_name: String,
    pub os_version: Option<String>,
    pub architecture: String,
    pub cpu_logical_cores: Option<u32>,
    pub memory_total_mb: Option<u64>,
    pub memory_available_mb: Option<u64>,
    pub accelerator: Option<&'a AcceleratorObservation>,
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

/// Normalizes OS memory readings once for both model-fit advice and inventory output.
///
/// Non-positive totals are unknown. Available RAM above a known positive total is
/// treated as unknown rather than clamped or used to recommend a model.
pub(crate) fn normalize_memory_readings(
    total_memory_mb: Option<u64>,
    available_memory_mb: Option<u64>,
) -> (Option<u64>, Option<u64>) {
    let total_memory_mb = total_memory_mb.filter(|memory| *memory > 0);
    let available_memory_mb = match (total_memory_mb, available_memory_mb) {
        (Some(total), Some(available)) if available > total => None,
        (_, available) => available,
    };
    (total_memory_mb, available_memory_mb)
}

/// Build inventory v1 from already-bounded OS and local-runtime observations.
/// Unknown accelerator, storage, context and capability values remain unknown;
/// this function never probes another endpoint or reads user content.
pub fn build_inventory(input: ModelInventoryInput<'_>) -> ModelRuntimeInventory {
    let ModelInventoryInput {
        models,
        metadata,
        runtime_available,
        runtime_version,
        os_name,
        os_version,
        architecture,
        cpu_logical_cores,
        memory_total_mb,
        memory_available_mb,
        accelerator,
    } = input;
    let observed_at = now_unix_ms();
    let (memory_total_mb, memory_available_mb) =
        normalize_memory_readings(memory_total_mb, memory_available_mb);
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
            memory_total_mb,
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
            evidence: if runtime_available {
                InventoryEvidence::reported("runtime_api", observed_at)
            } else {
                InventoryEvidence::unknown()
            },
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
                families: None,
                parameter_size: Some("3B".into()),
                quantization_level: Some("Q4_K_M".into()),
            }),
        }
    }

    #[test]
    fn inventory_keeps_unknown_hardware_and_model_capabilities_explicit() {
        let models = [model("llama3.2:latest", 1_000_000)];
        let metadata = HashMap::new();
        let inventory = build_inventory(ModelInventoryInput {
            models: &models,
            metadata: &metadata,
            runtime_available: false,
            runtime_version: None,
            os_name: "Linux".into(),
            os_version: None,
            architecture: "x86_64".into(),
            cpu_logical_cores: Some(8),
            memory_total_mb: Some(16_384),
            memory_available_mb: Some(8_192),
            accelerator: None,
        });

        assert_eq!(inventory.schema_version, 1);
        assert_eq!(inventory.runtimes[0].availability, "unavailable");
        assert_eq!(inventory.runtimes[0].network_scope, "loopback_only");
        assert_eq!(inventory.runtimes[0].evidence.confidence, "unknown");
        assert_eq!(inventory.models[0].capabilities, vec!["unknown".to_string()]);
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
    fn inconsistent_available_memory_is_reported_as_unknown() {
        let models = [];
        let metadata = HashMap::new();
        let inventory = build_inventory(ModelInventoryInput {
            models: &models,
            metadata: &metadata,
            runtime_available: false,
            runtime_version: None,
            os_name: "Linux".into(),
            os_version: None,
            architecture: "x86_64".into(),
            cpu_logical_cores: Some(8),
            memory_total_mb: Some(8_192),
            memory_available_mb: Some(16_384),
            accelerator: None,
        });

        assert_eq!(inventory.hardware.memory_total_mb, Some(8_192));
        assert_eq!(inventory.hardware.memory_available_mb, None);
    }

    #[test]
    fn available_memory_is_preserved_when_total_memory_is_unknown() {
        let models = [];
        let metadata = HashMap::new();
        let inventory = build_inventory(ModelInventoryInput {
            models: &models,
            metadata: &metadata,
            runtime_available: false,
            runtime_version: None,
            os_name: "Linux".into(),
            os_version: None,
            architecture: "x86_64".into(),
            cpu_logical_cores: None,
            memory_total_mb: None,
            memory_available_mb: Some(2_048),
            accelerator: None,
        });

        assert_eq!(inventory.hardware.memory_total_mb, None);
        assert_eq!(inventory.hardware.memory_available_mb, Some(2_048));
    }

    #[test]
    fn inventory_serializes_the_v1_required_sections_and_privacy_flags() {
        let models = [];
        let metadata = HashMap::new();
        let inventory = build_inventory(ModelInventoryInput {
            models: &models,
            metadata: &metadata,
            runtime_available: false,
            runtime_version: None,
            os_name: "Linux".into(),
            os_version: None,
            architecture: "x86_64".into(),
            cpu_logical_cores: None,
            memory_total_mb: None,
            memory_available_mb: None,
            accelerator: None,
        });
        let value = serde_json::to_value(inventory).expect("inventory serializes");
        assert_eq!(value["schema_version"], 1);
        assert!(value["generated_at_unix_ms"].as_u64().unwrap_or(0) > 0);
        assert!(value["platform"]["os"].is_string());
        assert!(value["platform"]["architecture"].is_string());
        assert!(value["hardware"].get("cpu_logical_cores").is_some());
        assert!(value["hardware"].get("memory_total_mb").is_some());
        assert!(value["hardware"].get("memory_available_mb").is_some());
        assert!(value["hardware"]["accelerators"].is_array());
        assert!(value["runtimes"].is_array());
        assert!(value["models"].is_array());
        assert_eq!(value["privacy"]["contains_prompts"], false);
        assert_eq!(value["privacy"]["contains_user_files"], false);
        assert_eq!(value["privacy"]["contains_local_paths"], false);
        assert_eq!(value["privacy"]["contains_credentials"], false);
    }

    #[test]
    fn inventory_preserves_only_runtime_reported_capabilities_and_context() {
        let model = model("qwen2.5:3b", 2_000_000);
        let models = [model.clone()];
        let metadata = HashMap::from([(
            model.name.clone(),
            OllamaModelMetadata {
                context_length_tokens: Some(32_768),
                capabilities: vec!["chat".into(), "vision".into(), "tool_use".into()],
            },
        )]);
        let accelerator = AcceleratorObservation {
            gpu_vram_allocated_bytes: 4 * 1024 * 1024 * 1024,
            running_models: 1,
            evidence: "Ollama reports active model VRAM allocation".into(),
        };
        let inventory = build_inventory(ModelInventoryInput {
            models: &models,
            metadata: &metadata,
            runtime_available: true,
            runtime_version: Some("0.5.12".into()),
            os_name: "Windows 11".into(),
            os_version: Some("10.0".into()),
            architecture: "x86_64".into(),
            cpu_logical_cores: Some(12),
            memory_total_mb: Some(32_768),
            memory_available_mb: Some(20_000),
            accelerator: Some(&accelerator),
        });

        assert_eq!(inventory.runtimes[0].version.as_deref(), Some("0.5.12"));
        assert_eq!(inventory.models[0].context_length_tokens, Some(32_768));
        assert_eq!(
            inventory.models[0].capabilities,
            vec!["chat".to_string(), "vision".to_string(), "tool_use".to_string()]
        );
        assert_eq!(inventory.models[0].evidence.source, "runtime_metadata");
        assert_eq!(inventory.hardware.accelerators[0].evidence.confidence, "reported");
        // Active allocation is not mislabeled as total VRAM capacity.
        assert_eq!(inventory.hardware.accelerators[0].memory_mb, None);
    }
}
