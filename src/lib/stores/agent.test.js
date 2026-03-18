// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { get } from 'svelte/store';

const agentStore = await import('./agent.ts');

test('agent store starts in agent mode with no tool calls or pending auth', () => {
  assert.equal(get(agentStore.agentMode), true);
  assert.deepEqual(get(agentStore.activeToolCalls), []);
  assert.equal(get(agentStore.pendingAuth), null);
});

test('agent store adds, updates, completes, and clears tool calls', () => {
  agentStore.clearToolCalls();

  agentStore.addToolCall({
    toolCallId: 'call-1',
    toolName: 'bash',
    args: '{"command":"pwd"}',
    status: 'running',
    messageId: 'tool-msg-1',
    startTime: 1,
  });

  assert.deepEqual(get(agentStore.activeToolCalls), [
    {
      toolCallId: 'call-1',
      toolName: 'bash',
      args: '{"command":"pwd"}',
      status: 'running',
      messageId: 'tool-msg-1',
      startTime: 1,
    },
  ]);

  agentStore.updateToolCall('call-1', { result: '/tmp', status: 'running' });
  assert.deepEqual(get(agentStore.activeToolCalls), [
    {
      toolCallId: 'call-1',
      toolName: 'bash',
      args: '{"command":"pwd"}',
      status: 'running',
      result: '/tmp',
      messageId: 'tool-msg-1',
      startTime: 1,
    },
  ]);

  agentStore.completeToolCall('call-1', '/tmp', false);
  const [completedCall] = get(agentStore.activeToolCalls);
  assert.equal(completedCall.status, 'completed');
  assert.equal(completedCall.result, '/tmp');
  assert.equal(typeof completedCall.endTime, 'number');

  agentStore.clearToolCalls();
  assert.deepEqual(get(agentStore.activeToolCalls), []);
});

test('agent store marks tool errors and pending auth state', () => {
  agentStore.clearToolCalls();
  agentStore.addToolCall({
    toolCallId: 'call-2',
    toolName: 'read_file',
    args: '{"path":"README.md"}',
    status: 'running',
    messageId: 'tool-msg-2',
    startTime: 2,
  });

  agentStore.completeToolCall('call-2', 'permission denied', true);
  const [failedCall] = get(agentStore.activeToolCalls);
  assert.equal(failedCall.status, 'error');
  assert.equal(failedCall.result, 'permission denied');

  agentStore.pendingAuth.set({
    toolCallId: 'call-3',
    toolName: 'bash',
    args: '{"command":"echo hello"}',
  });
  assert.deepEqual(get(agentStore.pendingAuth), {
    toolCallId: 'call-3',
    toolName: 'bash',
    args: '{"command":"echo hello"}',
  });
  agentStore.pendingAuth.set(null);
});
