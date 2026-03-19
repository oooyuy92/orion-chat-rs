// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../app.css', import.meta.url), 'utf8');

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
