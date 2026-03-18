// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const agentToggle = readFileSync(new URL('./AgentToggle.svelte', import.meta.url), 'utf8');
const toolTimeline = readFileSync(new URL('./ToolTimeline.svelte', import.meta.url), 'utf8');
const toolTimelineItem = readFileSync(new URL('./ToolTimelineItem.svelte', import.meta.url), 'utf8');
const toolAuthDialog = readFileSync(new URL('./ToolAuthDialog.svelte', import.meta.url), 'utf8');
const inputArea = readFileSync(new URL('./InputArea.svelte', import.meta.url), 'utf8');
const messageList = readFileSync(new URL('./MessageList.svelte', import.meta.url), 'utf8');
const page = readFileSync(new URL('../../../routes/+page.svelte', import.meta.url), 'utf8');

test('input area renders AgentToggle in the model row', () => {
  assert.match(agentToggle, /import BotIcon from '@lucide\/svelte\/icons\/bot'/);
  assert.match(agentToggle, /agentMode/);
  assert.match(inputArea, /import AgentToggle from '\.\/AgentToggle\.svelte'/);
  assert.match(inputArea, /<AgentToggle \{disabled\} \/>/);
});

test('tool timeline components render timeline items and expandable results', () => {
  assert.match(toolTimeline, /<ToolTimelineItem \{call\} \/>/);
  assert.match(toolTimeline, /border-left|border-left: 2px solid|border-left:2px solid/);
  assert.match(toolTimelineItem, /call\.status === 'completed'/);
  assert.match(toolTimelineItem, /call\.status === 'error'/);
  assert.match(toolTimelineItem, /expanded && call\.result/);
});

test('tool auth dialog wires pending auth to authorize actions', () => {
  assert.match(toolAuthDialog, /import \* as Dialog from '\$lib\/components\/ui\/dialog'/);
  assert.match(toolAuthDialog, /import \{ Button \} from '\$lib\/components\/ui\/button'/);
  assert.match(toolAuthDialog, /agentAuthorizeTool/);
  assert.match(toolAuthDialog, /respond\('allowSession'\)/);
  assert.match(toolAuthDialog, /respond\('allow'\)/);
  assert.match(toolAuthDialog, /respond\('deny'\)/);
});

test('message flow integrates tool timeline and auth dialog', () => {
  assert.match(messageList, /ToolTimeline/);
  assert.match(page, /agentChat/);
  assert.match(page, /agentStop/);
  assert.match(page, /pendingAuth\.set/);
  assert.match(page, /<ToolAuthDialog \/>/);
});
