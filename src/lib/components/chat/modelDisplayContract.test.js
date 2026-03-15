// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const modelSelector = readFileSync(new URL('./ModelSelector.svelte', import.meta.url), 'utf8');
const comboSelector = readFileSync(new URL('./ComboSelector.svelte', import.meta.url), 'utf8');
const versionCompareView = readFileSync(new URL('./VersionCompareView.svelte', import.meta.url), 'utf8');
const assistantSettings = readFileSync(
  new URL('../settings/AssistantSettings.svelte', import.meta.url),
  'utf8',
);

test('model selector uses shared model-display helper for trigger and menu labels', () => {
  assert.match(modelSelector, /from '\$lib\/utils\/modelDisplay'/);
  assert.match(modelSelector, /resolveModelLabel/);
});

test('model selector pins composer dropdown placement to the top with viewport padding', () => {
  assert.match(modelSelector, /<DropdownMenuContent[^>]*side="top"/s);
  assert.match(modelSelector, /<DropdownMenuContent[^>]*collisionPadding=\{8\}/s);
});

test('assistant settings uses shared model-display helper for default model options', () => {
  assert.match(assistantSettings, /from '\$lib\/utils\/modelDisplay'/);
  assert.match(assistantSettings, /resolveModelLabel/);
});

test('combo selector uses shared model-display helper for combo model names', () => {
  assert.match(comboSelector, /from '\$lib\/utils\/modelDisplay'/);
  assert.match(comboSelector, /resolveModelLabel/);
});

test('version compare view uses shared model-display helper for version headers', () => {
  assert.match(versionCompareView, /from '\$lib\/utils\/modelDisplay'/);
  assert.match(versionCompareView, /resolveModelLabel/);
});
