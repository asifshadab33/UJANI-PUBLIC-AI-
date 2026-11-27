import assert from "node:assert/strict";
import test from "node:test";
import { subscribeWithCleanup } from "./asyncEventSubscription.js";

test("unlistens when registration resolves after effect cleanup", async () => {
  let resolveRegistration;
  let unlistenCalls = 0;
  const registration = new Promise((resolve) => { resolveRegistration = resolve; });
  const dispose = subscribeWithCleanup(registration);

  dispose();
  resolveRegistration(() => { unlistenCalls += 1; });
  await registration;
  await Promise.resolve();

  assert.equal(unlistenCalls, 1);
  dispose();
  assert.equal(unlistenCalls, 1);
});

test("unlistens once when cleanup follows successful registration", async () => {
  let unlistenCalls = 0;
  const dispose = subscribeWithCleanup(Promise.resolve(() => { unlistenCalls += 1; }));
  await Promise.resolve();

  dispose();
  dispose();

  assert.equal(unlistenCalls, 1);
});

test("does not report a late registration error after cleanup", async () => {
  let rejectRegistration;
  const errors = [];
  const registration = new Promise((_, reject) => { rejectRegistration = reject; });
  const dispose = subscribeWithCleanup(registration, (error) => errors.push(error));

  dispose();
  rejectRegistration(new Error("registration failed after cleanup"));
  await Promise.resolve();
  await Promise.resolve();

  assert.equal(errors.length, 0);
});
