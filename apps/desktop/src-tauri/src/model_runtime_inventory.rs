//! Read-only, privacy-minimised model/runtime inventory matching schemas/model-runtime-inventory-v1.schema.json.
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::System;

#[derive(Debug, Clone, Serialize)]
pub struct Evidence {
    pub source: &'static str,
    pub confidence: &'static str,
    pub observed_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlatformInfo {
    pub os: String,
    pub os_version: Option<String>,
    pub architecture: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AcceleratorInfo {
    pub kind: &'static str,
    pub vendor: Option<String>,
    pub device_family: Option<String>,
    pub memory_mb: Option<u64>,
    pub runtime_api: Option<String>,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Serialize)]
pub struct HardwareInfo {
    pub cpu_logical_cores: Option<u16>,
    pub memory_total_mb: Option<u64>,
    pub memory_available_mb: Option<u64>,
    pub storage_free_mb: Option<u64>,
    pub accelerators: Vec<AcceleratorInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeInfo {
    pub runtime_id: String,
    pub provider: String,
    pub version: Option<String>,
    pub availability: &'static str,
    pub network_scope: &'static str,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub model_ref: String,
    pub runtime_id: String,
    pub display_name: String,
    pub family: Option<String>,
    pub version: Option<String>,
    pub availability: &'static str,
    pub size_bytes: Option<u64>,
    pub context_length_tokens: Option<u64>,
    pub estimated_min_memory_mb: Option<u64>,
    pub capabilities: Vec<&'static str>,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrivacyInfo {
    pub contains_prompts: bool,
    pub contains_user_files: bool,
    pub contains_local_paths: bool,
    pub contains_credentials: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelRuntimeInventory {
    pub schema_version: u8,
    pub generated_at_unix_ms: u64,
    pub device_ref: Option<String>,
    pub platform: PlatformInfo,
    pub hardware: HardwareInfo,
    pub runtimes: Vec<RuntimeInfo>,
    pub models: Vec<ModelInfo>,
    pub privacy: PrivacyInfo,
}

fn observed_at() -> Result<u64, String> {
    let value = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|error| format!("System clock is before Unix epoch: {error}"))?
        .as_millis().min(u64::MAX as u128) as u64;
    Ok(value.max(1))
}

fn valid_model_ref(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 153
        && name
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._:-/".contains(c))
}

pub async fn collect() -> Result<ModelRuntimeInventory, String> {
    let now = observed_at()?;
    let mut system = System::new();
    system.refresh_cpu();
    system.refresh_memory();

    let cores = u16::try_from(system.cpus().len()).ok().filter(|count| *count > 0);
    let memory_total_mb = Some(system.total_memory() / (1024 * 1024)).filter(|value| *value > 0);
    let memory_available_mb = Some(system.available_memory() / (1024 * 1024));

    // Ollama's adapter is hard-coded to 127.0.0.1 and disables proxies/redirects.
    // A failed probe is represented as Unknown: it may be stopped, slow or inaccessible.
    let (runtime, models) = match crate::ollama::list_models().await {
        Ok(installed) => {
            if installed.len() > 512 {
                return Err("Local Ollama reports more than the 512-model inventory safety limit; inventory was not truncated silently.".into());
            }
            let mut models = Vec::with_capacity(installed.len());
            for model in installed {
                if !valid_model_ref(&model.name) {
                    return Err("Local Ollama returned a model identifier outside the inventory contract; no partial inventory was returned.".into());
                }
                models.push(ModelInfo {
                    model_ref: format!("ollama:{}", model.name),
                    runtime_id: "ollama.local".into(),
                    display_name: model.name,
                    family: None,
                    version: None,
                    availability: "installed",
                    size_bytes: Some(model.size),
                    context_length_tokens: None,
                    estimated_min_memory_mb: None,
                    capabilities: vec!["unknown"],
                    evidence: Evidence { source: "runtime_api", confidence: "reported", observed_at_unix_ms: Some(now) },
                });
            }
            (
                RuntimeInfo {
                    runtime_id: "ollama.local".into(),
                    provider: "Ollama".into(),
                    version: None,
                    availability: "available",
                    network_scope: "loopback_only",
                    evidence: Evidence { source: "runtime_api", confidence: "reported", observed_at_unix_ms: Some(now) },
                },
                models,
            )
        }
        Err(_) => (
            RuntimeInfo {
                runtime_id: "ollama.local".into(),
                provider: "Ollama".into(),
                version: None,
                availability: "unknown",
                network_scope: "loopback_only",
                evidence: Evidence { source: "unknown", confidence: "unknown", observed_at_unix_ms: Some(now) },
            },
            Vec::new(),
        ),
    };

    Ok(ModelRuntimeInventory {
        schema_version: 1,
        generated_at_unix_ms: now,
        device_ref: None,
        platform: PlatformInfo {
            os: std::env::consts::OS.into(),
            os_version: System::long_os_version(),
            architecture: std::env::consts::ARCH.into(),
        },
        hardware: HardwareInfo {
            cpu_logical_cores: cores,
            memory_total_mb,
            memory_available_mb,
            storage_free_mb: None,
            // Accelerator discovery remains Unknown until a supported OS/runtime API
            // can provide reliable device identity and memory evidence.
            accelerators: Vec::new(),
        },
        runtimes: vec![runtime],
        models,
        privacy: PrivacyInfo {
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

    #[test]
    fn model_identifier_is_bounded_and_does_not_accept_paths_outside_runtime_names() {
        assert!(valid_model_ref("llama3.2:latest"));
        assert!(valid_model_ref("library/qwen2.5:7b"));
        assert!(!valid_model_ref(""));
        assert!(!valid_model_ref("../outside"));
        assert!(!valid_model_ref("library/../outside"));
        assert!(!valid_model_ref("/absolute/model"));
        assert!(!valid_model_ref("library//model"));
        assert!(!valid_model_ref("library/./model"));
        assert!(!valid_model_ref(&"m".repeat(161)));
        assert!(!valid_model_ref("model name"));
    }

    #[test]
    fn inventory_serialization_matches_v1_required_shape() {
        let evidence = Evidence {
            source: "unknown",
            confidence: "unknown",
            observed_at_unix_ms: None,
        };
        let inventory = ModelRuntimeInventory {
            schema_version: 1,
            generated_at_unix_ms: 1,
            device_ref: None,
            platform: PlatformInfo {
                os: "linux".into(),
                os_version: None,
                architecture: "x86_64".into(),
            },
            hardware: HardwareInfo {
                cpu_logical_cores: None,
                memory_total_mb: None,
                memory_available_mb: None,
                storage_free_mb: None,
                accelerators: Vec::new(),
            },
            runtimes: vec![RuntimeInfo {
                runtime_id: "ollama.local".into(),
                provider: "Ollama".into(),
                version: None,
                availability: "unknown",
                network_scope: "loopback_only",
                evidence: evidence.clone(),
            }],
            models: vec![ModelInfo {
                model_ref: "ollama:example".into(),
                runtime_id: "ollama.local".into(),
                display_name: "example".into(),
                family: None,
                version: None,
                availability: "installed",
                size_bytes: Some(1),
                context_length_tokens: None,
                estimated_min_memory_mb: None,
                capabilities: vec!["unknown"],
                evidence: evidence.clone(),
            }],
            privacy: PrivacyInfo {
                contains_prompts: false,
                contains_user_files: false,
                contains_local_paths: false,
                contains_credentials: false,
            },
        };

        let value = serde_json::to_value(inventory).unwrap();
        let top_level = value.as_object().unwrap();
        for key in [
            "schema_version", "generated_at_unix_ms", "device_ref", "platform",
            "hardware", "runtimes", "models", "privacy",
        ] {
            assert!(top_level.contains_key(key), "missing v1 field: {key}");
        }
        assert_eq!(top_level.len(), 8, "unexpected top-level v1 field");
        assert_eq!(value["schema_version"], 1);
        assert!(value["device_ref"].is_null());
        assert!(value["platform"]["os_version"].is_null());
        assert!(value["hardware"]["accelerators"].is_array());
        assert_eq!(value["runtimes"][0]["availability"], "unknown");
        assert_eq!(value["models"][0]["capabilities"][0], "unknown");
        assert_eq!(value["privacy"]["contains_prompts"], false);
        assert_eq!(value["privacy"]["contains_user_files"], false);
        assert_eq!(value["privacy"]["contains_local_paths"], false);
        assert_eq!(value["privacy"]["contains_credentials"], false);
    }

    #[test]
    fn inventory_serializes_privacy_and_unknown_capabilities_explicitly() {
        let evidence = Evidence { source: "unknown", confidence: "unknown", observed_at_unix_ms: None };
        let model = ModelInfo {
            model_ref: "ollama:example".into(),
            runtime_id: "ollama.local".into(),
            display_name: "example".into(),
            family: None,
            version: None,
            availability: "installed",
            size_bytes: Some(1024),
            context_length_tokens: None,
            estimated_min_memory_mb: None,
            capabilities: vec!["unknown"],
            evidence,
        };
        let value = serde_json::to_value(model).unwrap();
        assert_eq!(value["capabilities"][0], "unknown");
        assert_eq!(value["size_bytes"], 1024);
        assert!(value["context_length_tokens"].is_null());
    }
}
