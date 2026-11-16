import test from "node:test";
import assert from "node:assert/strict";
import { CLOCK_JUMP_THRESHOLD_MS, hasWallClockJump } from "./clockMonitor.js";

test("does not report normal elapsed time or sub-threshold scheduling jitter", () => {
  assert.equal(hasWallClockJump(10_000, 11_000, 500, 1_001), false);
  assert.equal(hasWallClockJump(10_000, 16_000, 500, 1_500), false);
});

test("detects forward and backward wall-clock steps", () => {
  assert.equal(hasWallClockJump(10_000, 30_001, 500, 1_500), true);
  assert.equal(hasWallClockJump(30_001, 10_000, 1_500, 2_500), true);
});

test("uses a strict five-second threshold", () => {
  assert.equal(CLOCK_JUMP_THRESHOLD_MS, 5_000);
  assert.equal(hasWallClockJump(0, 6_000, 0, 1_000), false);
  assert.equal(hasWallClockJump(0, 6_001, 0, 1_000), true);
});

test("treats resume-time wall/monotonic divergence as a diagnostic jump", () => {
  assert.equal(hasWallClockJump(1_000, 61_000, 100, 101), true);
});

test("ignores invalid measurements rather than producing a false warning", () => {
  assert.equal(hasWallClockJump(Number.NaN, 1, 1, 2), false);
  assert.equal(hasWallClockJump(1, 2, 1, 2, -1), false);
});
