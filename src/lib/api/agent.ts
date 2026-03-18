import { Channel, invoke } from '@tauri-apps/api/core';
import type { AuthAction, ChatEvent, Message, ToolPermissions } from '$lib/types';

export async function agentChat(
  conversationId: string,
  message: string,
  modelId: string,
  onEvent: (event: ChatEvent) => void,
): Promise<Message> {
  const channel = new Channel<ChatEvent>();
  channel.onmessage = onEvent;

  return invoke<Message>('agent_chat', {
    conversationId,
    message,
    modelId,
    channel,
  });
}

export async function agentStop(conversationId: string): Promise<void> {
  return invoke('agent_stop', { conversationId });
}

export async function agentAuthorizeTool(
  toolCallId: string,
  action: AuthAction,
): Promise<void> {
  return invoke('agent_authorize_tool', { toolCallId, action });
}

export async function getToolPermissions(): Promise<ToolPermissions> {
  return invoke<ToolPermissions>('get_tool_permissions');
}

export async function setToolPermissions(perms: ToolPermissions): Promise<void> {
  return invoke('set_tool_permissions', { permissions: perms });
}
