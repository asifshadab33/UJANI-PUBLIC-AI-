export type ModelSelectionMode = "auto" | "manual";
export type ModelSelectionPreference = {
  mode: ModelSelectionMode;
  model: string;
};

export const MODEL_SELECTION_STORAGE_KEY: string;
export const LEGACY_MODEL_SELECTION_STORAGE_KEY: string;

export function readModelSelectionPreference(
  storage?: { getItem(key: string): string | null } | null
): ModelSelectionPreference;
