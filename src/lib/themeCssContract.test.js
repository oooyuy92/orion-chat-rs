// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../app.css', import.meta.url), 'utf8');
const nonDefaultThemeBlocks = Array.from(
  source.matchAll(/:root\[data-theme="([^"]+)"\]\s*\{([\s\S]*?)\n\}/g),
);

for (const theme of ['default', 'blue', 'green', 'orange', 'red', 'rose', 'violet', 'yellow']) {
  test(`app.css defines selector for ${theme} theme preset`, () => {
    if (theme === 'default') {
      assert.match(source, /:root,\s*:root\[data-theme="default"\]/);
      return;
    }

    const escaped = theme.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    assert.match(source, new RegExp(`:root\\[data-theme="${escaped}"\\]`));
  });
}

for (const [, themeName, block] of nonDefaultThemeBlocks.filter(([_, themeName]) => themeName !== 'default')) {
  test(`${themeName} theme only overrides accent-related tokens`, () => {
    assert.match(block, /--primary:/);
    assert.doesNotMatch(block, /--background:/);
    assert.doesNotMatch(block, /--foreground:/);
    assert.doesNotMatch(block, /--card:/);
    assert.doesNotMatch(block, /--card-foreground:/);
    assert.doesNotMatch(block, /--popover:/);
    assert.doesNotMatch(block, /--popover-foreground:/);
    assert.doesNotMatch(block, /--secondary:/);
    assert.doesNotMatch(block, /--secondary-foreground:/);
    assert.doesNotMatch(block, /--muted:/);
    assert.doesNotMatch(block, /--muted-foreground:/);
    assert.doesNotMatch(block, /--accent:/);
    assert.doesNotMatch(block, /--accent-foreground:/);
    assert.doesNotMatch(block, /--border:/);
    assert.doesNotMatch(block, /--input:/);
    assert.doesNotMatch(block, /--sidebar:/);
    assert.doesNotMatch(block, /--sidebar-foreground:/);
    assert.doesNotMatch(block, /--sidebar-border:/);
    assert.doesNotMatch(block, /--surface:/);
  });
}
