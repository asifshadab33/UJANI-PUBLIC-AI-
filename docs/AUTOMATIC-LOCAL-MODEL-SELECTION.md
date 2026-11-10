# Automatic local model selection — first implementation

UJANI Public AI defaults to automatic model selection. This first implementation applies that policy to the existing Ollama local runtime; it is deliberately not presented as provider-neutral multi-runtime selection yet.

## Behaviour

- Automatic is the default selection mode.
- On explicit runtime discovery/refresh, UJANI requests the existing local model-fit recommendation and selects only a currently installed Ollama model that meets its conservative RAM-fit estimate.
- If no installed model meets that estimate, automatic mode does not silently select an oversized model. The user can install a model through the existing explicit flow or manually override the choice.
- A manual model choice is retained as the preference on that device. If that model is no longer installed, UJANI does not silently replace it.
- Existing v1 preferences are migrated: a real manual choice is preserved, while the old fresh-install `manual`/empty placeholder becomes the automatic default. New manual/empty choices are preserved in v2.
- The mode and selected model identifier are stored locally in browser/WebView local storage. Prompts, files, API keys and task content are not stored by this preference.
- Automatic selection never downloads, installs, starts or switches a runtime. It only selects among models already reported as installed by the local Ollama service.

## Evidence and limitations

The current advisor uses an estimate based on model disk size and currently available RAM, with a safety reserve. It does not benchmark model quality, measure all runtime overhead, verify GPU/NPU compatibility or guarantee that inference will fit. The UI must describe it as advisory.

This is a first step toward the broader provider-neutral design. A typed provider registry, additional local runtimes, capability-aware cross-provider ranking, and per-task execution provenance remain separate implementation work. Ollama remains the first supported local model provider, not UJANI's permanent identity.
