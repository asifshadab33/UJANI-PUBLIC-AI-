import test from "node:test";
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
import { MODEL_RECOMMENDATION_MAX_SAMPLE_AGE_MS, recommendDefaultModel } from "./modelRecommendation.js";

const FRESH_SAMPLE_MS = 1_000_000;
const FRESH_NOW_MS = 1_000_000;
const recommend = (total, available, os) =>
  recommendDefaultModel(total, available, os, FRESH_SAMPLE_MS, FRESH_NOW_MS);

test("unknown, invalid or inconsistent memory evidence fails closed", () => {
  for (const [total, available, os] of [
    [undefined, 4096, "Linux"],
    [16384, undefined, "Linux"],
    [Number.NaN, 4096, "Linux"],
    [16384, Number.POSITIVE_INFINITY, "Linux"],
    [-1, 4096, "Linux"],
    [16384, 0, "Linux"],
    [8192, 9000, "Linux"],
    [8192, 4096, ""],
  ]) {
    assert.equal(recommend(total, available, os).model, null);
  }
});

test("4 GiB desktop and memory below the minimum receive no model", () => {
  assert.equal(recommend(4096, 3072, "Linux").model, null);
  assert.equal(recommend(6144, 2047, "Linux").model, null);
});

test("6 GiB and 2 GiB available reaches only the smallest candidate", () => {
  assert.equal(recommend(6144, 2048, "Linux").model, "qwen3:0.6b");
  assert.equal(recommend(6144, 2047, "Linux").model, null);
});

test("8 GiB and 4 GiB available recommends the 1.7B candidate", () => {
  assert.equal(recommend(8192, 4096, "Windows 11").model, "qwen3:1.7b");
  assert.equal(recommend(8192, 4095, "Windows 11").model, "qwen3:0.6b");
});

test("16 GiB and 8 GiB available recommends the 4B candidate", () => {
  assert.equal(recommend(16384, 8192, "macOS").model, "qwen3:4b");
  assert.equal(recommend(16384, 8191, "macOS").model, "qwen3:1.7b");
});

test("24 GiB and 16 GiB available recommends the 8B candidate", () => {
  assert.equal(recommend(24576, 16384, "Linux").model, "qwen3:8b");
  assert.equal(recommend(24576, 16383, "Linux").model, "qwen3:4b");
});

test("mobile platforms never receive a desktop Ollama recommendation", () => {
  for (const os of ["Android 15", "iOS", "iPhone", "iPadOS"]) {
    assert.equal(recommend(65536, 32768, os).model, null);
  }
});

test("unknown desktop platforms never receive an unverified Ollama recommendation", () => {
  for (const os of ["FreeBSD", "ChromeOS", "Plan9", "Unknown OS"]) {
    assert.equal(recommend(32768, 16384, os).model, null);
  }
});

test("higher profile thresholds take precedence when all evidence is sufficient", () => {
  assert.equal(recommend(32768, 16384, "Linux").model, "qwen3:8b");
});

test("recommendation is advisory and does not expose a download action", () => {
  const result = recommend(8192, 4096, "Linux");
  assert.deepEqual(Object.keys(result).sort(), ["model", "reason", "title"]);
  assert.equal(result.model, "qwen3:1.7b");
});
test("missing, stale or implausibly future hardware samples fail closed", () => {
  assert.equal(recommendDefaultModel(24576, 10240, "Linux", undefined, FRESH_NOW_MS).model, null);
  assert.equal(
    recommendDefaultModel(24576, 10240, "Linux", FRESH_SAMPLE_MS, FRESH_NOW_MS + MODEL_RECOMMENDATION_MAX_SAMPLE_AGE_MS + 1).model,
    null,
  );
  assert.equal(recommendDefaultModel(24576, 10240, "Linux", FRESH_NOW_MS + 30_001, FRESH_NOW_MS).model, null);
  assert.equal(
    recommendDefaultModel(24576, 16384, "Linux", FRESH_SAMPLE_MS, FRESH_NOW_MS + MODEL_RECOMMENDATION_MAX_SAMPLE_AGE_MS).model,
    "qwen3:8b",
  );
});


test("every automatic recommendation maps to a catalogue entry and meets its minimum RAM guide", () => {
  const catalogue = JSON.parse(readFileSync(new URL("./model-catalog.json", import.meta.url), "utf8"));
  const cases = [
    [6144, 2048, "Linux", "qwen3:0.6b"],
    [8192, 4096, "Linux", "qwen3:1.7b"],
    [16384, 8192, "Linux", "qwen3:4b"],
    [24576, 16384, "Linux", "qwen3:8b"],
  ];
  for (const [total, available, os, expected] of cases) {
    const recommendation = recommend(total, available, os);
    assert.equal(recommendation.model, expected);
    const entry = catalogue.models.find((model) => model.runtime_artifact === recommendation.model);
    assert.ok(entry, `recommended artifact ${recommendation.model} must exist in the model catalogue`);
    assert.ok(total / 1024 >= entry.planning_ram_gib.minimum);
    assert.ok(available / 1024 >= entry.planning_ram_gib.minimum);
    assert.equal(entry.download_requires_confirmation, true);
  }
});

test("catalogue policy keeps model installation optional and managed pulls disabled", () => {
  const catalogue = JSON.parse(readFileSync(new URL("./model-catalog.json", import.meta.url), "utf8"));
  assert.equal(catalogue.policy.optional_install, true);
  assert.equal(catalogue.policy.managed_downloads_enabled, false);
  assert.equal(catalogue.policy.silent_downloads_allowed, false);
  assert.equal(catalogue.policy.silent_model_switching_allowed, false);
  assert.equal(catalogue.policy.model_output_is_untrusted, true);
});
