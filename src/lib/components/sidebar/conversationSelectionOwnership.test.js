// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const appSidebarSource = readFileSync(new URL('./AppSidebar.svelte', import.meta.url), 'utf8');
const pageSource = readFileSync(new URL('../../../routes/+page.svelte', import.meta.url), 'utf8');

test('app sidebar passes active conversation id as a read-only prop to conversation list', () => {
  assert.match(appSidebarSource, /activeId=\{activeConversationId\}/);
  assert.doesNotMatch(appSidebarSource, /bind:activeId=\{activeConversationId\}/);
});

test('page keeps conversation switching centralized in handleConversationSelect', () => {
  assert.match(pageSource, /onConversationSelect=\{handleConversationSelect\}/);
  assert.match(pageSource, /function handleConversationSelect\(selection: ConversationSelection\)/);
});
