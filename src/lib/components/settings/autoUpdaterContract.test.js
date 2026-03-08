// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const cargoToml = readFileSync(new URL('../../../../src-tauri/Cargo.toml', import.meta.url), 'utf8');
const tauriConfig = JSON.parse(
  readFileSync(new URL('../../../../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
);
const libRs = readFileSync(new URL('../../../../src-tauri/src/lib.rs', import.meta.url), 'utf8');
const defaultCapability = JSON.parse(
  readFileSync(new URL('../../../../src-tauri/capabilities/default.json', import.meta.url), 'utf8'),
);
const releaseWorkflow = readFileSync(
  new URL('../../../../.github/workflows/release.yml', import.meta.url),
  'utf8',
);

const permissions = defaultCapability.permissions ?? [];

test('rust crate version is aligned to 0.3.0', () => {
  assert.match(cargoToml, /^version = "0\.3\.0"$/m);
});

test('tauri app version is aligned to 0.3.0', () => {
  assert.equal(tauriConfig.version, '0.3.0');
});

test('rust dependencies include updater and process plugins', () => {
  assert.match(cargoToml, /^tauri-plugin-updater = /m);
  assert.match(cargoToml, /^tauri-plugin-process = /m);
});

test('tauri builder registers updater and process plugins', () => {
  assert.match(libRs, /tauri_plugin_process::init\(\)/);
  assert.match(libRs, /tauri_plugin_updater::Builder::new\(\)\.build\(\)/);
});

test('desktop capability allows updater and process frontend APIs', () => {
  assert.ok(permissions.includes('updater:default'));
  assert.ok(permissions.includes('process:default'));
});

test('tauri updater config includes public key and github endpoint', () => {
  const updater = tauriConfig.plugins?.updater;
  assert.equal(typeof updater?.pubkey, 'string');
  assert.ok(updater.pubkey.length > 20);
  assert.deepEqual(updater.endpoints, [
    'https://github.com/oooyuy92/orion-chat-rs/releases/latest/download/latest.json',
  ]);
});

test('release workflow generates updater metadata and reads signing secrets', () => {
  assert.match(releaseWorkflow, /includeUpdaterJson:\s*true/);
  assert.match(releaseWorkflow, /TAURI_SIGNING_PRIVATE_KEY/);
});
