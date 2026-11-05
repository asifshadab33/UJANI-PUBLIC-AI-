//! Versioned, bounded task-graph validation and local planning.
//! This module validates declarative workflows only: it never executes nodes,
//! opens files, starts processes, or makes network requests.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MAX_GRAPH_NODES: usize = 32;
const MAX_NODE_ID_BYTES: usize = 64;
const SUPPORTED_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskNode {
    pub id: String,
    pub operation_id: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskGraph {
    pub schema_version: u16,
    pub nodes: Vec<TaskNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskGraphPlan {
    pub accepted: bool,
    pub schema_version: u16,
    pub ordered_node_ids: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub estimated_peak_memory_mb: u64,
    pub available_memory_mb: u64,
    pub reason: String,
    pub execution_started: bool,
}

fn valid_node_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_NODE_ID_BYTES
        && value.chars().all(|c| c.is_ascii_alphanumeric() || "._:-".contains(c))
}

fn rejected(graph: &TaskGraph, available_memory_mb: u64, reason: impl Into<String>) -> TaskGraphPlan {
    TaskGraphPlan {
        accepted: false,
        schema_version: graph.schema_version,
        ordered_node_ids: Vec::new(),
        required_capabilities: Vec::new(),
        estimated_peak_memory_mb: 0,
        available_memory_mb,
        reason: reason.into(),
        execution_started: false,
    }
}

/// Validate a graph and produce a deterministic topological order.
/// Nodes are assumed to execute sequentially, so peak memory is the maximum
/// per-operation minimum, not the sum. This is a coarse preflight, not a
/// runtime reservation or model-specific memory estimate.
pub fn plan_task_graph(
    graph: &TaskGraph,
    mode: &str,
    available_memory_mb: u64,
    capabilities: &[crate::capability::CapabilityRecord],
) -> TaskGraphPlan {
    if graph.schema_version != SUPPORTED_SCHEMA_VERSION {
        return rejected(graph, available_memory_mb, "Unsupported task-graph schema version.");
    }
    if mode != "fortress" && mode != "smart" {
        return rejected(graph, available_memory_mb, "Only local Fortress and Smart workflows can be planned; Worker and Partner require a secure external gateway.");
    }
    if graph.nodes.is_empty() || graph.nodes.len() > MAX_GRAPH_NODES {
        return rejected(graph, available_memory_mb, "A workflow must contain between 1 and 32 nodes.");
    }

    let operations = crate::capability::registered_operations();
    let operation_by_id: BTreeMap<_, _> = operations.into_iter().map(|op| (op.id.clone(), op)).collect();
    let mut node_by_id = BTreeMap::new();

    for node in &graph.nodes {
        if !valid_node_id(&node.id) {
            return rejected(graph, available_memory_mb, "A node has an invalid identifier.");
        }
        if node_by_id.insert(node.id.as_str(), node).is_some() {
            return rejected(graph, available_memory_mb, "Node identifiers must be unique.");
        }
        if !operation_by_id.contains_key(&node.operation_id) {
            return rejected(graph, available_memory_mb, format!("Operation '{}' is not in the trusted operation catalogue.", node.operation_id));
        }
    }

    let mut indegree = BTreeMap::<String, usize>::new();
    let mut dependents = BTreeMap::<String, Vec<String>>::new();
    for node in &graph.nodes {
        indegree.insert(node.id.clone(), 0);
    }

    for node in &graph.nodes {
        let mut unique_dependencies = BTreeSet::new();
        for dependency in &node.depends_on {
            if dependency == &node.id || !unique_dependencies.insert(dependency) {
                return rejected(graph, available_memory_mb, "A node contains a self-dependency or duplicate dependency.");
            }
            if !node_by_id.contains_key(dependency.as_str()) {
                return rejected(graph, available_memory_mb, format!("Dependency '{dependency}' does not exist."));
            }
            *indegree.get_mut(&node.id).expect("node inserted above") += 1;
            dependents.entry(dependency.clone()).or_default().push(node.id.clone());
        }
    }

    // BTreeSet makes ordering reproducible across runs and platforms.
    let mut ready: BTreeSet<String> = indegree.iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut ordered = VecDeque::new();
    while let Some(id) = ready.pop_first() {
        ordered.push_back(id.clone());
        if let Some(children) = dependents.get(&id) {
            for child in children {
                let degree = indegree.get_mut(child).expect("validated child");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(child.clone());
                }
            }
        }
    }
    if ordered.len() != graph.nodes.len() {
        return rejected(graph, available_memory_mb, "Workflow contains a dependency cycle.");
    }

    let mut required = BTreeSet::new();
    let mut peak_memory = 0_u64;
    for node in &graph.nodes {
        let operation = operation_by_id.get(&node.operation_id).expect("validated operation");
        required.extend(operation.required_capabilities.iter().cloned());
        peak_memory = peak_memory.max(operation.minimum_available_memory_mb);
    }

    let missing: Vec<String> = required.iter().filter(|id| {
        !capabilities.iter().any(|cap| cap.id == **id && cap.state == crate::capability::CapabilityState::Available)
    }).cloned().collect();
    if !missing.is_empty() {
        return rejected(graph, available_memory_mb, format!("Required capabilities are unavailable or unverified: {}.", missing.join(", ")));
    }
    if available_memory_mb < peak_memory {
        return rejected(graph, available_memory_mb, "Available memory is below the workflow's conservative minimum.");
    }

    TaskGraphPlan {
        accepted: true,
        schema_version: graph.schema_version,
        ordered_node_ids: ordered.into_iter().collect(),
        required_capabilities: required.into_iter().collect(),
        estimated_peak_memory_mb: peak_memory,
        available_memory_mb,
        reason: "Workflow graph is valid and locally plannable. No node has been executed and resources have not been reserved.".into(),
        execution_started: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{CapabilityRecord, CapabilityState};

    fn caps() -> Vec<CapabilityRecord> {
        vec![
            CapabilityRecord { id: "cpu".into(), category: "compute".into(), state: CapabilityState::Available, source: "test".into(), detail: "test".into() },
            CapabilityRecord { id: "ollama.local".into(), category: "model".into(), state: CapabilityState::Available, source: "test".into(), detail: "test".into() },
        ]
    }
    fn graph(nodes: Vec<TaskNode>) -> TaskGraph {
        TaskGraph { schema_version: 1, nodes }
    }
    fn node(id: &str, operation: &str, depends_on: &[&str]) -> TaskNode {
        TaskNode { id: id.into(), operation_id: operation.into(), depends_on: depends_on.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn plans_valid_graph_in_dependency_order() {
        let g = graph(vec![
            node("translate", "translate.text", &["summarize"]),
            node("analyze", "file.analyze_text", &[]),
            node("summarize", "document.summarize", &["analyze"]),
        ]);
        let plan = plan_task_graph(&g, "fortress", 4096, &caps());
        assert!(plan.accepted, "{}", plan.reason);
        assert_eq!(plan.ordered_node_ids, vec!["analyze", "summarize", "translate"]);
        assert!(!plan.execution_started);
    }

    #[test]
    fn rejects_unknown_operation_and_missing_dependency() {
        let unknown = graph(vec![node("bad", "shell.execute", &[])]);
        assert!(!plan_task_graph(&unknown, "smart", 4096, &caps()).accepted);
        let missing = graph(vec![node("a", "file.analyze_text", &["absent"])]);
        assert!(!plan_task_graph(&missing, "smart", 4096, &caps()).accepted);
    }

    #[test]
    fn rejects_cycles_and_duplicate_ids() {
        let cycle = graph(vec![node("a", "file.analyze_text", &["b"]), node("b", "file.analyze_text", &["a"])]);
        assert!(plan_task_graph(&cycle, "smart", 4096, &caps()).reason.contains("cycle"));
        let duplicate = graph(vec![node("a", "file.analyze_text", &[]), node("a", "file.analyze_text", &[])]);
        assert!(!plan_task_graph(&duplicate, "smart", 4096, &caps()).accepted);
    }

    #[test]
    fn rejects_unsupported_schema_modes_and_oversized_graphs() {
        let mut unsupported = graph(vec![node("a", "file.analyze_text", &[])]);
        unsupported.schema_version = 2;
        assert!(!plan_task_graph(&unsupported, "fortress", 4096, &caps()).accepted);
        let one = graph(vec![node("a", "file.analyze_text", &[])]);
        assert!(!plan_task_graph(&one, "worker", 4096, &caps()).accepted);
        let many = graph((0..33).map(|i| node(&format!("n{i}"), "file.analyze_text", &[])).collect());
        assert!(!plan_task_graph(&many, "smart", 4096, &caps()).accepted);
    }

    #[test]
    fn computes_sequential_peak_memory_and_blocks_missing_capabilities() {
        let g = graph(vec![node("a", "file.analyze_text", &[]), node("b", "translate.text", &["a"])]);
        let plan = plan_task_graph(&g, "smart", 4096, &caps());
        assert!(plan.accepted);
        assert_eq!(plan.estimated_peak_memory_mb, 512);
        assert!(!plan_task_graph(&g, "smart", 4096, &caps()[..1]).accepted);
        assert!(!plan_task_graph(&g, "smart", 128, &caps()).accepted);
    }

    #[test]
    fn rejects_self_and_duplicate_dependencies() {
        let self_dep = graph(vec![node("a", "file.analyze_text", &["a"])]);
        assert!(!plan_task_graph(&self_dep, "smart", 4096, &caps()).accepted);
        let dup_dep = graph(vec![node("a", "file.analyze_text", &[]), node("b", "file.analyze_text", &["a", "a"])]);
        assert!(!plan_task_graph(&dup_dep, "smart", 4096, &caps()).accepted);
    }
}
