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

test('provider settings renders theme options as a single-row circular swatch picker', () => {
  assert.match(source, /THEME_OPTIONS/);
  assert.match(source, /class="theme-swatch-row"/);
  assert.match(source, /class="theme-swatch"/);
  assert.match(source, /class="theme-swatch-fill"/);
  assert.match(source, /\.theme-swatch-row\s*\{/);
  assert.match(source, /\.theme-swatch\.is-selected\s*\{[\s\S]*border-color:\s*var\(--swatch-color\);/);
  assert.doesNotMatch(source, /class="theme-card"/);
  assert.doesNotMatch(source, /class="theme-preview"/);
});
