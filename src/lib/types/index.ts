export type Role = 'system' | 'user' | 'assistant';

export type MessageStatus = 'streaming' | 'done' | 'error';
export type MessageType = 'text' | 'toolCall' | 'toolResult';

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
  messageType: MessageType;
  toolCallId?: string;
  toolName?: string;
  toolInput?: string;
  toolError: boolean;
}

export interface PagedMessages {
  messages: Message[];
  hasMore: boolean;
}

export interface GetMessagesOptions {
  limit?: number;
  beforeMessageId?: string | null;
}

export type ChatEvent =
  | { type: 'started'; messageId: string }
  | { type: 'delta'; messageId: string; content: string }
  | { type: 'reasoning'; messageId: string; content: string }
  | { type: 'usage'; messageId: string; promptTokens: number; completionTokens: number }
  | { type: 'finished'; messageId: string }
  | { type: 'error'; messageId: string; message: string }
  | { type: 'toolCallStart'; messageId: string; toolCallId: string; toolName: string; args: string }
  | {
      type: 'toolCallUpdate';
      messageId: string;
      toolCallId: string;
      partialResult: string;
    }
  | {
      type: 'toolCallEnd';
      messageId: string;
      toolCallId: string;
      result: string;
      isError: boolean;
    }
  | { type: 'toolAuthRequest'; toolCallId: string; toolName: string; args: string };

export type PermissionLevel = 'auto' | 'ask' | 'deny';
export type AuthAction = 'allow' | 'allowSession' | 'deny';

export interface ToolPermissions {
  [toolName: string]: PermissionLevel;
}

export interface ToolCallState {
  toolCallId: string;
  toolName: string;
  args: string;
  status: 'running' | 'completed' | 'error';
  result?: string;
  messageId: string;
  startTime: number;
  endTime?: number;
}


export interface SearchSidebarResult {
  conversationId: string;
  messageId: string | null;
  snippet: string;
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
  workingDirs: string[];
}

export type ProviderType = 'openaiCompat' | 'anthropic' | 'gemini' | 'ollama';
export type ModelSource = 'synced' | 'manual';

export interface ModelInfo {
  id: string;
  name: string;
  requestName: string;
  displayName: string | null;
  providerId: string;
  contextLength: number | null;
  supportsVision: boolean;
  supportsStreaming: boolean;
  enabled: boolean;
  source: ModelSource;
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

export type AssistantExtraParams = ProviderParams | Record<string, never>;

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
  extraParams: AssistantExtraParams;
  sortOrder: number;
  createdAt: string;
}

export interface ModelCombo {
  id: string;
  name: string;
  modelIds: string[];
}
