export const MODEL_SELECTION_STORAGE_KEY = "ujani.public.model-selection.v2";
export const LEGACY_MODEL_SELECTION_STORAGE_KEY = "ujani.public.model-selection.v1";

const DEFAULT_PREFERENCE = Object.freeze({ mode: "manual", model: "" });

function browserStorage() {
  try {
    return typeof window === "undefined" ? null : window.localStorage;
  } catch {
    return null;
  }
}

function parsePreference(storage, key) {
  try {
    const raw = storage.getItem(key);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

function isValidPreference(value) {
  return Boolean(
    value
      && typeof value === "object"
      && !Array.isArray(value)
      && (value.mode === "auto" || value.mode === "manual")
      && typeof value.model === "string"
      && value.model.length <= 128
  );
}

/**
 * Read only the user's local model-selection preference.
 * A fresh installation deliberately requires a model choice.
 *
 * @param {{ getItem: (key: string) => string | null } | null} [storage]
 * @returns {{ mode: "auto" | "manual", model: string }}
 */
export function readModelSelectionPreference(storage = browserStorage()) {
  if (!storage) return { ...DEFAULT_PREFERENCE };

  const current = parsePreference(storage, MODEL_SELECTION_STORAGE_KEY);
  if (isValidPreference(current)) {
    return { mode: current.mode, model: current.model };
  }

  const legacy = parsePreference(storage, LEGACY_MODEL_SELECTION_STORAGE_KEY);
  if (isValidPreference(legacy)) {
    // Preserve an explicit legacy Automatic choice, but do not convert the old
    // manual/empty fresh-install default into an implicit model-selection policy.
    return { mode: legacy.mode, model: legacy.model };
  }

  return { ...DEFAULT_PREFERENCE };
}
