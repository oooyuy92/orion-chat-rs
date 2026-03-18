// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./agent.ts', import.meta.url), 'utf8');

test('agent API exports the required tauri commands', () => {
  assert.match(source, /export async function agentChat\(/);
  assert.match(source, /new Channel<ChatEvent>\(\)/);
  assert.match(source, /invoke<Message>\('agent_chat'/);
  assert.match(source, /export async function agentStop\(/);
  assert.match(source, /invoke\('agent_stop'/);
  assert.match(source, /export async function agentAuthorizeTool\(/);
  assert.match(source, /invoke\('agent_authorize_tool'/);
  assert.match(source, /export async function getToolPermissions\(/);
  assert.match(source, /invoke<ToolPermissions>\('get_tool_permissions'/);
  assert.match(source, /export async function setToolPermissions\(/);
  assert.match(source, /invoke\('set_tool_permissions'/);
});
