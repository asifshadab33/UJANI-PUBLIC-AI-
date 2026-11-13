import test from "node:test";
import assert from "node:assert/strict";
import { MODEL_RECOMMENDATION_MAX_SAMPLE_AGE_MS, recommendDefaultModel } from "./modelRecommendation.js";

const FRESH_SAMPLE_MS = 1_000_000;
const FRESH_NOW_MS = 1_000_000;
const recommend = (total, available, os) =>
  recommend(total, available, os, FRESH_SAMPLE_MS, FRESH_NOW_MS);

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
  assert.equal(recommend(6144, 2559, "Linux").model, null);
});

test("6 GiB and 2.5 GiB available reaches only the smallest candidate", () => {
  assert.equal(recommend(6144, 2560, "Linux").model, "qwen3:0.6b");
});

test("8 GiB and 3.5 GiB available recommends the 1.7B candidate", () => {
  assert.equal(recommend(8192, 3584, "Windows 11").model, "qwen3:1.7b");
});

test("16 GiB and 6 GiB available recommends the 4B candidate", () => {
  assert.equal(recommend(16384, 6144, "macOS").model, "qwen3:4b");
});

test("24 GiB and 10 GiB available recommends the 8B candidate", () => {
  assert.equal(recommend(24576, 10240, "Linux").model, "qwen3:8b");
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
    recommendDefaultModel(24576, 10240, "Linux", FRESH_SAMPLE_MS, FRESH_NOW_MS + MODEL_RECOMMENDATION_MAX_SAMPLE_AGE_MS).model,
    "qwen3:8b",
  );
});
