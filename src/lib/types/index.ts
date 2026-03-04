export type Role = 'system' | 'user' | 'assistant';

export type MessageStatus = 'streaming' | 'done' | 'error';

export interface Message {
  id: string;
  conversationId: string;
  role: Role;
  content: string;
  reasoning: string | null;
  modelId: string | null;
  status: MessageStatus;
  tokenCount: number | null;
  createdAt: string;
}

export interface Conversation {
  id: string;
  title: string;
  assistantId: string | null;
  modelId: string | null;
  isPinned: boolean;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export type ProviderType = 'openaiCompat' | 'anthropic' | 'gemini' | 'ollama';

export interface ModelInfo {
  id: string;
  name: string;
  providerId: string;
  contextLength: number | null;
  supportsVision: boolean;
  supportsStreaming: boolean;
  enabled: boolean;
}

export interface ModelGroup {
  providerId: string;
  providerName: string;
  models: ModelInfo[];
}

export interface ProviderConfig {
  id: string;
  name: string;
  providerType: ProviderType;
  apiBase: string;
  apiKey: string | null;
  models: ModelInfo[];
  enabled: boolean;
}

export interface Assistant {
  id: string;
  name: string;
  icon: string | null;
  systemPrompt: string | null;
  modelId: string | null;
  temperature: number | null;
  topP: number | null;
  maxTokens: number | null;
  extraParams: Record<string, unknown>;
  sortOrder: number;
  createdAt: string;
}
