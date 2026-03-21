// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

function readSource() {
  try {
    return readFileSync(new URL('./theme.js', import.meta.url), 'utf8');
  } catch {
    return '';
  }
}

test('theme utility exports default theme and preset options', () => {
  const source = readSource();
  assert.match(source, /export const DEFAULT_THEME = 'default';/);
  assert.match(source, /export const THEME_OPTIONS = \[/);
  assert.match(source, /id: 'blue'/);
  assert.match(source, /id: 'yellow'/);
});

test('theme utility normalizes and applies valid theme ids', () => {
  const source = readSource();
  assert.match(source, /export function normalizeTheme\(theme\)/);
  assert.match(source, /export function applyTheme\(/);
  assert.match(source, /root\.dataset\.theme = resolvedTheme/);
  assert.match(source, /root\.setAttribute\('data-theme', resolvedTheme\)/);
});
