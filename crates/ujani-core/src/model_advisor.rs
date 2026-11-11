//! Conservative, explainable local model fit estimates; never guarantees fit.
use serde::{Deserialize, Serialize};

const MIB: u64 = 1024 * 1024;
const RUNTIME_MULTIPLIER: u64 = 2;
const FIXED_RUNTIME_OVERHEAD_MB: u64 = 512;
const USABLE_MEMORY_NUMERATOR: u64 = 3;
const USABLE_MEMORY_DENOMINATOR: u64 = 4;

/// Estimate working memory from model disk size using the shared advisory heuristic.
/// This is an estimate only; it is not runtime telemetry or a fit guarantee.
pub fn estimate_model_working_memory_mb(size_bytes: u64) -> Option<u64> {
    if size_bytes == 0 {
        return None;
    }
    let size_mb = size_bytes.saturating_add(MIB - 1) / MIB;
    Some(
        size_mb
            .saturating_mul(RUNTIME_MULTIPLIER)
            .saturating_add(FIXED_RUNTIME_OVERHEAD_MB),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledModelSize {
    pub name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelFitEstimate {
    pub name: String,
    pub size_bytes: u64,
    pub estimated_working_memory_mb: Option<u64>,
    pub estimated_to_fit: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRecommendation {
    pub recommended_model: Option<String>,
    pub available_memory_mb: u64,
    pub usable_memory_budget_mb: u64,
    pub candidates: Vec<ModelFitEstimate>,
    pub explanation: String,
}

/// Uses model disk size × 2 + 512 MiB and reserves 25% of currently available RAM.
/// This is a conservative advisory heuristic, not a runtime guarantee or model benchmark.
pub fn recommend_local_model(
    models: &[InstalledModelSize],
    available_memory_mb: u64,
) -> ModelRecommendation {
    let usable_memory_budget_mb = available_memory_mb
        .saturating_mul(USABLE_MEMORY_NUMERATOR)
        / USABLE_MEMORY_DENOMINATOR;

    let mut candidates: Vec<ModelFitEstimate> = models
        .iter()
        .map(|model| {
            let estimated_working_memory_mb =
                estimate_model_working_memory_mb(model.size_bytes);
            let estimated_to_fit = estimated_working_memory_mb.is_some_and(|required| {
                available_memory_mb > 0 && required <= usable_memory_budget_mb
            });
            let reason = match estimated_working_memory_mb {
                None => "Model size unknown; fit cannot be estimated.".into(),
                Some(_) if available_memory_mb == 0 => {
                    "Available RAM unknown; no recommendation.".into()
                }
                Some(required) if estimated_to_fit => format!(
                    "Rough estimate {required} MiB fits advisory budget {usable_memory_budget_mb} MiB."
                ),
                Some(required) => format!(
                    "Rough estimate {required} MiB exceeds advisory budget {usable_memory_budget_mb} MiB."
                ),
            };

            ModelFitEstimate {
                name: model.name.clone(),
                size_bytes: model.size_bytes,
                estimated_working_memory_mb,
                estimated_to_fit,
                reason,
            }
        })
        .collect();

    // Prefer the smallest estimated fit. Name is a deterministic tie-breaker.
    // This is not a model-quality ranking and never changes the user's selection.
    candidates.sort_by(|a, b| match (a.estimated_to_fit, b.estimated_to_fit) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a
            .estimated_working_memory_mb
            .cmp(&b.estimated_working_memory_mb)
            .then_with(|| a.name.cmp(&b.name)),
    });

    let recommended_model = candidates
        .iter()
        .find(|candidate| candidate.estimated_to_fit)
        .map(|candidate| candidate.name.clone());

    ModelRecommendation {
        recommended_model,
        available_memory_mb,
        usable_memory_budget_mb,
        candidates,
        explanation: "Advisory only: model disk size × 2 + 512 MiB, with 25% RAM headroom. Quantisation, context length, runtime and accelerator offload change real usage. No automatic model switching.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(name: &str, size_mb: u64) -> InstalledModelSize {
        InstalledModelSize {
            name: name.into(),
            size_bytes: size_mb * MIB,
        }
    }

    #[test]
    fn recommends_smallest_estimated_fit() {
        let result = recommend_local_model(
            &[model("large", 3_000), model("small", 500), model("medium", 1_000)],
            6_000,
        );
        assert_eq!(result.recommended_model.as_deref(), Some("small"));
        assert!(result.candidates[0].estimated_to_fit);
    }

    #[test]
    fn unknown_ram_never_recommends() {
        let result = recommend_local_model(&[model("tiny", 1)], 0);
        assert!(result.recommended_model.is_none());
        assert!(!result.candidates[0].estimated_to_fit);
    }

    #[test]
    fn unknown_size_never_recommends() {
        let result = recommend_local_model(
            &[InstalledModelSize {
                name: "unknown".into(),
                size_bytes: 0,
            }],
            32_000,
        );
        assert!(result.recommended_model.is_none());
        assert!(result.candidates[0].estimated_working_memory_mb.is_none());
    }

    #[test]
    fn working_memory_estimate_uses_rounded_mebibytes_and_fixed_overhead() {
        assert_eq!(estimate_model_working_memory_mb(0), None);
        assert_eq!(estimate_model_working_memory_mb(1), Some(514));
        assert_eq!(estimate_model_working_memory_mb(512 * MIB), Some(1_536));
    }

    #[test]
    fn oversized_candidate_is_not_recommended() {
        let result = recommend_local_model(&[model("large", 8_000)], 8_000);
        assert!(result.recommended_model.is_none());
        assert!(!result.candidates[0].estimated_to_fit);
    }

    #[test]
    fn exact_advisory_budget_boundary_is_accepted() {
        // 512 MiB on disk estimates to 1,536 MiB; 75% of 2,048 MiB is 1,536 MiB.
        let result = recommend_local_model(&[model("boundary", 512)], 2_048);
        assert_eq!(result.usable_memory_budget_mb, 1_536);
        assert_eq!(result.recommended_model.as_deref(), Some("boundary"));
    }

    #[test]
    fn equal_estimates_have_deterministic_name_order() {
        let result = recommend_local_model(&[model("zeta", 256), model("alpha", 256)], 4_096);
        assert_eq!(result.candidates[0].name, "alpha");
        assert_eq!(result.candidates[1].name, "zeta");
    }

    #[test]
    fn empty_catalogue_returns_no_recommendation() {
        let result = recommend_local_model(&[], 8_192);
        assert!(result.recommended_model.is_none());
        assert!(result.candidates.is_empty());
    }
}
