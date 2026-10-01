import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

export function nextVersion(version, bump) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version))
    throw new Error('Expected a stable semantic version.');
  const parts = version.split('.').map(Number);
  const index = ['major', 'minor', 'patch'].indexOf(bump);
  if (index < 0) throw new Error('Use patch, minor, or major.');
  parts[index]++;
  for (let i = index + 1; i < parts.length; i++) parts[i] = 0;
  return parts.join('.');
}

export function versions() {
  const pkg = JSON.parse(readFileSync('package.json', 'utf8')).version;
  const tauri = JSON.parse(
    readFileSync('src-tauri/tauri.conf.json', 'utf8'),
  ).version;
  const cargo = readFileSync('src-tauri/Cargo.toml', 'utf8').match(
    /^version = "([^"]+)"/m,
  )?.[1];
  const lock = readFileSync('src-tauri/Cargo.lock', 'utf8').match(
    /name = "resonance"\r?\nversion = "([^"]+)"/,
  )?.[1];
  if (![tauri, cargo, lock].every((version) => version === pkg))
    throw new Error('Package, Tauri, Cargo and lockfile versions must match.');
  return pkg;
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const version = versions();
  const tag = process.argv[2];
  if (tag && tag !== `v${version}`) throw new Error(`Tag must be v${version}.`);
  console.log(`Version ${version} is consistent.`);
}
