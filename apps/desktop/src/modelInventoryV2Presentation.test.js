import assert from "node:assert/strict";
import test from "node:test";
import {
  evidenceSummary,
  capabilityStatusLabel,
  booleanCapabilityLabel,
  formatOptionalNumber,
  formatOptionalBytes,
  formatObservedTimestamp,
} from "./modelInventoryV2Presentation.js";

test("missing and unknown evidence is shown as unknown instead of inferred", () => {
  assert.equal(evidenceSummary(null), "Unknown (no evidence)");
  assert.equal(evidenceSummary({ source: "unknown", confidence: "unknown", observed_at_unix_ms: null }), "Unknown (no evidence)");
});

test("reported and estimated provenance stays explicit", () => {
  assert.match(evidenceSummary({ source: "runtime_metadata", confidence: "reported", observed_at_unix_ms: null }), /runtime_metadata · reported/);
  assert.match(evidenceSummary({ source: "estimate", confidence: "estimated", observed_at_unix_ms: null }), /estimate · estimated/);
});

test("out-of-range timestamps do not render an invalid date", () => {
  assert.doesNotMatch(evidenceSummary({ source: "runtime_api", confidence: "reported", observed_at_unix_ms: 9e15 }), /Invalid Date/);
});

test("capability and nullable booleans preserve unknown", () => {
  assert.equal(capabilityStatusLabel("supported"), "Supported");
  assert.equal(capabilityStatusLabel("unsupported"), "Unsupported");
  assert.equal(capabilityStatusLabel("unknown"), "Unknown");
  assert.equal(booleanCapabilityLabel(true), "Supported");
  assert.equal(booleanCapabilityLabel(false), "Unsupported");
  assert.equal(booleanCapabilityLabel(null), "Unknown");
});

test("unknown resource values are not rendered as zero", () => {
  assert.equal(formatOptionalNumber(null, "MiB"), "Unknown");
  assert.equal(formatOptionalNumber(undefined, "tokens"), "Unknown");
  assert.equal(formatOptionalNumber(0, "bytes"), "0 bytes");
});

test("byte estimates are bounded to readable units and unknown stays explicit", () => {
  assert.equal(formatOptionalBytes(null), "Unknown");
  assert.equal(formatOptionalBytes(0), "0 bytes");
  assert.equal(formatOptionalBytes(1024), "1 KiB");
  assert.match(formatOptionalBytes(1024 * 1024), /1 MiB/);
  assert.match(formatOptionalBytes(1024 * 1024 * 1024), /1 GiB/);
});

test("observed timestamp presentation is compact and safely omits invalid values", () => {
  const observedAt = Date.UTC(2026, 0, 2, 3, 4);
  const expected = new Date(observedAt).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
  const rendered = formatObservedTimestamp(observedAt);
  assert.equal(rendered, ` · observed ${expected}`);
  assert.doesNotMatch(rendered, /Invalid Date/);
  assert.equal(formatObservedTimestamp(null), "");
  assert.equal(formatObservedTimestamp(Number.NaN), "");
  assert.equal(formatObservedTimestamp(-1), "");
  assert.equal(formatObservedTimestamp(9e15), "");
});
