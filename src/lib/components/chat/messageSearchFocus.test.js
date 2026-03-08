// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const messageListSource = readFileSync(new URL('./MessageList.svelte', import.meta.url), 'utf8');
const pageSource = readFileSync(new URL('../../../routes/+page.svelte', import.meta.url), 'utf8');

test('message list accepts focusedMessageId and marks message rows with ids', () => {
  assert.match(messageListSource, /focusedMessageId/);
  assert.match(messageListSource, /data-message-id=\{message\.id\}/);
});

test('page search selection flow keeps a pending focus message id', () => {
  assert.match(pageSource, /pendingFocusMessageId/);
});
