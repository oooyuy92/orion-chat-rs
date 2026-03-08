// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const chatAreaSource = readFileSync(new URL('./ChatArea.svelte', import.meta.url), 'utf8');
const messageListSource = readFileSync(new URL('./MessageList.svelte', import.meta.url), 'utf8');
const sidebarInsetSource = readFileSync(new URL('../ui/sidebar/sidebar-inset.svelte', import.meta.url), 'utf8');
const sidebarProviderSource = readFileSync(new URL('../ui/sidebar/sidebar-provider.svelte', import.meta.url), 'utf8');

test('chat message panel parent remains flex so inner scroller has bounded height', () => {
  assert.match(chatAreaSource, /<div class="flex-1 min-h-0 flex">\s*<MessageList/s);
});

test('message list viewport contains overscroll to prevent page scrolling at edges', () => {
  assert.match(messageListSource, /overscroll-behavior:\s*contain;/);
});

test('sidebar inset clips its content instead of letting the page become scrollable', () => {
  assert.match(sidebarInsetSource, /min-h-0/);
  assert.match(sidebarInsetSource, /overflow-hidden/);
});

test('app shell stays pinned to the viewport height', () => {
  assert.match(sidebarProviderSource, /h-svh/);
  assert.match(sidebarProviderSource, /overflow-hidden/);
});
