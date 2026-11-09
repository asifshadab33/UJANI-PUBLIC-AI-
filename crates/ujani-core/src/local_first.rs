//! Bounded local-first task partition planning.
//!
//! This is a side-effect-free planner: it never reads task content, transfers data,
//! invokes a model, or grants external approval. It identifies the smallest declared
//! segments whose required capabilities are unavailable locally. Callers must obtain
//! explicit approval for each external segment before any transfer.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const MAX_SEGMENTS: usize = 64;
const MAX_ID_BYTES: usize = 64;
const MAX_CAPABILITIES_PER_SEGMENT: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalFirstPolicy {
    /// External assistance is never permitted.
    LocalOnly,
    /// Keep supported work local; external fallback is a proposal requiring approval.
    LocalFirstWithApproval,
}

impl Default for LocalFirstPolicy {
    fn default() -> Self { Self::LocalFirstWithApproval }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSegment {
    pub id: String,
    /// Stable capability IDs from the trusted capability registry.
    pub required_capabilities: Vec<String>,
    /// Whether a separately configured external provider can perform this segment.
    pub external_fallback_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentPlacement {
    Local,
    NeedsExternalApproval,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentDecision {
    pub segment_id: String,
    pub placement: SegmentPlacement,
    /// Missing local capabilities; does not imply that external execution is approved.
    pub missing_capabilities: Vec<String>,
    /// True only when this exact segment is proposed for external assistance.
    pub external_approval_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFirstPlan {
    pub policy: LocalFirstPolicy,
    pub decisions: Vec<SegmentDecision>,
    pub local_segment_ids: Vec<String>,
    pub external_candidate_segment_ids: Vec<String>,
    pub unsupported_segment_ids: Vec<String>,
    /// Planning never transfers content or starts execution.
    pub execution_started: bool,
    pub data_transferred: bool,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value.chars().all(|c| c.is_ascii_alphanumeric() || "._:-".contains(c))
}

/// Partition a task by declared capability requirements, retaining all locally
/// supportable segments. External candidates are segment IDs only, never content.
pub fn plan_local_first(
    segments: &[TaskSegment],
    available_capabilities: &BTreeSet<String>,
    policy: LocalFirstPolicy,
) -> Result<LocalFirstPlan, &'static str> {
    if segments.is_empty() || segments.len() > MAX_SEGMENTS {
        return Err("A local-first plan must contain between 1 and 64 segments.");
    }

    let mut seen = BTreeSet::new();
    let mut decisions = Vec::with_capacity(segments.len());
    let mut local_segment_ids = Vec::new();
    let mut external_candidate_segment_ids = Vec::new();
    let mut unsupported_segment_ids = Vec::new();

    for segment in segments {
        if !valid_id(&segment.id) || !seen.insert(segment.id.as_str()) {
            return Err("Task segment IDs must be valid and unique.");
        }
        if segment.required_capabilities.len() > MAX_CAPABILITIES_PER_SEGMENT
            || segment.required_capabilities.iter().any(|id| !valid_id(id))
        {
            return Err("A segment contains invalid or too many capability IDs.");
        }
        let required: BTreeSet<_> = segment.required_capabilities.iter().cloned().collect();
        let missing: Vec<_> = required.difference(available_capabilities).cloned().collect();

        let (placement, external_approval_required) = if missing.is_empty() {
            local_segment_ids.push(segment.id.clone());
            (SegmentPlacement::Local, false)
        } else if policy == LocalFirstPolicy::LocalFirstWithApproval && segment.external_fallback_available {
            external_candidate_segment_ids.push(segment.id.clone());
            (SegmentPlacement::NeedsExternalApproval, true)
        } else {
            unsupported_segment_ids.push(segment.id.clone());
            (SegmentPlacement::Unsupported, false)
        };

        decisions.push(SegmentDecision {
            segment_id: segment.id.clone(),
            placement,
            missing_capabilities: missing,
            external_approval_required,
        });
    }

    Ok(LocalFirstPlan {
        policy,
        decisions,
        local_segment_ids,
        external_candidate_segment_ids,
        unsupported_segment_ids,
        execution_started: false,
        data_transferred: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(id: &str, caps: &[&str], fallback: bool) -> TaskSegment {
        TaskSegment {
            id: id.into(),
            required_capabilities: caps.iter().map(|s| s.to_string()).collect(),
            external_fallback_available: fallback,
        }
    }

    #[test]
    fn keeps_supported_segments_local_and_proposes_only_missing_segments() {
        let available = BTreeSet::from(["text.extract".to_string(), "language.en".to_string()]);
        let plan = plan_local_first(
            &[
                segment("extract", &["text.extract"], false),
                segment("english", &["language.en"], false),
                segment("unsupported-language", &["language.xx"], true),
            ],
            &available,
            LocalFirstPolicy::default(),
        ).unwrap();

        assert_eq!(plan.local_segment_ids, vec!["extract".to_string(), "english".to_string()]);
        assert_eq!(plan.external_candidate_segment_ids, vec!["unsupported-language".to_string()]);
        assert!(plan.unsupported_segment_ids.is_empty());
        assert!(plan.decisions[2].external_approval_required);
        assert!(!plan.execution_started);
        assert!(!plan.data_transferred);
    }

    #[test]
    fn local_only_never_proposes_external_fallback() {
        let plan = plan_local_first(
            &[segment("ocr", &["ocr"], true)],
            &BTreeSet::new(),
            LocalFirstPolicy::LocalOnly,
        ).unwrap();
        assert!(plan.external_candidate_segment_ids.is_empty());
        assert_eq!(plan.unsupported_segment_ids, vec!["ocr".to_string()]);
    }

    #[test]
    fn missing_fallback_is_reported_as_unsupported() {
        let plan = plan_local_first(
            &[segment("special-parser", &["parser.special"], false)],
            &BTreeSet::new(),
            LocalFirstPolicy::default(),
        ).unwrap();
        assert_eq!(plan.unsupported_segment_ids, vec!["special-parser".to_string()]);
    }

    #[test]
    fn rejects_duplicate_or_invalid_segments_and_oversized_plans() {
        assert!(plan_local_first(
            &[segment("same", &[], false), segment("same", &[], false)],
            &BTreeSet::new(),
            LocalFirstPolicy::default(),
        ).is_err());
        assert!(plan_local_first(
            &[segment("../escape", &[], false)],
            &BTreeSet::new(),
            LocalFirstPolicy::default(),
        ).is_err());
        let too_many = (0..65).map(|i| segment(&format!("s{i}"), &[], false)).collect::<Vec<_>>();
        assert!(plan_local_first(&too_many, &BTreeSet::new(), LocalFirstPolicy::default()).is_err());
    }
}
