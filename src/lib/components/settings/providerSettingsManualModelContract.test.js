// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./ProviderSettings.svelte', import.meta.url), 'utf8');

test('provider settings exposes add-model action and dialog state', () => {
  assert.match(source, /let showAddModelDialog = \$state\(false\);/);
  assert.match(source, /onclick=\{\(\) => \(showAddModelDialog = true\)\}/);
  assert.match(source, /manualModelAddBtn|添加模型|Add Model/);
});

test('provider settings includes manual-model form fields', () => {
  assert.match(source, /let manualModelRequestName = \$state\(''\);/);
  assert.match(source, /let manualModelDisplayName = \$state\(''\);/);
  assert.match(source, /let manualModelEnabled = \$state\(true\);/);
  assert.match(source, /api\.createManualModel\(/);
});

test('provider settings does not reset create-in-flight state when dialog closes', () => {
  assert.doesNotMatch(
    source,
    /\$effect\(\(\) => \{\s*if \(!showAddModelDialog\) \{\s*resetManualModelDraft\(\);\s*creatingManualModel = false;/s,
  );
});

test('provider settings captures provider id before awaiting manual model creation', () => {
  assert.match(source, /const providerId = selectedProvider\.id;/);
  assert.match(source, /api\.createManualModel\(\s*providerId,/s);
  assert.match(source, /provider\.id === providerId/s);
});

test('provider settings renders manual-model badge and dual-line labels', () => {
  assert.match(source, /resolveModelLabel\(/);
  assert.match(source, /resolveModelSecondaryLabel\(/);
  assert.match(source, /isManualModel\(/);
  assert.match(source, /model-source-badge/);
  assert.match(source, /model-card-secondary/);
});
