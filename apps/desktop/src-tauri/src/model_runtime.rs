//! Narrow local model adapter boundary. The bundled adapter is deliberately a mock.
use serde::Serialize;

const MAX_PROMPT_BYTES: usize = 16_000;
const MOCK_MODEL_ID: &str = "ujani.mock.echo.v1";

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub available: bool,
    pub is_mock: bool,
    pub required_capabilities: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InferenceResult {
    pub model_id: String,
    pub model_name: String,
    pub output: String,
    pub execution: String,
    pub is_mock: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    EmptyPrompt,
    PromptTooLarge,
    UnknownModel,
    CapabilityUnavailable,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::EmptyPrompt => "Prompt cannot be empty.",
            Self::PromptTooLarge => "Prompt exceeds the 16 KB local limit.",
            Self::UnknownModel => "The selected model is not in the trusted model registry.",
            Self::CapabilityUnavailable => "The selected model's required local capabilities are unavailable.",
        };
        f.write_str(message)
    }
}

pub trait LocalModelAdapter {
    fn model_info(&self) -> ModelInfo;
    fn infer(&self, prompt: &str) -> Result<InferenceResult, RuntimeError>;
}

/// Deterministic adapter for UI and contract testing only. This is not AI inference.
pub struct MockEchoAdapter {
    cpu_available: bool,
}

impl MockEchoAdapter {
    pub fn new(cpu_available: bool) -> Self {
        Self { cpu_available }
    }
}

impl LocalModelAdapter for MockEchoAdapter {
    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            id: MOCK_MODEL_ID.into(),
            name: "UJANI Mock Echo (test adapter)".into(),
            provider: "Built-in deterministic mock".into(),
            available: self.cpu_available,
            is_mock: true,
            required_capabilities: vec!["cpu".into()],
            status: if self.cpu_available { "Available for mock tests".into() } else { "CPU capability unavailable".into() },
        }
    }

    fn infer(&self, prompt: &str) -> Result<InferenceResult, RuntimeError> {
        let prompt = prompt.trim();
        if prompt.is_empty() { return Err(RuntimeError::EmptyPrompt); }
        if prompt.len() > MAX_PROMPT_BYTES { return Err(RuntimeError::PromptTooLarge); }
        if !self.cpu_available { return Err(RuntimeError::CapabilityUnavailable); }
        Ok(InferenceResult {
            model_id: MOCK_MODEL_ID.into(),
            model_name: "UJANI Mock Echo (test adapter)".into(),
            output: format!("MOCK RESPONSE — no model inference occurred. Input received locally ({} characters): {}", prompt.chars().count(), prompt),
            execution: "Local deterministic mock adapter".into(),
            is_mock: true,
        })
    }
}

/// Resolve only an explicitly registered model ID. "auto" is a UI selection path
/// that currently resolves to this one mock adapter, never to a remote provider.
pub fn run_local_model(model_id: &str, prompt: &str, cpu_available: bool) -> Result<InferenceResult, RuntimeError> {
    let adapter = MockEchoAdapter::new(cpu_available);
    if model_id != "auto" && model_id != MOCK_MODEL_ID {
        return Err(RuntimeError::UnknownModel);
    }
    adapter.infer(prompt)
}

pub fn discover_models(cpu_available: bool) -> Vec<ModelInfo> {
    vec![MockEchoAdapter::new(cpu_available).model_info()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_exposes_mock_honestly() {
        let models = discover_models(true);
        assert_eq!(models.len(), 1);
        assert!(models[0].is_mock);
        assert!(models[0].available);
    }

    #[test]
    fn auto_selection_resolves_only_to_local_mock() {
        let result = run_local_model("auto", "hello", true).unwrap();
        assert!(result.is_mock);
        assert_eq!(result.model_id, MOCK_MODEL_ID);
    }

    #[test]
    fn unknown_model_is_rejected() {
        assert_eq!(run_local_model("remote.model", "hello", true).unwrap_err(), RuntimeError::UnknownModel);
    }

    #[test]
    fn empty_and_oversized_prompts_are_rejected() {
        assert_eq!(run_local_model("auto", "  ", true).unwrap_err(), RuntimeError::EmptyPrompt);
        assert_eq!(run_local_model("auto", &"x".repeat(MAX_PROMPT_BYTES + 1), true).unwrap_err(), RuntimeError::PromptTooLarge);
    }

    #[test]
    fn missing_capability_blocks_inference() {
        assert_eq!(run_local_model("auto", "hello", false).unwrap_err(), RuntimeError::CapabilityUnavailable);
    }
}
