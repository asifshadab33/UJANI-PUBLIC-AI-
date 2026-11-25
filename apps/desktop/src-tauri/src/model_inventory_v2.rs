//! Local-first model/runtime inventory v2 producer.
//!
//! This is a read-only descriptor assembled only from bounded OS and loopback
//! Ollama observations. It never sends inventory to Enterprise, installs models,
//! reads prompts/files/paths, or grants model/workload execution permission.
use crate::model_inventory::{normalize_memory_readings, ModelInventoryInput};
use serde::Serialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_SCHEMA_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

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

fn unix_ms_from_duration(duration: Duration) -> Option<u64> {
    u64::try_from(duration.as_millis())
        .ok()
        .filter(|value| *value > 0 && *value <= MAX_SCHEMA_SAFE_INTEGER)
}

fn unix_ms_at(now: SystemTime) -> Option<u64> {
    unix_ms_from_duration(now.duration_since(UNIX_EPOCH).ok()?)
}

fn now_unix_ms() -> Option<u64> {
    unix_ms_at(SystemTime::now())
}
fn canonical_os(value: &str) -> &'static str {
    let value = value.to_ascii_lowercase();
    if value == "unknown" { "unknown" }
    else if value.contains("windows") { "windows" }
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
        ("tool_use", &["tool_use", "tools"][..]),
    ] {
        if aliases.iter().any(|alias| capability_is(reported, alias)) {
            values.push(Capability { name, status: "supported", evidence: evidence.clone() });
        }
    }
    values
}

/// Build v2 from the same already-collected observations as v1. Missing facts
/// remain unknown; capability names, hardware and compatibility are never guessed.
#[cfg(test)]
pub fn build_inventory_v2(input: ModelInventoryInput<'_>) -> Option<ModelRuntimeInventoryV2> {
    let observed_at = now_unix_ms();
    build_inventory_v2_at(input, observed_at, observed_at, observed_at)
}

/// Build v2 with a conservative timestamp captured before runtime discovery began.
pub fn build_inventory_v2_with_observation(
    input: ModelInventoryInput<'_>,
    runtime_observed_at_unix_ms: Option<u64>,
    os_observed_at_unix_ms: Option<u64>,
) -> Option<ModelRuntimeInventoryV2> {
    build_inventory_v2_at(input, now_unix_ms(), runtime_observed_at_unix_ms, os_observed_at_unix_ms)
}

fn build_inventory_v2_at(
    input: ModelInventoryInput<'_>,
    generated_at: Option<u64>,
    runtime_observed_at_unix_ms: Option<u64>,
    os_observed_at_unix_ms: Option<u64>,
) -> Option<ModelRuntimeInventoryV2> {
    if [generated_at, runtime_observed_at_unix_ms, os_observed_at_unix_ms]
        .into_iter()
        .flatten()
        .any(|value| value == 0 || value > MAX_SCHEMA_SAFE_INTEGER)
    {
        return None;
    }
    let generated_at = generated_at?;
    // Observation timestamps are captured before inventory assembly. Reject an
    // inconsistent/future observation instead of emitting evidence newer than
    // the descriptor that claims to contain it.
    if [runtime_observed_at_unix_ms, os_observed_at_unix_ms]
        .into_iter()
        .flatten()
        .any(|observed_at| observed_at > generated_at)
    {
        return None;
    }
    let ModelInventoryInput {
        models, metadata, runtime_available, runtime_version, os_name: _, os_version,
        architecture, cpu_logical_cores, memory_total_mb, memory_available_mb, accelerator,
    } = input;
    let (memory_total_mb, memory_available_mb) =
        normalize_memory_readings(memory_total_mb, memory_available_mb);
    let platform_evidence = os_observed_at_unix_ms
        .map(Evidence::verified_os)
        .unwrap_or_else(Evidence::unknown);
    let hardware_has_measurement = cpu_logical_cores.is_some()
        || memory_total_mb.is_some() || memory_available_mb.is_some();
    let hardware_evidence = if hardware_has_measurement {
        os_observed_at_unix_ms
            .map(Evidence::verified_os)
            .unwrap_or_else(Evidence::unknown)
    } else { Evidence::unknown() };
    let runtime_evidence = if runtime_available {
        runtime_observed_at_unix_ms
            .map(|observed_at| Evidence::observed("runtime_api", observed_at))
            .unwrap_or_else(Evidence::unknown)
    } else { Evidence::unknown() };

    // Keep the wire descriptor within the shared schema's hard bounds.
    // Invalid names are omitted rather than truncated into a different model ID.
    const MAX_MODEL_REF_CHARS: usize = 160;
    const MAX_MODEL_SIZE_BYTES: u64 = 1_099_511_627_776;
    const MAX_CONTEXT_TOKENS: u64 = 10_000_000;
    let inventory_models = models
        .iter()
        .filter(|model| {
            !model.name.trim().is_empty()
                && model.name.chars().count() <= MAX_MODEL_REF_CHARS
                && !model.name.chars().any(char::is_control)
        })
        .take(512)
        .map(|model| {
            let details = metadata
                .get(&model.name)
                .filter(|item| crate::ollama::metadata_matches_model_identity(model, item));
            let reported_capabilities = details
                .map(|item| item.capabilities.as_slice())
                .unwrap_or(&[]);
            let model_evidence = runtime_observed_at_unix_ms
                .map(|observed_at| {
                    if details.is_some() {
                        Evidence::observed("runtime_metadata", observed_at)
                    } else {
                        Evidence::observed("runtime_api", observed_at)
                    }
                })
                .unwrap_or_else(Evidence::unknown);
            let context_tokens = details
                .and_then(|item| item.context_length_tokens)
                .filter(|tokens| *tokens > 0 && *tokens <= MAX_CONTEXT_TOKENS);
            let memory_estimate =
                ujani_core::model_advisor::estimate_model_working_memory_mb(model.size)
                    .filter(|value| *value > 0 && *value <= 16_777_216);
            let has_tool_use = capability_is(reported_capabilities, "tool_use")
                || capability_is(reported_capabilities, "tools");
            let context_evidence = if context_tokens.is_some() {
                model_evidence.clone()
            } else {
                Evidence::unknown()
            };
            let resource_evidence = if memory_estimate.is_some() {
                Evidence::estimate(generated_at)
            } else {
                Evidence::unknown()
            };
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
            size_bytes: (model.size > 0 && model.size <= MAX_MODEL_SIZE_BYTES).then_some(model.size),
            capabilities: capabilities(reported_capabilities, &model_evidence),
            context: Context { max_tokens: context_tokens, evidence: context_evidence },
            tool_use: BooleanCapability {
                supported: if has_tool_use { Some(true) } else { None },
                evidence: if has_tool_use { model_evidence.clone() } else { Evidence::unknown() },
            },
            // Ollama's current metadata adapter does not report image-size/MIME
            // limits or embedding dimensions, so those detailed capabilities stay unknown.
            vision: VisionCapability {
                supported: None,
                max_image_bytes: None,
                supported_mime_types: Vec::new(),
                evidence: Evidence::unknown(),
            },
            embeddings: EmbeddingsCapability {
                supported: None,
                dimensions: None,
                evidence: Evidence::unknown(),
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
        })
        .collect();

    Some(ModelRuntimeInventoryV2 {
        schema_version: 2,
        generated_at_unix_ms: generated_at,
        collection_context: CollectionContext {
            scope: "personal_local_only",
            owner_consent: "not_required_local_only",
            organisation_authorisation: "not_applicable",
            policy_version: "public-local-v2",
        },
        platform: Platform {
            // sysinfo reports distribution names (for example Ubuntu); use the
            // compile-time OS family as the canonical platform identifier.
            os: canonical_os(std::env::consts::OS),
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
            // Ollama /api/ps reports active GPU allocation, not physical VRAM
            // capacity. Record only confirmed active GPU use and never present
            // allocated bytes as total accelerator memory.
            accelerators: accelerator
                .filter(|observation| observation.gpu_vram_allocated_bytes > 0)
                .map(|_| vec![Accelerator {
                    kind: "gpu",
                    vendor: None,
                    device_family: None,
                    memory_mb: None,
                    runtime_api: Some("Ollama /api/ps".to_string()),
                    evidence: runtime_observed_at_unix_ms
                        .map(|observed_at| Evidence::observed("runtime_api", observed_at))
                        .unwrap_or_else(Evidence::unknown),
                }])
                .unwrap_or_default(),
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ollama::{
        AcceleratorObservation as OllamaAcceleratorObservation, OllamaModel, OllamaModelDetails,
        OllamaModelMetadata,
    };
    use std::collections::HashMap;

    fn sample_digest() -> String {
        format!("sha256:{}", "a".repeat(64))
    }

    fn sample_model() -> OllamaModel {
        OllamaModel {
            name: "llama3.2:latest".into(),
            size: 1_000_000_000,
            digest: Some(sample_digest()),
            details: Some(OllamaModelDetails {
                format: Some("gguf".into()), family: Some("llama".into()),
                families: None, parameter_size: Some("3B".into()),
                quantization_level: Some("Q4_K_M".into()),
            }),
        }
    }
    #[test]
    fn invalid_or_unrepresentable_wall_clock_never_produces_a_fabricated_timestamp() {
        assert_eq!(unix_ms_at(UNIX_EPOCH), None);
        assert_eq!(unix_ms_at(UNIX_EPOCH - Duration::from_millis(1)), None);
        assert_eq!(
            unix_ms_from_duration(Duration::from_millis(MAX_SCHEMA_SAFE_INTEGER)),
            Some(MAX_SCHEMA_SAFE_INTEGER)
        );
        assert_eq!(
            unix_ms_from_duration(Duration::from_millis(MAX_SCHEMA_SAFE_INTEGER + 1)),
            None
        );
        assert_eq!(unix_ms_from_duration(Duration::from_millis(u64::MAX)), None);
        assert_eq!(
            unix_ms_from_duration(Duration::from_millis(u64::MAX) + Duration::from_millis(1)),
            None
        );
    }

    #[test]
    fn supplied_timestamps_must_fit_the_schema_safe_integer_range() {
        let models: [OllamaModel; 0] = [];
        let metadata = HashMap::new();
        let input = || ModelInventoryInput {
            models: &models,
            metadata: &metadata,
            runtime_available: true,
            runtime_version: None,
            os_name: "Linux".into(),
            os_version: None,
            architecture: "x86_64".into(),
            cpu_logical_cores: None,
            memory_total_mb: None,
            memory_available_mb: None,
            accelerator: None,
        };
        let above_max = MAX_SCHEMA_SAFE_INTEGER + 1;
        assert!(build_inventory_v2_at(input(), Some(above_max), None, None).is_none());
        assert!(build_inventory_v2_at(input(), Some(1), Some(above_max), None).is_none());
        assert!(build_inventory_v2_at(input(), Some(1), None, Some(above_max)).is_none());
        assert!(build_inventory_v2_at(input(), Some(1), Some(0), None).is_none());
        assert!(build_inventory_v2_at(input(), Some(10), Some(11), None).is_none());
        assert!(build_inventory_v2_at(input(), Some(10), None, Some(11)).is_none());
        assert!(build_inventory_v2_at(input(), Some(10), Some(10), Some(10)).is_some());
    }

    #[test]
    fn unavailable_clock_omits_the_entire_v2_inventory() {
        let models: [OllamaModel; 0] = [];
        let metadata = HashMap::new();
        let inventory = build_inventory_v2_at(ModelInventoryInput {
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
        }, None, None, None);
        assert!(inventory.is_none());
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
        }).expect("valid system clock"))
        .unwrap();
        assert_eq!(value["schema_version"], 2);
        assert_eq!(value["collection_context"]["scope"], "personal_local_only");
        assert_eq!(value["collection_context"]["owner_consent"], "not_required_local_only");
        assert!(value.get("device_ref").is_none());
        assert_eq!(value["platform"]["os"], canonical_os(std::env::consts::OS));
        assert_eq!(value["platform"]["evidence"]["confidence"], "verified");
        assert_eq!(value["hardware"]["evidence"]["confidence"], "verified");
        assert_eq!(value["models"][0]["availability"], "installed");
        assert_eq!(value["models"][0]["compatibility"]["status"], "unknown");
        assert_eq!(value["models"][0]["capabilities"].as_array().unwrap().len(), 0);
        assert_eq!(value["models"][0]["resource_estimates"]["minimum_memory_mb"], 2_420);
        assert_eq!(value["models"][0]["resource_estimates"]["evidence"]["confidence"], "estimated");
        assert_eq!(value["privacy"]["contains_prompts"], false);
        assert_eq!(value["privacy"]["contains_user_files"], false);
        assert_eq!(value["privacy"]["contains_local_paths"], false);
        assert_eq!(value["privacy"]["contains_credentials"], false);
    }

    #[test]
    fn ubuntu_distribution_name_does_not_change_canonical_linux_family() {
        let models: [OllamaModel; 0] = [];
        let metadata = HashMap::new();
        let value = serde_json::to_value(build_inventory_v2(ModelInventoryInput {
            models: &models, metadata: &metadata, runtime_available: true,
            runtime_version: None, os_name: "Ubuntu".into(), os_version: Some("24.04".into()),
            architecture: "x86_64".into(), cpu_logical_cores: None,
            memory_total_mb: None, memory_available_mb: None, accelerator: None,
        }).expect("valid system clock")).unwrap();
        assert_eq!(value["platform"]["os"], canonical_os(std::env::consts::OS));
        assert_ne!(value["platform"]["os"], "other");
    }

    #[test]
    fn runtime_evidence_uses_collection_start_not_inventory_assembly_time() {
        let models = [sample_model()];
        let metadata = HashMap::from([("llama3.2:latest".to_string(), OllamaModelMetadata {
            model_digest: Some(sample_digest()),
            context_length_tokens: Some(8192), capabilities: vec!["chat".into()],
        })]);
        let value = serde_json::to_value(build_inventory_v2_at(ModelInventoryInput {
            models: &models, metadata: &metadata, runtime_available: true,
            runtime_version: Some("0.5.0".into()), os_name: "Ubuntu".into(),
            os_version: None, architecture: "x86_64".into(), cpu_logical_cores: Some(8),
            memory_total_mb: Some(16_384), memory_available_mb: Some(8_192), accelerator: None,
        }, Some(5_000), Some(1_000), Some(4_000)).expect("timestamps are valid")).unwrap();
        assert_eq!(value["generated_at_unix_ms"], 5_000);
        assert_eq!(value["platform"]["evidence"]["observed_at_unix_ms"], 4_000);
        assert_eq!(value["hardware"]["evidence"]["observed_at_unix_ms"], 4_000);
        assert_eq!(value["runtimes"][0]["evidence"]["observed_at_unix_ms"], 1_000);
        assert_eq!(value["models"][0]["evidence"]["observed_at_unix_ms"], 1_000);
        assert_eq!(value["models"][0]["context"]["evidence"]["observed_at_unix_ms"], 1_000);
    }

    #[test]
    fn ignores_runtime_metadata_without_a_matching_immutable_model_digest() {
        let models = [sample_model()];
        for metadata_digest in [None, Some(format!("sha256:{}", "b".repeat(64)))] {
            let metadata = HashMap::from([(
                "llama3.2:latest".to_string(),
                OllamaModelMetadata {
                    model_digest: metadata_digest,
                    context_length_tokens: Some(8_192),
                    capabilities: vec!["chat".into()],
                },
            )]);
            let value = serde_json::to_value(build_inventory_v2(ModelInventoryInput {
                models: &models,
                metadata: &metadata,
                runtime_available: true,
                runtime_version: None,
                os_name: "Linux".into(),
                os_version: None,
                architecture: "x86_64".into(),
                cpu_logical_cores: None,
                memory_total_mb: None,
                memory_available_mb: None,
                accelerator: None,
            }).expect("valid system clock")).unwrap();
            assert!(value["models"][0]["context"]["max_tokens"].is_null());
            assert!(value["models"][0]["capabilities"].as_array().unwrap().is_empty());
            assert_eq!(value["models"][0]["context"]["evidence"]["confidence"], "unknown");
        }
    }

    #[test]
    fn only_runtime_reported_capabilities_are_marked_supported() {
        let models = [sample_model()];
        let metadata = HashMap::from([("llama3.2:latest".to_string(), OllamaModelMetadata {
            model_digest: Some(sample_digest()),
            context_length_tokens: Some(8192),
            capabilities: vec!["chat".into(), "vision".into(), "tools".into()],
        })]);
        let value = serde_json::to_value(build_inventory_v2(ModelInventoryInput {
            models: &models, metadata: &metadata, runtime_available: true,
            runtime_version: None, os_name: "Darwin".into(), os_version: None,
            architecture: "arm64".into(), cpu_logical_cores: None,
            memory_total_mb: None, memory_available_mb: None, accelerator: None,
        }).expect("valid system clock")).unwrap();
        assert_eq!(value["models"][0]["capabilities"].as_array().unwrap().len(), 2);
        assert_eq!(value["models"][0]["capabilities"][0]["name"], "chat");
        assert_eq!(value["models"][0]["capabilities"][1]["name"], "tool_use");
        assert_eq!(value["models"][0]["tool_use"]["supported"], true);
        assert_eq!(value["models"][0]["vision"]["supported"], serde_json::Value::Null);
        assert_eq!(value["models"][0]["vision"]["evidence"]["confidence"], "unknown");
        assert_eq!(value["models"][0]["context"]["max_tokens"], 8192);
        assert_eq!(value["models"][0]["compatibility"]["status"], "unknown");
        assert_eq!(value["models"][0]["embeddings"]["supported"], serde_json::Value::Null);
        assert_eq!(value["models"][0]["embeddings"]["evidence"]["confidence"], "unknown");
    }

    #[test]
    fn reports_only_active_gpu_use_without_mislabeling_allocated_vram_as_capacity() {
        let models: [OllamaModel; 0] = [];
        let metadata = HashMap::new();
        let accelerator = OllamaAcceleratorObservation {
            gpu_vram_allocated_bytes: 2_147_483_648,
            running_models: 1,
            evidence: "ollama_api_ps".into(),
        };
        let value = serde_json::to_value(build_inventory_v2(ModelInventoryInput {
            models: &models,
            metadata: &metadata,
            runtime_available: true,
            runtime_version: None,
            os_name: "Linux".into(),
            os_version: None,
            architecture: "x86_64".into(),
            cpu_logical_cores: None,
            memory_total_mb: None,
            memory_available_mb: None,
            accelerator: Some(&accelerator),
        }).expect("valid system clock"))
        .unwrap();

        assert_eq!(value["hardware"]["accelerators"].as_array().unwrap().len(), 1);
        assert_eq!(value["hardware"]["accelerators"][0]["kind"], "gpu");
        assert_eq!(value["hardware"]["accelerators"][0]["runtime_api"], "Ollama /api/ps");
        assert_eq!(value["hardware"]["accelerators"][0]["memory_mb"], serde_json::Value::Null);
        assert_eq!(value["hardware"]["accelerators"][0]["evidence"]["source"], "runtime_api");
        assert_eq!(value["hardware"]["accelerators"][0]["evidence"]["confidence"], "reported");
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
        }).expect("valid system clock")).unwrap();
        assert_eq!(value["hardware"]["evidence"]["confidence"], "unknown");
        assert_eq!(value["runtimes"][0]["evidence"]["confidence"], "unknown");
        assert_eq!(value["models"].as_array().unwrap().len(), 0);
    }
}
