// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';

import {
  DEFAULT_THEME,
  THEME_OPTIONS,
  applyTheme,
  normalizeTheme,
} from './theme.js';

test('exports exactly the official shadcn theme keys', () => {
  assert.deepEqual(
    THEME_OPTIONS.map((theme) => theme.id),
    ['default', 'blue', 'green', 'orange', 'red', 'rose', 'violet', 'yellow'],
  );
});

test('normalizes unknown theme keys to default', () => {
  assert.equal(normalizeTheme('blue'), 'blue');
  assert.equal(normalizeTheme('unknown-theme'), DEFAULT_THEME);
  assert.equal(normalizeTheme(null), DEFAULT_THEME);
});

test('applyTheme writes the root data-theme attribute', () => {
  const root = {
    dataset: {},
    setAttribute(name, value) {
      if (name === 'data-theme') {
        this.dataset.theme = value;
      }
    },
  };

  applyTheme('green', root);
  assert.equal(root.dataset.theme, 'green');

  applyTheme('not-real', root);
  assert.equal(root.dataset.theme, DEFAULT_THEME);
});

test('theme previews keep neutral surfaces while only primary changes across themes', () => {
  const backgrounds = new Set(THEME_OPTIONS.map((theme) => theme.preview.background));
  const borders = new Set(THEME_OPTIONS.map((theme) => theme.preview.border));
  const primaries = new Set(THEME_OPTIONS.map((theme) => theme.preview.primary));

  assert.equal(backgrounds.size, 1);
  assert.equal(borders.size, 1);
  assert.notEqual(primaries.size, 1);
  assert.ok(THEME_OPTIONS.every((theme) => !theme.preview.primary.includes('oklch(')));
});
