import { writable } from 'svelte/store';
import type { ToolCallState } from '$lib/types';

export const agentMode = writable<boolean>(true);
export const activeToolCalls = writable<ToolCallState[]>([]);
export const pendingAuth = writable<{
  toolCallId: string;
  toolName: string;
  args: string;
} | null>(null);

export function addToolCall(call: ToolCallState) {
  activeToolCalls.update((calls) => [...calls, call]);
}

export function updateToolCall(toolCallId: string, partial: Partial<ToolCallState>) {
  activeToolCalls.update((calls) =>
    calls.map((call) => (call.toolCallId === toolCallId ? { ...call, ...partial } : call)),
  );
}

export function completeToolCall(toolCallId: string, result: string, isError: boolean) {
  activeToolCalls.update((calls) =>
    calls.map((call) =>
      call.toolCallId === toolCallId
        ? {
            ...call,
            status: isError ? 'error' : 'completed',
            result,
            endTime: Date.now(),
          }
        : call,
    ),
  );
}

export function clearToolCalls() {
  activeToolCalls.set([]);
}
