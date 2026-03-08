// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./ChatArea.svelte', import.meta.url), 'utf8');

test('chat message panel parent is a flex container so inner scroller can own vertical scrolling', () => {
  assert.match(
    source,
    /<div class="flex-1 min-h-0 flex">\s*<MessageList/s,
  );
});
