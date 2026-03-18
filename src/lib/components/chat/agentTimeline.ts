import type { Message, ToolCallState } from '$lib/types';

export interface MessageRow {
  message: Message;
  timelineCalls: ToolCallState[];
}

function parseTimestamp(value: string): number {
  const timestamp = Date.parse(value);
  return Number.isNaN(timestamp) ? 0 : timestamp;
}

function toToolCallState(message: Message): ToolCallState {
  return {
    toolCallId: message.toolCallId ?? message.id,
    toolName: message.toolName ?? 'tool',
    args: message.toolInput ?? '',
    status: 'running',
    messageId: message.id,
    startTime: parseTimestamp(message.createdAt),
  };
}

function mergeToolResult(existing: ToolCallState | undefined, message: Message): ToolCallState {
  return {
    toolCallId: message.toolCallId ?? existing?.toolCallId ?? message.id,
    toolName: message.toolName ?? existing?.toolName ?? 'tool',
    args: existing?.args ?? message.toolInput ?? '',
    status: message.toolError ? 'error' : 'completed',
    result: message.content,
    messageId: existing?.messageId ?? message.id,
    startTime: existing?.startTime ?? parseTimestamp(message.createdAt),
    endTime: parseTimestamp(message.createdAt),
  };
}

export function buildMessageRows(messages: Message[], liveToolCalls: ToolCallState[]): MessageRow[] {
  const rows: MessageRow[] = [];
  let pendingToolCalls: ToolCallState[] = [];

  for (const message of messages) {
    if (message.messageType === 'toolCall') {
      pendingToolCalls = [...pendingToolCalls, toToolCallState(message)];
      continue;
    }

    if (message.messageType === 'toolResult') {
      const toolCallId = message.toolCallId ?? message.id;
      const existing = pendingToolCalls.find((call) => call.toolCallId === toolCallId);
      const merged = mergeToolResult(existing, message);

      if (existing) {
        pendingToolCalls = pendingToolCalls.map((call) =>
          call.toolCallId === toolCallId ? merged : call,
        );
      } else {
        pendingToolCalls = [...pendingToolCalls, merged];
      }
      continue;
    }

    const timelineCalls =
      message.role === 'assistant' && message.status === 'streaming' && liveToolCalls.length > 0
        ? liveToolCalls
        : message.role === 'assistant' && pendingToolCalls.length > 0
          ? pendingToolCalls
          : [];

    rows.push({
      message,
      timelineCalls,
    });

    if (message.role === 'assistant' && pendingToolCalls.length > 0) {
      pendingToolCalls = [];
    }
  }

  return rows;
}
