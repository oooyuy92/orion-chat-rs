// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./ProviderSettings.svelte', import.meta.url), 'utf8');

test('provider settings persists selected theme under the theme key', () => {
  assert.match(source, /store\.get<[^>]+>\('theme'\)/);
  assert.match(source, /store\.set\('theme',\s*selectedTheme\)/);
});

test('provider settings no longer persists legacy colorIndex theme state', () => {
  assert.doesNotMatch(source, /store\.set\('colorIndex'/);
  assert.doesNotMatch(source, /store\.get<[^>]+>\('colorIndex'\)/);
});
