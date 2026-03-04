import { invoke, Channel } from '@tauri-apps/api/core';
import type {
  Conversation,
  Message,
  ProviderConfig,
  ProviderType,
  ModelInfo,
  Assistant,
} from '$lib/types';

export type ChatEventHandler = (event: ChatEvent) => void;

export type ChatEvent =
  | { type: 'started'; messageId: string }
  | { type: 'delta'; content: string }
  | { type: 'reasoning'; content: string }
  | { type: 'usage'; promptTokens: number; completionTokens: number }
  | { type: 'finished'; messageId: string }
  | { type: 'error'; message: string };

export const api = {
  // Conversations
  createConversation(title: string, assistantId?: string, modelId?: string): Promise<Conversation> {
    return invoke('create_conversation', {
      title,
      assistantId: assistantId ?? null,
      modelId: modelId ?? null,
    });
  },

  listConversations(): Promise<Conversation[]> {
    return invoke('list_conversations');
  },

  updateConversationTitle(id: string, title: string): Promise<void> {
    return invoke('update_conversation_title', { id, title });
  },

  deleteConversation(id: string): Promise<void> {
    return invoke('delete_conversation', { id });
  },

  // Messages
  getMessages(conversationId: string): Promise<Message[]> {
    return invoke('get_messages', { conversationId });
  },

  sendMessage(
    conversationId: string,
    content: string,
    modelId: string,
    onEvent: ChatEventHandler,
  ): Promise<Message> {
    const channel = new Channel<ChatEvent>();
    channel.onmessage = onEvent;
    return invoke('send_message', {
      conversationId,
      content,
      modelId,
      channel,
    });
  },

  stopGeneration(): Promise<void> {
    return invoke('stop_generation');
  },

  // Providers
  addProvider(
    name: string,
    providerType: ProviderType,
    apiBase: string,
    apiKey?: string,
    enabled = true,
  ): Promise<ProviderConfig> {
    return invoke('add_provider', {
      name,
      providerType,
      apiKey: apiKey ?? null,
      apiBase,
      enabled,
    });
  },

  listProviders(): Promise<ProviderConfig[]> {
    return invoke('list_providers');
  },

  updateProvider(
    id: string,
    name: string,
    providerType: ProviderType,
    apiBase: string,
    apiKey: string | null,
    enabled: boolean,
  ): Promise<ProviderConfig> {
    return invoke('update_provider', {
      id,
      name,
      providerType,
      apiBase,
      apiKey,
      enabled,
    });
  },

  deleteProvider(id: string): Promise<void> {
    return invoke('delete_provider', { id });
  },

  fetchModels(providerId: string): Promise<ModelInfo[]> {
    return invoke('fetch_models', { providerId });
  },

  updateModelVisibility(modelId: string, enabled: boolean): Promise<void> {
    return invoke('update_model_visibility', { modelId, enabled });
  },

  updateProviderModelsVisibility(providerId: string, enabled: boolean): Promise<number> {
    return invoke('update_provider_models_visibility', { providerId, enabled });
  },

  // Assistants
  createAssistant(
    name: string,
    systemPrompt?: string,
    modelId?: string,
    temperature?: number,
    topP?: number,
    maxTokens?: number,
  ): Promise<Assistant> {
    return invoke('create_assistant', {
      name,
      systemPrompt: systemPrompt ?? null,
      modelId: modelId ?? null,
      temperature: temperature ?? null,
      topP: topP ?? null,
      maxTokens: maxTokens ?? null,
    });
  },

  listAssistants(): Promise<Assistant[]> {
    return invoke('list_assistants');
  },

  updateAssistant(assistant: Assistant): Promise<void> {
    return invoke('update_assistant', { assistant });
  },

  deleteAssistant(id: string): Promise<void> {
    return invoke('delete_assistant', { id });
  },

  // Search
  searchMessages(query: string): Promise<Message[]> {
    return invoke('search_messages', { query });
  },

  // Export
  exportMarkdown(conversationId: string): Promise<string> {
    return invoke('export_conversation_markdown', { conversationId });
  },

  exportJson(conversationId: string): Promise<string> {
    return invoke('export_conversation_json', { conversationId });
  },
};
