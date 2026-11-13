import test from "node:test";
import assert from "node:assert/strict";
import { recommendDefaultModel } from "./modelRecommendation.js";

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
    assert.equal(recommendDefaultModel(total, available, os).model, null);
  }
});

test("4 GiB desktop and memory below the minimum receive no model", () => {
  assert.equal(recommendDefaultModel(4096, 3072, "Linux").model, null);
  assert.equal(recommendDefaultModel(6144, 2559, "Linux").model, null);
});

test("6 GiB and 2.5 GiB available reaches only the smallest candidate", () => {
  assert.equal(recommendDefaultModel(6144, 2560, "Linux").model, "qwen3:0.6b");
});

test("8 GiB and 3.5 GiB available recommends the 1.7B candidate", () => {
  assert.equal(recommendDefaultModel(8192, 3584, "Windows 11").model, "qwen3:1.7b");
});

test("16 GiB and 6 GiB available recommends the 4B candidate", () => {
  assert.equal(recommendDefaultModel(16384, 6144, "macOS").model, "qwen3:4b");
});

test("24 GiB and 10 GiB available recommends the 8B candidate", () => {
  assert.equal(recommendDefaultModel(24576, 10240, "Linux").model, "qwen3:8b");
});

test("mobile platforms never receive a desktop Ollama recommendation", () => {
  for (const os of ["Android 15", "iOS", "iPhone", "iPadOS"]) {
    assert.equal(recommendDefaultModel(65536, 32768, os).model, null);
  }
});

test("unknown desktop platforms never receive an unverified Ollama recommendation", () => {
  for (const os of ["FreeBSD", "ChromeOS", "Plan9", "Unknown OS"]) {
    assert.equal(recommendDefaultModel(32768, 16384, os).model, null);
  }
});

test("higher profile thresholds take precedence when all evidence is sufficient", () => {
  assert.equal(recommendDefaultModel(32768, 16384, "Linux").model, "qwen3:8b");
});

test("recommendation is advisory and does not expose a download action", () => {
  const result = recommendDefaultModel(8192, 4096, "Linux");
  assert.deepEqual(Object.keys(result).sort(), ["model", "reason", "title"]);
  assert.equal(result.model, "qwen3:1.7b");
});
