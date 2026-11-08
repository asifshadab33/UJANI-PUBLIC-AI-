import { execFileSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const desktopRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const manifest = path.join(desktopRoot, 'src-tauri', 'Cargo.toml');
const manifestRoot = path.dirname(manifest);
const args = process.argv.slice(2);
let explicitTarget = null;
let debugBuild = false;
for (let index = 0; index < args.length; index += 1) {
  if (args[index] === '--target' && args[index + 1]) explicitTarget = args[++index];
  if (args[index] === '--debug') debugBuild = true;
}
const target = explicitTarget || process.env.TAURI_ENV_TARGET_TRIPLE || execFileSync('rustc', ['--print', 'host-tuple'], { encoding: 'utf8' }).trim();
if (!target || !/^[A-Za-z0-9_.-]+$/.test(target)) throw new Error('Could not determine a safe Rust target triple.');
const profile = debugBuild ? 'debug' : 'release';
const extension = process.platform === 'win32' ? '.exe' : '';
const targetDir = process.env.CARGO_TARGET_DIR || path.join(manifestRoot, 'target');
const destinationDir = path.join(manifestRoot, 'binaries');
const destination = path.join(destinationDir, 'ujani-pdf-worker-' + target + extension);
fs.mkdirSync(destinationDir, { recursive: true });
// Skip only Tauri's app-context build step while compiling the isolated worker.
// The normal app build later runs build.rs and validates the real staged sidecar.
const cargoArgs = ['build', '--manifest-path', manifest, '--bin', 'ujani-pdf-worker', '--locked'];
if (!debugBuild) cargoArgs.push('--release');
if (explicitTarget || process.env.TAURI_ENV_TARGET_TRIPLE) cargoArgs.push('--target', target);
const result = spawnSync('cargo', cargoArgs, { stdio: 'inherit', env: { ...process.env, UJANI_PDF_WORKER_BUILD: '1' } });
if (result.status !== 0) process.exit(result.status ?? 1);
const built = path.join(targetDir, ...(explicitTarget || process.env.TAURI_ENV_TARGET_TRIPLE ? [target] : []), profile, 'ujani-pdf-worker' + extension);
if (!fs.existsSync(built)) throw new Error('PDF worker build output was not found: ' + built);
fs.copyFileSync(built, destination);
if (process.platform !== 'win32') {
  fs.chmodSync(destination, 0o755);
  fs.chmodSync(built, 0o755);
}
// Cargo already places the unbundled worker beside the desktop executable in
// target/<triple>/<profile>; the target-specific copy above is for Tauri bundling.
console.log('Staged dedicated PDF worker for ' + target + ': ' + destination);
