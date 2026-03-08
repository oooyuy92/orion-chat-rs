// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./ConversationList.svelte', import.meta.url), 'utf8');

test('conversation list renders a search input below the new chat button', () => {
  assert.match(source, /placeholder=\{i18n\.t\.searchMessages\}/);
});

test('conversation list calls sidebar search api and switches to search results mode', () => {
  assert.match(source, /api\.searchSidebarResults\(/);
  assert.match(source, /searchResults/);
});
