/** Type declarations for evidence-qualified model/runtime inventory presentation helpers. */

export type EvidenceSummaryInput = {
  source?: string | null;
  confidence?: string | null;
  observed_at_unix_ms?: number | null;
} | null | undefined;

export function evidenceSummary(evidence: EvidenceSummaryInput): string;
export function capabilityStatusLabel(status: string | null | undefined): "Supported" | "Unsupported" | "Unknown";
export function booleanCapabilityLabel(value: boolean | null | undefined): "Supported" | "Unsupported" | "Unknown";
export function formatOptionalNumber(value: number | null | undefined, unit: string): string;
export function formatOptionalBytes(value: number | null | undefined): string;
