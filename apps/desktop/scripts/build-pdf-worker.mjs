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
const cargoArgs = ['build', '--manifest-path', manifest, '--bin', 'ujani-pdf-worker', '--locked'];
if (!debugBuild) cargoArgs.push('--release');
if (explicitTarget || process.env.TAURI_ENV_TARGET_TRIPLE) cargoArgs.push('--target', target);
const result = spawnSync('cargo', cargoArgs, { stdio: 'inherit' });
if (result.status !== 0) process.exit(result.status ?? 1);
const extension = process.platform === 'win32' ? '.exe' : '';
const targetDir = process.env.CARGO_TARGET_DIR || path.join(manifestRoot, 'target');
const built = path.join(targetDir, ...(explicitTarget || process.env.TAURI_ENV_TARGET_TRIPLE ? [target] : []), profile, 'ujani-pdf-worker' + extension);
const destinationDir = path.join(manifestRoot, 'binaries');
const destination = path.join(destinationDir, 'ujani-pdf-worker-' + target + extension);
if (!fs.existsSync(built)) throw new Error('PDF worker build output was not found: ' + built);
fs.mkdirSync(destinationDir, { recursive: true });
fs.copyFileSync(built, destination);
if (process.platform !== 'win32') fs.chmodSync(destination, 0o755);
// Also place the worker beside the unbundled Tauri executable for `tauri dev`
// and direct target/debug or target/release launches. The app never searches PATH.
const appBinaryDir = path.join(targetDir, ...(explicitTarget || process.env.TAURI_ENV_TARGET_TRIPLE ? [target] : []), profile);
const adjacent = path.join(appBinaryDir, 'ujani-pdf-worker' + extension);
fs.copyFileSync(built, adjacent);
if (process.platform !== 'win32') fs.chmodSync(adjacent, 0o755);
console.log('Staged dedicated PDF worker for ' + target + ': ' + destination + ' and ' + adjacent);
