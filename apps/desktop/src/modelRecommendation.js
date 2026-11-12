/**
 * Conservative desktop model onboarding advice.
 * This function is pure: it never installs, downloads, selects or runs a model.
 */
export function recommendDefaultModel(totalMemoryMb, availableMemoryMb, os) {
  const noRecommendation = {
    model: null,
    title: "No model download recommended yet",
    reason: "UJANI cannot verify this device's operating system and available memory. You can still use UJANI's built-in features and choose a model later.",
  };

  if (
    !Number.isFinite(totalMemoryMb) ||
    !Number.isFinite(availableMemoryMb) ||
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
