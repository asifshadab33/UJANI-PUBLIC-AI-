import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { compareSemver, parseSemver, publishManifest, validateManifest } from "./publish-updater-feed.mjs";

const manifest = (version) => ({
  version,
  platforms: {
    "linux-x86_64": { url: "https://github.com/example/release/download/app.AppImage", signature: "signed-artifact" },
  },
});

test("orders stable releases after prereleases and compares prerelease identifiers correctly", () => {
  assert.equal(compareSemver("1.0.0", "1.0.0-rc.9"), 1);
  assert.equal(compareSemver("1.0.0-rc.10", "1.0.0-rc.9"), 1);
  assert.equal(compareSemver("1.0.0-rc.2", "1.0.0-rc.10"), -1);
  assert.equal(compareSemver("1.2.0", "1.10.0"), -1);
  assert.equal(compareSemver("2.0.0+build.2", "2.0.0+build.1"), 0);
});

test("rejects invalid versions and malformed updater artifacts", () => {
  assert.throws(() => parseSemver("01.0.0"));
  assert.throws(() => parseSemver("1.0.0-01"));
  assert.throws(() => validateManifest({ version: "1.0.0", platforms: {} }));
  assert.throws(() => validateManifest({ version: "1.0.0", platforms: { linux: { url: "http://example.test/a", signature: "sig" } } }));
  assert.throws(() => validateManifest({ version: "1.0.0", platforms: { linux: { url: "https://example.test/a", signature: "" } } }));
});

test("binds every artifact URL and the manifest version to the approved release", () => {
  const approved = {
    version: "0.1.0",
    platforms: {
      "linux-x86_64": {
        url: "https://github.com/asifshadab33/UJANI-PUBLIC-AI-/releases/download/v0.1.0/ujani.AppImage.tar.gz",
        signature: "signed-artifact",
      },
    },
  };
  assert.doesNotThrow(() => validateManifest(approved, {
    version: "0.1.0",
    repository: "asifshadab33/UJANI-PUBLIC-AI-",
    tag: "v0.1.0",
  }));
  assert.throws(() => validateManifest(approved, {
    version: "0.1.1",
    repository: "asifshadab33/UJANI-PUBLIC-AI-",
    tag: "v0.1.0",
  }));
  assert.throws(() => validateManifest(approved, {
    version: "0.1.0",
    repository: "attacker/other-repo",
    tag: "v0.1.0",
  }));
  assert.throws(() => validateManifest(approved, {
    version: "0.1.0",
    repository: "asifshadab33/UJANI-PUBLIC-AI-",
    tag: "v0.1.1",
  }));
});

test("publishes first valid feed and atomically replaces it only with a newer version", (t) => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "ujani-updater-feed-"));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const incoming = path.join(directory, "incoming.json");
  const output = path.join(directory, "latest.json");

  fs.writeFileSync(incoming, JSON.stringify(manifest("0.1.0-rc.1")));
  assert.equal(publishManifest(incoming, output), true);
  assert.equal(JSON.parse(fs.readFileSync(output, "utf8")).version, "0.1.0-rc.1");

  fs.writeFileSync(incoming, JSON.stringify(manifest("0.1.0-rc.1")));
  assert.equal(publishManifest(incoming, output), false);
  assert.equal(JSON.parse(fs.readFileSync(output, "utf8")).version, "0.1.0-rc.1");

  fs.writeFileSync(incoming, JSON.stringify(manifest("0.1.0")));
  assert.equal(publishManifest(incoming, output), true);
  assert.equal(JSON.parse(fs.readFileSync(output, "utf8")).version, "0.1.0");
});

test("fails closed when the existing feed is malformed", (t) => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "ujani-updater-feed-invalid-"));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const incoming = path.join(directory, "incoming.json");
  const output = path.join(directory, "latest.json");
  fs.writeFileSync(incoming, JSON.stringify(manifest("0.2.0")));
  fs.writeFileSync(output, "{broken");
  assert.throws(() => publishManifest(incoming, output));
  assert.equal(fs.readFileSync(output, "utf8"), "{broken");
});
