//! Local-first model/runtime inventory v2 producer.
//!
//! This is a read-only descriptor assembled only from bounded OS and loopback
//! Ollama observations. It never sends inventory to Enterprise, installs models,
//! reads prompts/files/paths, or grants model/workload execution permission.
use crate::model_inventory::{normalize_memory_readings, ModelInventoryInput};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct ModelRuntimeInventoryV2 {
    schema_version: u8,
    generated_at_unix_ms: u64,
    collection_context: CollectionContext,
    platform: Platform,
    hardware: Hardware,
    runtimes: Vec<Runtime>,
    models: Vec<Model>,
    privacy: Privacy,
}

#[derive(Debug, Clone, Serialize)]
struct CollectionContext {
    scope: &'static str,
    owner_consent: &'static str,
    organisation_authorisation: &'static str,
    policy_version: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct Evidence {
    source: &'static str,
    confidence: &'static str,
    observed_at_unix_ms: Option<u64>,
}
impl Evidence {
    fn observed(source: &'static str, at: u64) -> Self {
        Self { source, confidence: "reported", observed_at_unix_ms: Some(at) }
    }
    fn verified_os(at: u64) -> Self {
        Self { source: "os_api", confidence: "verified", observed_at_unix_ms: Some(at) }
    }
    fn unknown() -> Self {
        Self { source: "unknown", confidence: "unknown", observed_at_unix_ms: None }
    }
    fn estimate(at: u64) -> Self {
        Self { source: "estimate", confidence: "estimated", observed_at_unix_ms: Some(at) }
    }
}

#[derive(Debug, Clone, Serialize)]
struct Platform {
    os: &'static str,
    os_version: Option<String>,
    architecture: String,
    evidence: Evidence,
}
#[derive(Debug, Clone, Serialize)]
struct Hardware {
    cpu_logical_cores: Option<u32>,
    memory_total_mb: Option<u64>,
    memory_available_mb: Option<u64>,
    storage_free_mb: Option<u64>,
    accelerators: Vec<Accelerator>,
    evidence: Evidence,
}
#[derive(Debug, Clone, Serialize)]
struct Accelerator {
    kind: &'static str,
    vendor: Option<String>,
    device_family: Option<String>,
    memory_mb: Option<u64>,
    runtime_api: Option<String>,
    evidence: Evidence,
}
#[derive(Debug, Clone, Serialize)]
struct Runtime {
    runtime_id: &'static str,
    provider: &'static str,
    version: Option<String>,
    availability: &'static str,
    network_scope: &'static str,
    evidence: Evidence,
}
#[derive(Debug, Clone, Serialize)]
struct Capability {
    name: &'static str,
    status: &'static str,
    evidence: Evidence,
}
#[derive(Debug, Clone, Serialize)]
struct Context {
    max_tokens: Option<u64>,
    evidence: Evidence,
}
#[derive(Debug, Clone, Serialize)]
struct BooleanCapability {
    supported: Option<bool>,
    evidence: Evidence,
}
#[derive(Debug, Clone, Serialize)]
struct VisionCapability {
    supported: Option<bool>,
    max_image_bytes: Option<u64>,
    supported_mime_types: Vec<String>,
    evidence: Evidence,
}
#[derive(Debug, Clone, Serialize)]
struct EmbeddingsCapability {
    supported: Option<bool>,
    dimensions: Option<u64>,
    evidence: Evidence,
}
#[derive(Debug, Clone, Serialize)]
struct ResourceEstimates {
    minimum_memory_mb: Option<u64>,
    minimum_accelerator_memory_mb: Option<u64>,
    minimum_storage_bytes: Option<u64>,
    recommended_cpu_threads: Option<u32>,
    evidence: Evidence,
}
#[derive(Debug, Clone, Serialize)]
struct Compatibility {
    status: &'static str,
    supported_os: Vec<&'static str>,
    supported_architectures: Vec<String>,
    required_accelerator_kinds: Vec<&'static str>,
    required_accelerator_apis: Vec<String>,
    evidence: Evidence,
}
#[derive(Debug, Clone, Serialize)]
struct Model {
    model_ref: String,
    runtime_id: &'static str,
    display_name: String,
    family: Option<String>,
    version: Option<String>,
    availability: &'static str,
    size_bytes: Option<u64>,
    capabilities: Vec<Capability>,
    context: Context,
    tool_use: BooleanCapability,
    vision: VisionCapability,
    embeddings: EmbeddingsCapability,
    resource_estimates: ResourceEstimates,
    compatibility: Compatibility,
    evidence: Evidence,
}
#[derive(Debug, Clone, Serialize)]
struct Privacy {
    contains_prompts: bool,
    contains_user_files: bool,
    contains_local_paths: bool,
    contains_credentials: bool,
}

fn now_unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(1).max(1)
}
fn canonical_os(value: &str) -> &'static str {
    let value = value.to_ascii_lowercase();
    if value.contains("windows") { "windows" }
    else if value.contains("linux") { "linux" }
    else if value.contains("mac") || value.contains("darwin") { "macos" }
    else { "other" }
}
fn clean_text(value: Option<String>, max_chars: usize) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        if value.is_empty() || value.chars().any(char::is_control) { None }
        else { Some(value.chars().take(max_chars).collect()) }
    })
}
fn capability_is(reported: &[String], name: &str) -> bool {
    reported.iter().any(|item| item == name)
}
fn capabilities(reported: &[String], evidence: &Evidence) -> Vec<Capability> {
    let mut values = Vec::new();
    for (name, aliases) in [
        ("chat", &["chat", "completion"][..]),
        ("vision", &["vision"][..]),
        ("tool_use", &["tool_use", "tools"][..]),
        ("embeddings", &["embeddings", "embedding"][..]),
    ] {
        if aliases.iter().any(|alias| capability_is(reported, alias)) {
            values.push(Capability { name, status: "supported", evidence: evidence.clone() });
        }
    }
    values
}

/// Build v2 from the same already-collected observations as v1. Missing facts
/// remain unknown; capability names, hardware and compatibility are never guessed.
pub fn build_inventory_v2(input: ModelInventoryInput<'_>) -> ModelRuntimeInventoryV2 {
    let ModelInventoryInput {
        models, metadata, runtime_available, runtime_version, os_name, os_version,
        architecture, cpu_logical_cores, memory_total_mb, memory_available_mb, accelerator: _,
    } = input;
    let generated_at = now_unix_ms();
    let (memory_total_mb, memory_available_mb) =
        normalize_memory_readings(memory_total_mb, memory_available_mb);
    let platform_evidence = Evidence::verified_os(generated_at);
    let hardware_has_measurement = cpu_logical_cores.is_some()
        || memory_total_mb.is_some() || memory_available_mb.is_some();
    let hardware_evidence = if hardware_has_measurement {
        Evidence::verified_os(generated_at)
    } else { Evidence::unknown() };
    let runtime_evidence = if runtime_available {
        Evidence::observed("runtime_api", generated_at)
    } else { Evidence::unknown() };

    let inventory_models = models.iter().map(|model| {
        let details = metadata.get(&model.name);
        let reported_capabilities = details.map(|item| item.capabilities.as_slice()).unwrap_or(&[]);
        let model_evidence = if details.is_some() {
            Evidence::observed("runtime_metadata", generated_at)
        } else { Evidence::observed("runtime_api", generated_at) };
        let context_tokens = details.and_then(|item| item.context_length_tokens);
        let memory_estimate = ujani_core::model_advisor::estimate_model_working_memory_mb(model.size)
            .filter(|value| *value > 0 && *value <= 16_777_216);
        let has_tool_use = capability_is(reported_capabilities, "tool_use");
        let has_vision = capability_is(reported_capabilities, "vision");
        let has_embeddings = capability_is(reported_capabilities, "embeddings");
        let context_evidence = if context_tokens.is_some() { model_evidence.clone() } else { Evidence::unknown() };
        let resource_evidence = if memory_estimate.is_some() { Evidence::estimate(generated_at) } else { Evidence::unknown() };
        Model {
            model_ref: model.name.clone(),
            runtime_id: "ollama.local",
            display_name: model.name.clone(),
            family: clean_text(model.details.as_ref().and_then(|item| item.family.clone()), 96),
            version: model.digest.as_deref().and_then(|digest| {
                let hex = digest.strip_prefix("sha256:").unwrap_or(digest);
                if hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    Some(hex.to_ascii_lowercase())
                } else { None }
            }),
            availability: "installed",
            size_bytes: (model.size > 0).then_some(model.size),
            capabilities: capabilities(reported_capabilities, &model_evidence),
            context: Context { max_tokens: context_tokens, evidence: context_evidence },
            tool_use: BooleanCapability {
                supported: if has_tool_use { Some(true) } else { None },
                evidence: if has_tool_use { model_evidence.clone() } else { Evidence::unknown() },
            },
            vision: VisionCapability {
                supported: if has_vision { Some(true) } else { None },
                max_image_bytes: None,
                supported_mime_types: Vec::new(),
                evidence: if has_vision { model_evidence.clone() } else { Evidence::unknown() },
            },
            embeddings: EmbeddingsCapability {
                supported: if has_embeddings { Some(true) } else { None },
                dimensions: None,
                evidence: if has_embeddings { model_evidence.clone() } else { Evidence::unknown() },
            },
            resource_estimates: ResourceEstimates {
                minimum_memory_mb: memory_estimate,
                minimum_accelerator_memory_mb: None,
                minimum_storage_bytes: None,
                recommended_cpu_threads: None,
                evidence: resource_evidence,
            },
            compatibility: Compatibility {
                status: "unknown",
                supported_os: Vec::new(),
                supported_architectures: Vec::new(),
                required_accelerator_kinds: Vec::new(),
                required_accelerator_apis: Vec::new(),
                evidence: Evidence::unknown(),
            },
            evidence: model_evidence,
        }
    }).collect();

    ModelRuntimeInventoryV2 {
        schema_version: 2,
        generated_at_unix_ms: generated_at,
        collection_context: CollectionContext {
            scope: "personal_local_only",
            owner_consent: "not_required_local_only",
            organisation_authorisation: "not_applicable",
            policy_version: "public-local-v2",
        },
        platform: Platform {
            os: canonical_os(&os_name),
            os_version: clean_text(os_version, 64),
            architecture: {
                let value = architecture.trim();
                if value.is_empty() || value.len() > 32 || value.chars().any(char::is_control) {
                    std::env::consts::ARCH.to_string()
                } else { value.to_string() }
            },
            evidence: platform_evidence,
        },
        hardware: Hardware {
            cpu_logical_cores: cpu_logical_cores.filter(|value| *value > 0 && *value <= 65_535),
            memory_total_mb,
            memory_available_mb,
            storage_free_mb: None,
            accelerators: Vec::new(),
            evidence: hardware_evidence,
        },
        runtimes: vec![Runtime {
            runtime_id: "ollama.local",
            provider: "Ollama",
            version: clean_text(runtime_version, 64),
            availability: if runtime_available { "available" } else { "unavailable" },
            network_scope: "loopback_only",
            evidence: runtime_evidence,
        }],
        models: inventory_models,
        privacy: Privacy {
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
    use crate::ollama::{OllamaModel, OllamaModelDetails, OllamaModelMetadata};
    use std::collections::HashMap;

    fn sample_model() -> OllamaModel {
        OllamaModel {
            name: "llama3.2:latest".into(),
            size: 1_000_000_000,
            digest: None,
            details: Some(OllamaModelDetails {
                format: Some("gguf".into()), family: Some("llama".into()),
                families: None, parameter_size: Some("3B".into()),
                quantization_level: Some("Q4_K_M".into()),
            }),
        }
    }
    #[test]
    fn emits_personal_local_v2_contract_without_enterprise_identity() {
        let models = [sample_model()];
        let metadata = HashMap::new();
        let value = serde_json::to_value(build_inventory_v2(ModelInventoryInput {
            models: &models, metadata: &metadata, runtime_available: true,
            runtime_version: Some("0.5.0".into()), os_name: "Linux".into(),
            os_version: Some("6.8".into()), architecture: "x86_64".into(),
            cpu_logical_cores: Some(8), memory_total_mb: Some(16_384),
            memory_available_mb: Some(8_192), accelerator: None,
        })).unwrap();
        assert_eq!(value["schema_version"], 2);
        assert_eq!(value["collection_context"]["scope"], "personal_local_only");
        assert_eq!(value["collection_context"]["owner_consent"], "not_required_local_only");
        assert!(value.get("device_ref").is_none());
        assert_eq!(value["platform"]["os"], "linux");
        assert_eq!(value["platform"]["evidence"]["confidence"], "verified");
        assert_eq!(value["hardware"]["evidence"]["confidence"], "verified");
        assert_eq!(value["models"][0]["availability"], "installed");
        assert_eq!(value["models"][0]["compatibility"]["status"], "unknown");
        assert_eq!(value["models"][0]["capabilities"].as_array().unwrap().len(), 0);
        assert_eq!(value["models"][0]["resource_estimates"]["evidence"]["confidence"], "estimated");
        assert_eq!(value["privacy"]["contains_prompts"], false);
        assert_eq!(value["privacy"]["contains_user_files"], false);
        assert_eq!(value["privacy"]["contains_local_paths"], false);
        assert_eq!(value["privacy"]["contains_credentials"], false);
    }

    #[test]
    fn only_runtime_reported_capabilities_are_marked_supported() {
        let models = [sample_model()];
        let metadata = HashMap::from([("llama3.2:latest".to_string(), OllamaModelMetadata {
            context_length_tokens: Some(8192),
            capabilities: vec!["chat".into(), "vision".into()],
        })]);
        let value = serde_json::to_value(build_inventory_v2(ModelInventoryInput {
            models: &models, metadata: &metadata, runtime_available: true,
            runtime_version: None, os_name: "Darwin".into(), os_version: None,
            architecture: "arm64".into(), cpu_logical_cores: None,
            memory_total_mb: None, memory_available_mb: None, accelerator: None,
        })).unwrap();
        assert_eq!(value["models"][0]["capabilities"][0]["name"], "chat");
        assert_eq!(value["models"][0]["capabilities"][1]["name"], "vision");
        assert_eq!(value["models"][0]["vision"]["supported"], true);
        assert_eq!(value["models"][0]["context"]["max_tokens"], 8192);
        assert_eq!(value["models"][0]["compatibility"]["status"], "unknown");
        assert_eq!(value["models"][0]["embeddings"]["supported"], serde_json::Value::Null);
    }

    #[test]
    fn absent_hardware_and_runtime_facts_remain_unknown() {
        let models: [OllamaModel; 0] = [];
        let metadata = HashMap::new();
        let value = serde_json::to_value(build_inventory_v2(ModelInventoryInput {
            models: &models, metadata: &metadata, runtime_available: false,
            runtime_version: None, os_name: "unknown".into(), os_version: None,
            architecture: "unknown".into(), cpu_logical_cores: None,
            memory_total_mb: None, memory_available_mb: None, accelerator: None,
        })).unwrap();
        assert_eq!(value["hardware"]["evidence"]["confidence"], "unknown");
        assert_eq!(value["runtimes"][0]["evidence"]["confidence"], "unknown");
        assert_eq!(value["models"].as_array().unwrap().len(), 0);
    }
}
