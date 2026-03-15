// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const pageSource = readFileSync(new URL('../../../routes/+page.svelte', import.meta.url), 'utf8');
const invokeSource = readFileSync(new URL('../../utils/invoke.ts', import.meta.url), 'utf8');

test('frontend api exposes conversation model update', () => {
  assert.match(invokeSource, /updateConversationModel\(id: string, modelId: string \| null\): Promise<void>/);
});

test('page restores selected model from conversation model id before assistant defaults', () => {
  assert.match(pageSource, /conversation\?\.modelId/);
});

test('page persists manual model selection back to the active conversation', () => {
  assert.match(pageSource, /api\.updateConversationModel\(/);
});
