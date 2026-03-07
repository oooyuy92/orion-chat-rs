import { invoke, Channel } from '@tauri-apps/api/core';
import type {
  Conversation,
  Message,
  ProviderConfig,
  ProviderType,
  ModelInfo,
  Assistant,
  VersionInfo,
  CommonParams,
  ProviderParams,
} from '$lib/types';

export type ChatEventHandler = (event: ChatEvent) => void;

export type ChatEvent =
  | { type: 'started'; messageId: string }
  | { type: 'delta'; content: string }
  | { type: 'reasoning'; content: string }
  | { type: 'usage'; promptTokens: number; completionTokens: number }
  | { type: 'finished'; messageId: string }
  | { type: 'error'; message: string };

export type AppPaths = {
  dataDir: string;
  logDir: string;
  dbPath: string;
  cacheDir: string;
};

export const api = {
  // Conversations
  createConversation(title: string, assistantId?: string, modelId?: string): Promise<Conversation> {
    return invoke('create_conversation', { title, assistantId: assistantId ?? null, modelId: modelId ?? null });
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
  pinConversation(id: string, isPinned: boolean): Promise<void> {
    return invoke('pin_conversation', { id, isPinned });
  },
  generateConversationTitle(conversationId: string, modelId: string): Promise<string> {
    return invoke('generate_conversation_title', { conversationId, modelId });
  },

  // Messages
  getMessages(conversationId: string): Promise<Message[]> {
    return invoke('get_messages', { conversationId });
  },
  sendMessage(conversationId: string, content: string, modelId: string, onEvent: ChatEventHandler, commonParams?: CommonParams, providerParams?: ProviderParams): Promise<Message> {
    const channel = new Channel<ChatEvent>();
    channel.onmessage = onEvent;
    return invoke('send_message', { conversationId, content, modelId, channel, commonParams: commonParams ?? null, providerParams: providerParams ?? null });
  },
  resendMessage(conversationId: string, modelId: string, onEvent: ChatEventHandler, commonParams?: CommonParams, providerParams?: ProviderParams): Promise<Message> {
    const channel = new Channel<ChatEvent>();
    channel.onmessage = onEvent;
    return invoke('resend_message', { conversationId, modelId, channel, commonParams: commonParams ?? null, providerParams: providerParams ?? null });
  },
  stopGeneration(): Promise<void> {
    return invoke('stop_generation');
  },
  compressConversation(conversationId: string, modelId: string, onEvent: ChatEventHandler): Promise<Message[]> {
    const channel = new Channel<ChatEvent>();
    channel.onmessage = onEvent;
    return invoke('compress_conversation', { conversationId, modelId, channel });
  },
  deleteMessage(id: string): Promise<void> {
    return invoke('delete_message', { id });
  },
  restoreMessage(id: string): Promise<void> {
    return invoke('restore_message', { id });
  },
  deleteMessagesAfter(conversationId: string, messageId: string): Promise<void> {
    return invoke('delete_messages_after', { conversationId, messageId });
  },
  deleteMessagesFrom(conversationId: string, messageId: string): Promise<void> {
    return invoke('delete_messages_from', { conversationId, messageId });
  },
  updateMessageContent(id: string, content: string): Promise<void> {
    return invoke('update_message_content', { id, content });
  },

  // Versions
  generateVersion(conversationId: string, messageId: string, modelId: string, onEvent: ChatEventHandler, commonParams?: CommonParams, providerParams?: ProviderParams): Promise<Message> {
    const channel = new Channel<ChatEvent>();
    channel.onmessage = onEvent;
    return invoke('generate_version', { conversationId, messageId, modelId, channel, commonParams: commonParams ?? null, providerParams: providerParams ?? null });
  },
  regenerateMessage(conversationId: string, messageId: string, modelId: string, onEvent: ChatEventHandler, commonParams?: CommonParams, providerParams?: ProviderParams): Promise<Message> {
    const channel = new Channel<ChatEvent>();
    channel.onmessage = onEvent;
    return invoke('regenerate_message', { conversationId, messageId, modelId, channel, commonParams: commonParams ?? null, providerParams: providerParams ?? null });
  },
  switchVersion(versionGroupId: string, versionNumber: number): Promise<void> {
    return invoke('switch_version', { versionGroupId, versionNumber });
  },
  listVersions(versionGroupId: string): Promise<VersionInfo[]> {
    return invoke('list_versions', { versionGroupId });
  },
  getVersionModels(versionGroupId: string): Promise<[number, string][]> {
    return invoke('get_version_models', { versionGroupId });
  },

  // Providers
  addProvider(name: string, providerType: ProviderType, apiBase: string, apiKey?: string, enabled = true): Promise<ProviderConfig> {
    return invoke('add_provider', { name, providerType, apiKey: apiKey ?? null, apiBase, enabled });
  },
  listProviders(): Promise<ProviderConfig[]> {
    return invoke('list_providers');
  },
  updateProvider(id: string, name: string, providerType: ProviderType, apiBase: string, apiKey: string | null, enabled: boolean): Promise<ProviderConfig> {
    return invoke('update_provider', { id, name, providerType, apiBase, apiKey, enabled });
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
  createAssistant(name: string, systemPrompt?: string, modelId?: string, temperature?: number, topP?: number, maxTokens?: number): Promise<Assistant> {
    return invoke('create_assistant', { name, systemPrompt: systemPrompt ?? null, modelId: modelId ?? null, temperature: temperature ?? null, topP: topP ?? null, maxTokens: maxTokens ?? null });
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

  // Settings — paths & filesystem
  getAppPaths(): Promise<AppPaths> {
    return invoke('get_app_paths');
  },
  openPath(path: string): Promise<void> {
    return invoke('open_path', { path });
  },
  pickDirectory(): Promise<string | null> {
    return invoke('pick_directory');
  },
  getCacheSize(): Promise<string> {
    return invoke('get_cache_size');
  },
  clearCache(): Promise<void> {
    return invoke('clear_cache');
  },
  resetAppData(): Promise<void> {
    return invoke('reset_app_data');
  },
  localBackup(destPath: string): Promise<void> {
    return invoke('local_backup', { destPath });
  },

  // Settings — autostart & proxy
  getAutostartEnabled(): Promise<boolean> {
    return invoke('get_autostart_enabled');
  },
  setAutostartEnabled(enabled: boolean): Promise<void> {
    return invoke('set_autostart_enabled', { enabled });
  },
  setProxyMode(mode: string): Promise<void> {
    return invoke('set_proxy_mode', { mode });
  },
  getProxyMode(): Promise<string> {
    return invoke('get_proxy_mode');
  },
};
