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
  versionGroupId: string | null;
  versionNumber: number;
  totalVersions: number;
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
  providerType: ProviderType;
  models: ModelInfo[];
}

// -- Model parameter types (mirrors Rust serde structs) --

export interface CommonParams {
  temperature?: number | null;
  topP?: number | null;
  maxTokens?: number | null;
}

export type AnthropicThinking =
  | { type: 'adaptive' }
  | { type: 'enabled'; budgetTokens: number }
  | { type: 'disabled' };

export type AnthropicEffort = 'low' | 'medium' | 'high';
export type ReasoningEffort = 'low' | 'medium' | 'high';
export type GeminiThinkingLevel = 'low' | 'medium' | 'high';

export type ProviderParams =
  | {
      provider_type: 'openaiCompat';
      frequencyPenalty?: number | null;
      presencePenalty?: number | null;
      reasoningEffort?: ReasoningEffort | null;
      seed?: number | null;
      maxCompletionTokens?: number | null;
    }
  | {
      provider_type: 'anthropic';
      topK?: number | null;
      thinking?: AnthropicThinking | null;
      effort?: AnthropicEffort | null;
    }
  | {
      provider_type: 'gemini';
      thinkingBudget?: number | null;
      thinkingLevel?: GeminiThinkingLevel | null;
    }
  | {
      provider_type: 'ollama';
      think?: boolean | null;
      numCtx?: number | null;
      repeatPenalty?: number | null;
      minP?: number | null;
      keepAlive?: string | null;
    };

export interface ModelParams {
  common: CommonParams;
  providerParams: ProviderParams;
}

export interface VersionInfo {
  versionNumber: number;
  modelId: string | null;
  id: string;
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
