/**
 * Web 端 API 实现（HTTP fetch）。
 * 与 Tauri 端的 invoke.ts 保持相同的函数签名。
 */
import type {
  Conversation,
  Message,
  PagedMessages,
  GetMessagesOptions,
  ProviderConfig,
  ProviderType,
  ModelInfo,
  Assistant,
  VersionInfo,
  CommonParams,
  ProviderParams,
  SearchSidebarResult,
} from '$lib/types';
import type { ChatEventHandler, AppPaths } from '$lib/utils/invoke';
import { streamRequest } from './sse';

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`/api${path}`);
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error ?? res.statusText);
  }
  return res.json();
}

async function post<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`/api${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error ?? res.statusText);
  }
  return res.json();
}

async function patch<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`/api${path}`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error ?? res.statusText);
  }
  return res.json();
}

async function del<T>(path: string): Promise<T> {
  const res = await fetch(`/api${path}`, { method: 'DELETE' });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error ?? res.statusText);
  }
  return res.json();
}

export const webApi = {
  // Conversations
  createConversation(title: string, assistantId?: string, modelId?: string): Promise<Conversation> {
    return post('/conversations', { title, assistantId: assistantId ?? null, modelId: modelId ?? null });
  },
  listConversations(): Promise<Conversation[]> {
    return get('/conversations');
  },
  updateConversationTitle(id: string, title: string): Promise<void> {
    return patch(`/conversations/${id}/title`, { title });
  },
  deleteConversation(id: string): Promise<void> {
    return del(`/conversations/${id}`);
  },
  pinConversation(id: string, isPinned: boolean): Promise<void> {
    return patch(`/conversations/${id}/pin`, { isPinned });
  },
  updateConversationAssistant(id: string, assistantId: string | null): Promise<void> {
    return patch(`/conversations/${id}/assistant`, { assistantId });
  },
  generateConversationTitle(conversationId: string, modelId: string): Promise<string> {
    return post(`/conversations/${conversationId}/generate-title`, { modelId });
  },

  // Messages
  getMessages(conversationId: string, options: GetMessagesOptions = {}): Promise<PagedMessages> {
    const params = new URLSearchParams();
    if (options.limit) params.set('limit', String(options.limit));
    if (options.beforeMessageId) params.set('beforeMessageId', options.beforeMessageId);
    const qs = params.toString();
    return get(`/conversations/${conversationId}/messages${qs ? `?${qs}` : ''}`);
  },
  sendMessage(conversationId: string, content: string, modelId: string, onEvent: ChatEventHandler, commonParams?: CommonParams, providerParams?: ProviderParams): Promise<Message> {
    return new Promise((resolve, reject) => {
      let lastMessage: Message | null = null;
      streamRequest('/api/chat/send', { conversationId, content, modelId, commonParams: commonParams ?? null, providerParams: providerParams ?? null }, (event) => {
        onEvent(event);
        if (event.type === 'finished') {
          // message will be fetched by the caller after stream ends
        }
      }).then(() => resolve(lastMessage as unknown as Message)).catch(reject);
    });
  },
  sendMessageGroup(conversationId: string, content: string, modelIds: string[], onEvent: ChatEventHandler): Promise<Message[]> {
    return new Promise((resolve, reject) => {
      streamRequest('/api/chat/send-group', { conversationId, content, modelIds }, onEvent)
        .then(() => resolve([]))
        .catch(reject);
    });
  },
  resendMessage(conversationId: string, modelId: string, onEvent: ChatEventHandler, commonParams?: CommonParams, providerParams?: ProviderParams): Promise<Message> {
    return new Promise((resolve, reject) => {
      streamRequest('/api/chat/resend', { conversationId, modelId, commonParams: commonParams ?? null, providerParams: providerParams ?? null }, onEvent)
        .then(() => resolve({} as unknown as Message))
        .catch(reject);
    });
  },
  stopGeneration(): Promise<void> {
    return post('/chat/stop');
  },
  compressConversation(conversationId: string, modelId: string, onEvent: ChatEventHandler): Promise<Message[]> {
    return new Promise((resolve, reject) => {
      streamRequest('/api/chat/compress', { conversationId, modelId }, onEvent)
        .then(() => resolve([]))
        .catch(reject);
    });
  },
  deleteMessage(id: string): Promise<void> {
    return del(`/messages/${id}`);
  },
  restoreMessage(id: string): Promise<void> {
    return post(`/messages/${id}/restore`);
  },
  deleteMessagesAfter(conversationId: string, messageId: string): Promise<void> {
    return post(`/conversations/${conversationId}/messages/delete-after`, { messageId });
  },
  deleteMessagesFrom(conversationId: string, messageId: string): Promise<void> {
    return post(`/conversations/${conversationId}/messages/delete-from`, { messageId });
  },
  updateMessageContent(id: string, content: string): Promise<void> {
    return patch(`/messages/${id}/content`, { content });
  },
  getPasteBlobContent(pasteId: string): Promise<string> {
    return get(`/paste/${pasteId}`);
  },
  hydratePasteContent(content: string): Promise<string> {
    return post('/paste/hydrate', { content });
  },
  expandPasteContent(content: string): Promise<string> {
    return post('/paste/expand', { content });
  },

  // Versions
  generateVersion(conversationId: string, messageId: string, modelId: string, onEvent: ChatEventHandler, commonParams?: CommonParams, providerParams?: ProviderParams): Promise<Message> {
    return new Promise((resolve, reject) => {
      streamRequest('/api/chat/generate-version', { conversationId, messageId, modelId, commonParams: commonParams ?? null, providerParams: providerParams ?? null }, onEvent)
        .then(() => resolve({} as unknown as Message))
        .catch(reject);
    });
  },
  regenerateMessage(conversationId: string, messageId: string, modelId: string, onEvent: ChatEventHandler, commonParams?: CommonParams, providerParams?: ProviderParams): Promise<Message> {
    return new Promise((resolve, reject) => {
      streamRequest('/api/chat/regenerate', { conversationId, messageId, modelId, commonParams: commonParams ?? null, providerParams: providerParams ?? null }, onEvent)
        .then(() => resolve({} as unknown as Message))
        .catch(reject);
    });
  },
  switchVersion(versionGroupId: string, versionNumber: number): Promise<void> {
    return post(`/versions/${versionGroupId}/switch`, { versionNumber });
  },
  listVersions(versionGroupId: string): Promise<VersionInfo[]> {
    return get(`/versions/${versionGroupId}`);
  },
  listVersionMessages(versionGroupId: string): Promise<Message[]> {
    return get(`/versions/${versionGroupId}/messages`);
  },
  getVersionModels(versionGroupId: string): Promise<[number, string][]> {
    return get(`/versions/${versionGroupId}/models`);
  },

  // Providers
  addProvider(name: string, providerType: ProviderType, apiBase: string, apiKey?: string, enabled = true): Promise<ProviderConfig> {
    return post('/providers', { name, providerType, apiKey: apiKey ?? null, apiBase, enabled });
  },
  listProviders(): Promise<ProviderConfig[]> {
    return get('/providers');
  },
  updateProvider(id: string, name: string, providerType: ProviderType, apiBase: string, apiKey: string | null, enabled: boolean): Promise<ProviderConfig> {
    return patch(`/providers/${id}`, { name, providerType, apiBase, apiKey, enabled });
  },
  deleteProvider(id: string): Promise<void> {
    return del(`/providers/${id}`);
  },
  fetchModels(providerId: string): Promise<ModelInfo[]> {
    return post(`/providers/${providerId}/fetch-models`);
  },
  updateModelVisibility(modelId: string, enabled: boolean): Promise<void> {
    return patch(`/models/${modelId}/visibility`, { enabled });
  },
  updateProviderModelsVisibility(providerId: string, enabled: boolean): Promise<number> {
    return patch(`/providers/${providerId}/models/visibility`, { enabled });
  },

  // Assistants
  createAssistant(name: string, systemPrompt?: string, modelId?: string, temperature?: number, topP?: number, maxTokens?: number): Promise<Assistant> {
    return post('/assistants', { name, systemPrompt: systemPrompt ?? null, modelId: modelId ?? null, temperature: temperature ?? null, topP: topP ?? null, maxTokens: maxTokens ?? null });
  },
  listAssistants(): Promise<Assistant[]> {
    return get('/assistants');
  },
  updateAssistant(assistant: Assistant): Promise<void> {
    return patch(`/assistants/${assistant.id}`, { assistant });
  },
  deleteAssistant(id: string): Promise<void> {
    return del(`/assistants/${id}`);
  },

  // Search
  searchMessages(query: string): Promise<Message[]> {
    return get(`/search/messages?q=${encodeURIComponent(query)}`);
  },
  searchSidebarResults(query: string): Promise<SearchSidebarResult[]> {
    return get(`/search/sidebar?q=${encodeURIComponent(query)}`);
  },

  // Export
  exportMarkdown(conversationId: string): Promise<string> {
    return get(`/conversations/${conversationId}/export/markdown`);
  },
  exportJson(conversationId: string): Promise<string> {
    return get(`/conversations/${conversationId}/export/json`);
  },

  // Settings（跨平台部分）
  getAppPaths(): Promise<AppPaths> {
    return get('/settings/paths');
  },
  getCacheSize(): Promise<string> {
    return get('/settings/cache-size');
  },
  clearCache(): Promise<void> {
    return post('/settings/clear-cache');
  },
  resetAppData(): Promise<void> {
    return post('/settings/reset');
  },
  setProxyMode(mode: string): Promise<void> {
    return post('/settings/proxy-mode', { mode });
  },
  getProxyMode(): Promise<string> {
    return get('/settings/proxy-mode');
  },

  // 桌面专属（Web 端不支持）
  openPath(_path: string): Promise<void> {
    return Promise.reject(new Error('Not supported in web mode'));
  },
  pickDirectory(): Promise<string | null> {
    return Promise.reject(new Error('Not supported in web mode'));
  },
  localBackup(_destPath: string): Promise<void> {
    return Promise.reject(new Error('Not supported in web mode'));
  },
  getAutostartEnabled(): Promise<boolean> {
    return Promise.reject(new Error('Not supported in web mode'));
  },
  setAutostartEnabled(_enabled: boolean): Promise<void> {
    return Promise.reject(new Error('Not supported in web mode'));
  },
};
