export type DefaultModelRecommendation = {
  model: string | null;
  title: string;
  reason: string;
};

/**
 * Returns advisory model onboarding guidance only.
 * It never downloads, installs, selects or starts a model.
 */
export function recommendDefaultModel(
  totalMemoryMb: number | undefined,
  availableMemoryMb: number | undefined,
  os: string | undefined,
  sampledAtUnixMs: number | undefined,
  nowUnixMs: number,
): DefaultModelRecommendation;
