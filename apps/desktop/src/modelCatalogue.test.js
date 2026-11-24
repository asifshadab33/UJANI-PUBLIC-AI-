import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const catalogue = JSON.parse(
  readFileSync(new URL("./model-catalog.json", import.meta.url), "utf8"),
);

test("model catalogue keeps downloads and redistribution approval disabled", () => {
  assert.equal(catalogue.policy.managed_downloads_enabled, false);
  assert.equal(catalogue.policy.silent_downloads_allowed, false);
  assert.equal(catalogue.policy.silent_model_switching_allowed, false);
  assert.equal(catalogue.provenance_policy.exact_runtime_artifact_verified, false);
  assert.equal(catalogue.provenance_policy.redistribution_approved, false);
});

test("every candidate links publisher and licence evidence without claiming artifact verification", () => {
  assert.ok(catalogue.models.length > 0);
  const ids = new Set();
  for (const model of catalogue.models) {
    assert.ok(model.id && !ids.has(model.id), `duplicate or empty model id: ${model.id}`);
    ids.add(model.id);
    const evidence = model.provenance;
    assert.ok(evidence, `${model.id} is missing provenance`);
    assert.match(evidence.publisher_model_card_url, /^https:\/\//);
    assert.match(evidence.license_source_url, /^https:\/\//);
    assert.ok(evidence.license_name);
    assert.equal(evidence.publisher_metadata_checked_at, "2026-11-24");
    assert.equal(evidence.exact_runtime_artifact_verified, false);
    assert.equal(evidence.artifact_identity_status, "not_pinned");
    assert.ok(evidence.artifact_note.includes("exact Ollama registry manifest"));
    assert.ok(["spdx", "custom_terms"].includes(evidence.license_type));
    if (evidence.license_type === "spdx") {
      assert.ok(evidence.license_identifier);
    } else {
      assert.equal(evidence.license_identifier, null);
    }
  }
});

test("catalogue licence classifications preserve custom model terms", () => {
  const byId = new Map(catalogue.models.map((model) => [model.id, model.provenance]));
  assert.equal(byId.get("qwen3-0.6b").license_identifier, "Apache-2.0");
  assert.equal(byId.get("smollm2-1.7b").license_identifier, "Apache-2.0");
  assert.equal(byId.get("phi-4-mini").license_identifier, "MIT");
  assert.equal(byId.get("llama-3.2-1b").license_type, "custom_terms");
  assert.equal(byId.get("llama-3.2-3b").license_type, "custom_terms");
  assert.equal(byId.get("gemma-3-1b").license_type, "custom_terms");
  assert.equal(byId.get("gemma-3-1b").license_source_url, "https://gemma.ai/resources/terms/");
});
