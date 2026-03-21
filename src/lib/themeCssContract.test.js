// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../app.css', import.meta.url), 'utf8');

for (const themeId of ['default', 'blue', 'green', 'orange', 'red', 'rose', 'violet', 'yellow']) {
  test(`app.css defines ${themeId} theme variables`, () => {
    const selector =
      themeId === 'default'
        ? /:root,\s*:root\[data-theme="default"\]\s*\{[\s\S]*--primary:/
        : new RegExp(`:root\\[data-theme="${themeId}"\\]\\s*\\{[\\s\\S]*--primary:`);
    assert.match(source, selector);
  });
}
