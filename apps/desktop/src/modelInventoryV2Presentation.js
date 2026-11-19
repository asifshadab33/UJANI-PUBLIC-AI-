/** Presentation helpers for evidence-qualified model/runtime inventory metadata. */
export function evidenceSummary(evidence) {
  if (!evidence || evidence.source === "unknown" || evidence.confidence === "unknown") {
    return "Unknown (no evidence)";
  }
  const timestamp = evidence.observed_at_unix_ms;
  const observed = Number.isFinite(timestamp) && timestamp > 0 && timestamp <= 8.64e15
    ? ` · observed ${new Date(timestamp).toLocaleString()}`
    : "";
  return `${evidence.source} · ${evidence.confidence}${observed}`;
}

export function capabilityStatusLabel(status) {
  if (status === "supported") return "Supported";
  if (status === "unsupported") return "Unsupported";
  return "Unknown";
}

export function booleanCapabilityLabel(value) {
  if (value === true) return "Supported";
  if (value === false) return "Unsupported";
  return "Unknown";
}

export function formatOptionalNumber(value, unit) {
  if (value === null || value === undefined || !Number.isFinite(value)) return "Unknown";
  return `${value.toLocaleString()} ${unit}`;
}

export function formatOptionalBytes(value) {
  if (value === null || value === undefined || !Number.isFinite(value) || value < 0) return "Unknown";
  if (value === 0) return "0 bytes";
  if (value < 1024) return `${value} bytes`;
  const kib = value / 1024;
  if (kib < 1024) return `${kib.toLocaleString(undefined, { maximumFractionDigits: 1 })} KiB`;
  const mib = kib / 1024;
  if (mib < 1024) return `${mib.toLocaleString(undefined, { maximumFractionDigits: 1 })} MiB`;
  const gib = mib / 1024;
  return `${gib.toLocaleString(undefined, { maximumFractionDigits: 2 })} GiB`;
}
