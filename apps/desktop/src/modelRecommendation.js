/**
 * Conservative desktop model onboarding advice.
 * This function is pure: it never installs, downloads, selects or runs a model.
 */
export const MODEL_RECOMMENDATION_MAX_SAMPLE_AGE_MS = 5 * 60 * 1000;
const MAX_FUTURE_SAMPLE_SKEW_MS = 30 * 1000;

export function recommendDefaultModel(totalMemoryMb, availableMemoryMb, os, sampledAtUnixMs, nowUnixMs) {
  const noRecommendation = {
    model: null,
    title: "No model download recommended yet",
    reason: "UJANI needs a fresh hardware sample (no older than five minutes) before recommending a model. Refresh local model advice; you can still choose a model manually.",
  };

  if (
    !Number.isFinite(totalMemoryMb) ||
    !Number.isFinite(availableMemoryMb) ||
    !Number.isFinite(sampledAtUnixMs) ||
    !Number.isFinite(nowUnixMs) ||
    sampledAtUnixMs <= 0 ||
    sampledAtUnixMs > nowUnixMs + MAX_FUTURE_SAMPLE_SKEW_MS ||
    nowUnixMs - sampledAtUnixMs > MODEL_RECOMMENDATION_MAX_SAMPLE_AGE_MS ||
    totalMemoryMb <= 0 ||
    availableMemoryMb <= 0 ||
    availableMemoryMb > totalMemoryMb ||
    typeof os !== "string" ||
    os.trim().length === 0
  ) {
    return noRecommendation;
  }

  const platform = os.toLowerCase();
  if (["android", "ios", "iphone", "ipad"].some((name) => platform.includes(name))) {
    return {
      model: null,
      title: "No desktop model runtime recommendation",
      reason: "Ollama desktop support is not assumed on mobile. UJANI will offer a mobile model only after that exact runtime and device class have passed validation.",
    };
  }

  if (!["windows", "linux", "mac", "darwin"].some((name) => platform.includes(name))) {
    return {
      model: null,
      title: "No verified desktop model runtime recommendation",
      reason: "UJANI does not assume Ollama support on this operating system. Use built-in UJANI features and choose a model only after the exact runtime and platform have been validated.",
    };
  }

  if (totalMemoryMb < 6144) {
    return {
      model: null,
      title: "Recommended default: no local model",
      reason: "On a 4 GB computer, keep memory available for the operating system and UJANI. Native OS-powered features remain available without downloading a model.",
    };
  }
  if (totalMemoryMb >= 24576 && availableMemoryMb >= 10240) {
    return {
      model: "qwen3:8b",
      title: "Recommended candidate: Qwen3 8B",
      reason: "This device reports at least 24 GiB total RAM and 10 GiB available RAM. Start with a 4K context and one inference at a time.",
    };
  }
  if (totalMemoryMb >= 16384 && availableMemoryMb >= 6144) {
    return {
      model: "qwen3:4b",
      title: "Recommended candidate: Qwen3 4B",
      reason: "This device reports at least 16 GiB total RAM and 6 GiB available RAM. Start with a 4K context and one inference at a time.",
    };
  }
  if (totalMemoryMb >= 8192 && availableMemoryMb >= 3584) {
    return {
      model: "qwen3:1.7b",
      title: "Recommended candidate: Qwen3 1.7B",
      reason: "This device reports at least 8 GiB total RAM and 3.5 GiB available RAM. Start with a 4K context and one inference at a time.",
    };
  }
  if (totalMemoryMb >= 6144 && availableMemoryMb >= 2560) {
    return {
      model: "qwen3:0.6b",
      title: "Recommended candidate: Qwen3 0.6B",
      reason: "This device reports at least 6 GiB total RAM and 2.5 GiB available RAM. This is the lightest general-purpose chat candidate; start with a 4K context.",
    };
  }

  return {
    model: null,
    title: "Recommended default: no local model",
    reason: "Available memory is below UJANI's conservative starting threshold. Keep model installation optional and use native OS-powered features.",
  };
}
