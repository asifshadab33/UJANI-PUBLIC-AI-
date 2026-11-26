import assert from "node:assert/strict";
import test from "node:test";
import { formatReminderDue } from "./reminderDateFormat.js";

test("formats a reminder in its saved IANA timezone", () => {
  assert.match(formatReminderDue(Date.UTC(2026, 0, 1, 12), "Asia/Kolkata", "en-GB"), /17:30/);
});

test("does not silently substitute the host timezone for an unavailable saved zone", () => {
  const value = formatReminderDue(Date.UTC(2026, 0, 1, 12), "Invalid/Zone", "en-GB");
  assert.ok(value.startsWith("2026-01-01T12:00:00.000Z"));
  assert.match(value, /UTC; saved timezone unavailable/);
});

test("rejects non-finite and out-of-range timestamps without throwing", () => {
  assert.equal(formatReminderDue(Number.NaN, "UTC"), "Invalid reminder timestamp");
  assert.equal(formatReminderDue(Number.MAX_VALUE, "UTC"), "Invalid reminder timestamp");
});
