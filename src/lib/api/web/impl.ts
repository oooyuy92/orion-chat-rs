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

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`/api${path}`);
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error ?? res.statusText);
  }
  const text = await res.text();
  if (!text) {
    throw new Error(`Empty response from GET ${path}`);
  }
  try {
    return JSON.parse(text);
  } catch {
    throw new Error(`Invalid JSON from GET ${path}: ${text.slice(0, 200)}`);
  }
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
  const text = await res.text();
  if (!text) {
    throw new Error(`Empty response from POST ${path}`);
  }
  try {
    return JSON.parse(text);
  } catch {
    throw new Error(`Invalid JSON from POST ${path}: ${text.slice(0, 200)}`);
  }
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
  const text = await res.text();
  if (!text) {
    throw new Error(`Empty response from PATCH ${path}`);
  }
  try {
    return JSON.parse(text);
  } catch {
    throw new Error(`Invalid JSON from PATCH ${path}: ${text.slice(0, 200)}`);
  }
}

async function del<T>(path: string): Promise<T> {
  const res = await fetch(`/api${path}`, { method: 'DELETE' });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error ?? res.statusText);
  }
  const text = await res.text();
  if (!text) {
    throw new Error(`Empty response from DELETE ${path}`);
  }
  try {
    return JSON.parse(text);
  } catch {
    throw new Error(`Invalid JSON from DELETE ${path}: ${text.slice(0, 200)}`);
  }
}

async function postVoid(path: string, body?: unknown): Promise<void> {
  const res = await fetch(`/api${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error ?? res.statusText);
  }
}

async function patchVoid(path: string, body?: unknown): Promise<void> {
  const res = await fetch(`/api${path}`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error ?? res.statusText);
  }
}

async function delVoid(path: string): Promise<void> {
  const res = await fetch(`/api${path}`, { method: 'DELETE' });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error ?? res.statusText);
  }
}

/**
 * Poll for message updates until completion.
 * Used for tracking assistant message generation progress.
 */
async function pollMessage(
  conversationId: string,
  messageId: string,
  onEvent: ChatEventHandler
): Promise<Message> {
  const POLL_INTERVAL = 500; // 500ms
  const TIMEOUT = 5 * 60 * 1000; // 5 minutes
  const startTime = Date.now();

  let lastContent = '';
  let lastReasoning = '';
  let isDone = false;

  onEvent({ type: 'started', messageId });

  while (!isDone) {
    if (Date.now() - startTime > TIMEOUT) {
      onEvent({ type: 'error', messageId, message: 'Polling timeout after 5 minutes' });
      throw new Error('Polling timeout');
    }

    try {
      const pagedMessages = await get<PagedMessages>(`/conversations/${conversationId}/messages?limit=10`);
      const latestMessage = pagedMessages.messages.find(m => m.id === messageId);

      if (!latestMessage) {
        await new Promise(resolve => setTimeout(resolve, POLL_INTERVAL));
        continue;
      }

      // Check for content delta
      if (latestMessage.content !== lastContent) {
        const delta = latestMessage.content.slice(lastContent.length);
        if (delta) {
          onEvent({ type: 'delta', messageId, content: delta });
          lastContent = latestMessage.content;
        }
      }

      // Check for reasoning delta
      if (latestMessage.reasoning && latestMessage.reasoning !== lastReasoning) {
        const delta = latestMessage.reasoning.slice(lastReasoning.length);
        if (delta) {
          onEvent({ type: 'reasoning', messageId, content: delta });
          lastReasoning = latestMessage.reasoning;
        }
      }

      // Check if done
      if (latestMessage.status === 'done') {
        if (latestMessage.tokenCount !== null) {
          const totalTokens = latestMessage.tokenCount;
          const estimatedPromptTokens = Math.floor(totalTokens * 0.3);
          const estimatedCompletionTokens = totalTokens - estimatedPromptTokens;
          onEvent({
            type: 'usage',
            messageId,
            promptTokens: estimatedPromptTokens,
            completionTokens: estimatedCompletionTokens
          });
        }
        onEvent({ type: 'finished', messageId });
        return latestMessage;
      }

      // Check for error status
      if (latestMessage.status === 'error') {
        onEvent({ type: 'error', messageId, message: 'Message generation failed' });
        throw new Error('Message generation failed');
      }

    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      onEvent({ type: 'error', messageId, message: errorMessage });
      throw error;
    }

    await new Promise(resolve => setTimeout(resolve, POLL_INTERVAL));
  }

  throw new Error('Polling ended without completion');
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
    return patchVoid(`/conversations/${id}/title`, { title });
  },
  deleteConversation(id: string): Promise<void> {
    return delVoid(`/conversations/${id}`);
  },
  pinConversation(id: string, isPinned: boolean): Promise<void> {
    return patchVoid(`/conversations/${id}/pin`, { isPinned });
  },
  updateConversationAssistant(id: string, assistantId: string | null): Promise<void> {
    return patchVoid(`/conversations/${id}/assistant`, { assistantId });
  },
  updateConversationModel(id: string, modelId: string | null): Promise<void> {
    return patchVoid(`/conversations/${id}/model`, { modelId });
  },
  generateConversationTitle(conversationId: string, modelId: string): Promise<string> {
    return post(`/conversations/${conversationId}/generate-title`, { modelId });
  },
  forkConversation(sourceConversationId: string, upToMessageId: string): Promise<Conversation> {
    return post(`/conversations/${sourceConversationId}/fork`, { upToMessageId });
  },

  // Messages
  getMessages(conversationId: string, options: GetMessagesOptions = {}): Promise<PagedMessages> {
    const params = new URLSearchParams();
    if (options.limit) params.set('limit', String(options.limit));
    if (options.beforeMessageId) params.set('beforeMessageId', options.beforeMessageId);
    const qs = params.toString();
    return get(`/conversations/${conversationId}/messages${qs ? `?${qs}` : ''}`);
  },
  async sendMessage(conversationId: string, content: string, modelId: string, onEvent: ChatEventHandler, _commonParams?: CommonParams, _providerParams?: ProviderParams): Promise<Message> {
    const assistantMessage = await post<Message>(`/conversations/${conversationId}/messages`, {
      content,
      modelId,
    });
    if (!assistantMessage?.id) {
      throw new Error(`Invalid response from sendMessage: ${JSON.stringify(assistantMessage)}`);
    }
    return pollMessage(conversationId, assistantMessage.id, onEvent);
  },
  async sendMessageGroup(conversationId: string, content: string, modelIds: string[], onEvent: ChatEventHandler): Promise<Message[]> {
    const messages = await post<Message[]>(
      `/conversations/${conversationId}/messages/group`,
      { messages: [{ content }], modelId: modelIds[0] }
    );
    if (!Array.isArray(messages)) {
      throw new Error(`Invalid response from sendMessageGroup: ${JSON.stringify(messages)}`);
    }
    return Promise.all(
      messages.filter(m => m.role === 'assistant')
        .map(m => pollMessage(conversationId, m.id, onEvent))
    );
  },
  async resendMessage(conversationId: string, _modelId: string, onEvent: ChatEventHandler, _commonParams?: CommonParams, _providerParams?: ProviderParams): Promise<Message> {
    const assistantMessage = await post<Message>(`/conversations/${conversationId}/resend`);
    if (!assistantMessage?.id) {
      throw new Error(`Invalid response from resendMessage: ${JSON.stringify(assistantMessage)}`);
    }
    return pollMessage(conversationId, assistantMessage.id, onEvent);
  },
  stopGeneration(conversationId: string): Promise<void> {
    return postVoid(`/conversations/${conversationId}/stop`);
  },
  async compressConversation(conversationId: string, _modelId: string, onEvent: ChatEventHandler): Promise<Message[]> {
    const message = await post<Message>(`/conversations/${conversationId}/compress`);
    if (!message?.id) {
      throw new Error(`Invalid response from compressConversation: ${JSON.stringify(message)}`);
    }
    return [await pollMessage(conversationId, message.id, onEvent)];
  },
  deleteMessage(id: string): Promise<void> {
    return delVoid(`/messages/${id}`);
  },
  restoreMessage(id: string): Promise<void> {
    return postVoid(`/messages/${id}/restore`);
  },
  deleteMessagesAfter(conversationId: string, messageId: string): Promise<void> {
    return postVoid(`/conversations/${conversationId}/messages/delete-after`, { messageId });
  },
  deleteMessagesFrom(conversationId: string, messageId: string): Promise<void> {
    return postVoid(`/conversations/${conversationId}/messages/delete-from`, { messageId });
  },
  updateMessageContent(id: string, content: string): Promise<void> {
    return patchVoid(`/messages/${id}/content`, { content });
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
  async generateVersion(conversationId: string, messageId: string, _modelId: string, onEvent: ChatEventHandler, _commonParams?: CommonParams, _providerParams?: ProviderParams): Promise<Message> {
    const newMessage = await post<Message>(`/messages/${messageId}/generate-version`);
    if (!newMessage?.id) {
      throw new Error(`Invalid response from generateVersion: ${JSON.stringify(newMessage)}`);
    }
    return pollMessage(conversationId, newMessage.id, onEvent);
  },
  async regenerateMessage(conversationId: string, messageId: string, _modelId: string, onEvent: ChatEventHandler, _commonParams?: CommonParams, _providerParams?: ProviderParams): Promise<Message> {
    const newMessage = await post<Message>(`/messages/${messageId}/regenerate`);
    if (!newMessage?.id) {
      throw new Error(`Invalid response from regenerateMessage: ${JSON.stringify(newMessage)}`);
    }
    return pollMessage(conversationId, newMessage.id, onEvent);
  },
  switchVersion(versionGroupId: string, versionNumber: number): Promise<void> {
    return postVoid(`/messages/${versionGroupId}/switch-version`, { versionNumber });
  },
  listVersions(versionGroupId: string): Promise<VersionInfo[]> {
    return get(`/messages/${versionGroupId}/versions`);
  },
  listVersionMessages(versionGroupId: string): Promise<Message[]> {
    return get(`/messages/${versionGroupId}/version-messages`);
  },
  getVersionModels(versionGroupId: string): Promise<[number, string][]> {
    return get(`/messages/${versionGroupId}/version-models`);
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
    return delVoid(`/providers/${id}`);
  },
  fetchModels(providerId: string): Promise<ModelInfo[]> {
    return post(`/providers/${providerId}/fetch-models`);
  },
  createManualModel(providerId: string, requestName: string, displayName: string | null, enabled: boolean): Promise<ModelInfo> {
    return post(`/providers/${providerId}/models`, { requestName, displayName, enabled });
  },
  updateManualModel(modelId: string, requestName: string, displayName: string | null, enabled: boolean): Promise<ModelInfo> {
    return patch(`/models/${modelId}`, { requestName, displayName, enabled });
  },
  deleteManualModel(modelId: string): Promise<void> {
    return delVoid(`/models/${modelId}`);
  },
  updateModelVisibility(modelId: string, enabled: boolean): Promise<void> {
    return patchVoid(`/models/${modelId}/visibility`, { enabled });
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
    return patchVoid(`/assistants/${assistant.id}`, assistant);
  },
  deleteAssistant(id: string): Promise<void> {
    return delVoid(`/assistants/${id}`);
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
    return postVoid('/settings/clear-cache');
  },
  resetAppData(): Promise<void> {
    return postVoid('/settings/reset');
  },
  setProxyMode(mode: string): Promise<void> {
    return postVoid('/settings/proxy', { mode });
  },
  getProxyMode(): Promise<string> {
    return get('/settings/proxy');
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
