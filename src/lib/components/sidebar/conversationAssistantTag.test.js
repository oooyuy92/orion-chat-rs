// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./ConversationList.svelte', import.meta.url), 'utf8');

test('conversation list loads assistants for sidebar labels', () => {
  assert.match(source, /api\.listAssistants\(/);
});

test('conversation list renders assistant label under title', () => {
  assert.match(source, /#\{assistantNameFor\(conversation\)\}/);
});
