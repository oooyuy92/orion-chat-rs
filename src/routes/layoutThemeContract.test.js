// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./+layout.svelte', import.meta.url), 'utf8');

test('layout loads the saved theme and applies it on mount', () => {
  assert.match(source, /store\.get<[^>]+>\('theme'\)/);
  assert.match(source, /applyTheme\(/);
});
