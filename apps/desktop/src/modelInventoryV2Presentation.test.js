import assert from "node:assert/strict";
import test from "node:test";
import {
  evidenceSummary,
  capabilityStatusLabel,
  booleanCapabilityLabel,
  formatOptionalNumber,
  formatOptionalBytes,
} from "./modelInventoryV2Presentation.js";

test("missing and unknown evidence is shown as unknown instead of inferred", () => {
  assert.equal(evidenceSummary(null), "Unknown (no evidence)");
  assert.equal(evidenceSummary({ source: "unknown", confidence: "unknown", observed_at_unix_ms: null }), "Unknown (no evidence)");
});

test("reported and estimated provenance stays explicit", () => {
  assert.match(evidenceSummary({ source: "runtime_metadata", confidence: "reported", observed_at_unix_ms: null }), /runtime_metadata · reported/);
  assert.match(evidenceSummary({ source: "estimate", confidence: "estimated", observed_at_unix_ms: null }), /estimate · estimated/);
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
