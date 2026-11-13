import assert from "node:assert/strict";
import test from "node:test";
import {
  LEGACY_MODEL_SELECTION_STORAGE_KEY,
  MODEL_SELECTION_STORAGE_KEY,
  readModelSelectionPreference,
} from "./modelSelectionPreference.js";

function memoryStorage(values = {}) {
  return {
    getItem(key) {
      return Object.hasOwn(values, key) ? values[key] : null;
    },
  };
}

test("fresh installations require an explicit installed-model or Automatic choice", () => {
  assert.deepEqual(readModelSelectionPreference(null), {
    mode: "manual",
    model: "",
  });
  assert.deepEqual(readModelSelectionPreference(), {
    mode: "manual",
    model: "",
  });
});

test("legacy manual-empty defaults remain unselected instead of becoming Automatic", () => {
  const storage = memoryStorage({
    [LEGACY_MODEL_SELECTION_STORAGE_KEY]: JSON.stringify({
      mode: "manual",
      model: "",
    }),
  });

  assert.deepEqual(readModelSelectionPreference(storage), {
    mode: "manual",
    model: "",
  });
});

test("an explicitly saved Automatic preference is preserved", () => {
  const storage = memoryStorage({
    [MODEL_SELECTION_STORAGE_KEY]: JSON.stringify({
      mode: "auto",
      model: "",
    }),
  });

  assert.deepEqual(readModelSelectionPreference(storage), {
    mode: "auto",
    model: "",
  });
});

test("a previously selected installed model is preserved", () => {
  const storage = memoryStorage({
    [MODEL_SELECTION_STORAGE_KEY]: JSON.stringify({
      mode: "manual",
      model: "qwen3:4b",
    }),
  });

  assert.deepEqual(readModelSelectionPreference(storage), {
    mode: "manual",
    model: "qwen3:4b",
  });
});

test("invalid current data falls back to a valid legacy preference", () => {
  const storage = memoryStorage({
    [MODEL_SELECTION_STORAGE_KEY]: JSON.stringify({
      mode: "automatic",
      model: "qwen3:4b",
    }),
    [LEGACY_MODEL_SELECTION_STORAGE_KEY]: JSON.stringify({
      mode: "auto",
      model: "",
    }),
  });

  assert.deepEqual(readModelSelectionPreference(storage), {
    mode: "auto",
    model: "",
  });
});

test("corrupt or oversized preferences fail closed to no selected model", () => {
  const storage = memoryStorage({
    [MODEL_SELECTION_STORAGE_KEY]: "{not-json",
    [LEGACY_MODEL_SELECTION_STORAGE_KEY]: JSON.stringify({
      mode: "manual",
      model: "x".repeat(129),
    }),
  });

  assert.deepEqual(readModelSelectionPreference(storage), {
    mode: "manual",
    model: "",
  });
});

test("storage access failures do not create an implicit model selection", () => {
  const storage = {
    getItem() {
      throw new Error("storage denied");
    },
  };

  assert.deepEqual(readModelSelectionPreference(storage), {
    mode: "manual",
    model: "",
  });
});
