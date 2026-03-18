// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';

const { buildMessageRows } = await import('./agentTimeline.ts');

test('buildMessageRows attaches persisted tool call/result messages to the following assistant text message', () => {
  const messages = [
    {
      id: 'user-1',
      conversationId: 'conv-1',
      role: 'user',
      content: 'list files',
      reasoning: null,
      modelId: null,
      status: 'done',
      tokenCount: null,
      createdAt: '2026-03-18T00:00:00.000Z',
      versionGroupId: null,
      versionNumber: 1,
      totalVersions: 1,
      messageType: 'text',
      toolError: false,
    },
    {
      id: 'tool-start-1',
      conversationId: 'conv-1',
      role: 'assistant',
      content: '',
      reasoning: null,
      modelId: 'model-1',
      status: 'done',
      tokenCount: null,
      createdAt: '2026-03-18T00:00:01.000Z',
      versionGroupId: null,
      versionNumber: 1,
      totalVersions: 1,
      messageType: 'toolCall',
      toolCallId: 'call-1',
      toolName: 'list_files',
      toolInput: '{"path":"."}',
      toolError: false,
    },
    {
      id: 'tool-result-1',
      conversationId: 'conv-1',
      role: 'assistant',
      content: 'README.md\nsrc',
      reasoning: null,
      modelId: 'model-1',
      status: 'done',
      tokenCount: null,
      createdAt: '2026-03-18T00:00:02.000Z',
      versionGroupId: null,
      versionNumber: 1,
      totalVersions: 1,
      messageType: 'toolResult',
      toolCallId: 'call-1',
      toolName: 'list_files',
      toolError: false,
    },
    {
      id: 'assistant-1',
      conversationId: 'conv-1',
      role: 'assistant',
      content: 'I found two entries.',
      reasoning: null,
      modelId: 'model-1',
      status: 'done',
      tokenCount: null,
      createdAt: '2026-03-18T00:00:03.000Z',
      versionGroupId: null,
      versionNumber: 1,
      totalVersions: 1,
      messageType: 'text',
      toolError: false,
    },
  ];

  const rows = buildMessageRows(messages, []);
  assert.equal(rows.length, 2);
  assert.equal(rows[0].message.id, 'user-1');
  assert.deepEqual(rows[0].timelineCalls, []);
  assert.equal(rows[1].message.id, 'assistant-1');
  assert.equal(rows[1].timelineCalls.length, 1);
  assert.deepEqual(rows[1].timelineCalls[0], {
    toolCallId: 'call-1',
    toolName: 'list_files',
    args: '{"path":"."}',
    status: 'completed',
    result: 'README.md\nsrc',
    messageId: 'tool-start-1',
    startTime: Date.parse('2026-03-18T00:00:01.000Z'),
    endTime: Date.parse('2026-03-18T00:00:02.000Z'),
  });
});

test('buildMessageRows prefers live tool calls for a streaming assistant row', () => {
  const messages = [
    {
      id: 'assistant-stream',
      conversationId: 'conv-1',
      role: 'assistant',
      content: '',
      reasoning: null,
      modelId: 'model-1',
      status: 'streaming',
      tokenCount: null,
      createdAt: '2026-03-18T00:00:00.000Z',
      versionGroupId: null,
      versionNumber: 1,
      totalVersions: 1,
      messageType: 'text',
      toolError: false,
    },
  ];
  const liveCalls = [
    {
      toolCallId: 'call-live',
      toolName: 'bash',
      args: '{"command":"pwd"}',
      status: 'running',
      messageId: 'tool-live-1',
      startTime: 10,
    },
  ];

  const rows = buildMessageRows(messages, liveCalls);
  assert.equal(rows.length, 1);
  assert.deepEqual(rows[0].timelineCalls, liveCalls);
});
