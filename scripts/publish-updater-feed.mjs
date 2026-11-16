import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const VERSION_PATTERN = /^(0|[1-9]\\d*)\\.(0|[1-9]\\d*)\\.(0|[1-9]\\d*)(?:-([0-9A-Za-z-]+(?:\\.[0-9A-Za-z-]+)*))?(?:\\+[0-9A-Za-z-]+(?:\\.[0-9A-Za-z-]+)*)?$/;
const NUMERIC_IDENTIFIER = /^(0|[1-9]\\d*)$/;

export function parseSemver(value) {
  if (typeof value !== "string") throw new TypeError("Version must be a string.");
  const match = VERSION_PATTERN.exec(value);
  if (!match) throw new Error("Unsupported semantic version: " + value);
  const prerelease = match[4] ? match[4].split(".") : [];
  for (const identifier of prerelease) {
    if (/^\\d+$/.test(identifier) && !NUMERIC_IDENTIFIER.test(identifier)) {
      throw new Error("Numeric prerelease identifiers must not contain leading zeroes.");
    }
  }
  return { core: match.slice(1, 4).map(BigInt), prerelease };
}

export function compareSemver(left, right) {
  const a = typeof left === "string" ? parseSemver(left) : left;
  const b = typeof right === "string" ? parseSemver(right) : right;
  for (let index = 0; index < 3; index += 1) {
    if (a.core[index] !== b.core[index]) return a.core[index] < b.core[index] ? -1 : 1;
  }
  if (!a.prerelease.length && !b.prerelease.length) return 0;
  if (!a.prerelease.length) return 1;
  if (!b.prerelease.length) return -1;
  for (let index = 0; index < Math.max(a.prerelease.length, b.prerelease.length); index += 1) {
    const av = a.prerelease[index];
    const bv = b.prerelease[index];
    if (av === undefined) return -1;
    if (bv === undefined) return 1;
    if (av === bv) continue;
    const aNumeric = NUMERIC_IDENTIFIER.test(av);
    const bNumeric = NUMERIC_IDENTIFIER.test(bv);
    if (aNumeric && bNumeric) return BigInt(av) < BigInt(bv) ? -1 : 1;
    if (aNumeric !== bNumeric) return aNumeric ? -1 : 1;
    return av < bv ? -1 : 1;
  }
  return 0;
}

export function validateManifest(manifest) {
  if (!manifest || typeof manifest !== "object" || Array.isArray(manifest)) {
    throw new Error("Updater manifest must be a JSON object.");
  }
  parseSemver(manifest.version);
  if (!manifest.platforms || typeof manifest.platforms !== "object" || Array.isArray(manifest.platforms)) {
    throw new Error("Updater manifest must contain a platforms object.");
  }
  const platforms = Object.entries(manifest.platforms);
  if (!platforms.length) throw new Error("Updater manifest contains no platform artifacts.");
  for (const [target, artifact] of platforms) {
    if (!target || !artifact || typeof artifact !== "object" ||
        typeof artifact.url !== "string" || typeof artifact.signature !== "string" ||
        !artifact.signature.trim()) {
      throw new Error("Updater artifact is missing its URL or signature: " + target);
    }
    let url;
    try { url = new URL(artifact.url); } catch { throw new Error("Invalid updater artifact URL: " + target); }
    if (url.protocol !== "https:") throw new Error("Updater artifact URL must use HTTPS: " + target);
  }
  return manifest;
}

/** Atomically replace the feed only when the incoming signed manifest is newer. */
export function publishManifest(incomingPath, outputPath) {
  const incoming = validateManifest(JSON.parse(fs.readFileSync(incomingPath, "utf8")));
  if (fs.existsSync(outputPath)) {
    const current = validateManifest(JSON.parse(fs.readFileSync(outputPath, "utf8")));
    if (compareSemver(incoming.version, current.version) <= 0) return false;
  }
  const temporaryPath = outputPath + ".tmp";
  fs.copyFileSync(incomingPath, temporaryPath);
  fs.renameSync(temporaryPath, outputPath);
  return true;
}

function main() {
  const [incomingPath, outputPath] = process.argv.slice(2);
  if (!incomingPath || !outputPath || process.argv.length !== 4) {
    throw new Error("Usage: node publish-updater-feed.mjs <incoming-manifest> <output-manifest>");
  }
  const published = publishManifest(path.resolve(incomingPath), path.resolve(outputPath));
  process.stdout.write(published ? "publish\\n" : "skip\\n");
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  try { main(); }
  catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
