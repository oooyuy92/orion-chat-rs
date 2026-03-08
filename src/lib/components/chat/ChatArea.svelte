<script lang="ts">
  import type { Assistant, Message, ModelGroup } from '$lib/types';
  import { i18n } from '$lib/stores/i18n.svelte';
  import AssistantTabs from './AssistantTabs.svelte';
  import MessageList from './MessageList.svelte';
  import InputArea from './InputArea.svelte';

  type MessageAction =
    | { type: 'delete'; messageId: string }
    | { type: 'resend'; messageId: string }
    | { type: 'editResend'; messageId: string; content: string }
    | { type: 'regenerate'; messageId: string; modelId: string | null }
    | { type: 'generateVersion'; messageId: string }
    | { type: 'switchVersion'; versionGroupId: string; versionNumber: number };

  type ChatEvent =
    | { type: 'send'; content: string }
    | { type: 'stop' }
    | MessageAction;

  let {
    conversationId,
    messages,
    disabled = false,
    disabledReason = '',
    suggestions = [],
    modelGroups = [],
    assistants = [],
    selectedAssistantId = null,
    assistantSelectionLocked = false,
    hasMoreMessages = false,
    isLoadingMoreMessages = false,
    canLoadOlderMessages = true,
    focusedMessageId = null,
    selectedModelId = $bindable(''),
    onAssistantSelect,
    onLoadOlderMessages,
    onEvent,
  }: {
    conversationId: string;
    messages: Message[];
    disabled?: boolean;
    disabledReason?: string;
    suggestions?: string[];
    modelGroups?: ModelGroup[];
    assistants?: Assistant[];
    selectedAssistantId?: string | null;
    assistantSelectionLocked?: boolean;
    hasMoreMessages?: boolean;
    isLoadingMoreMessages?: boolean;
    canLoadOlderMessages?: boolean;
    focusedMessageId?: string | null;
    selectedModelId?: string;
    onAssistantSelect?: (assistantId: string | null) => void;
    onLoadOlderMessages?: () => void | Promise<void>;
    onEvent?: (event: ChatEvent) => void;
  } = $props();

  function handleSend(content: string) {
    onEvent?.({ type: 'send', content });
  }

  function handleStop() {
    onEvent?.({ type: 'stop' });
  }

  function handleAction(action: MessageAction) {
    console.log('[ChatArea] handleAction:', action);
    onEvent?.(action);
  }
</script>

<div class="flex flex-col h-full min-h-0">
  <div class="chat-header">
    <AssistantTabs
      {assistants}
      {selectedAssistantId}
      disabled={assistantSelectionLocked}
      onSelect={onAssistantSelect}
    />
    {#if assistantSelectionLocked}
      <p class="assistant-lock-note">{i18n.t.assistantLocked}</p>
    {/if}
  </div>

  <div class="flex-1 min-h-0 flex">
    <MessageList {conversationId} {messages} {disabled} {hasMoreMessages} {isLoadingMoreMessages} {canLoadOlderMessages} {focusedMessageId} onLoadOlder={onLoadOlderMessages} onAction={handleAction} />
  </div>

  <InputArea
    {disabled}
    {disabledReason}
    {suggestions}
    {modelGroups}
    bind:selectedModelId
    onSend={handleSend}
    onStop={handleStop}
  />
</div>

<style>
  .chat-header {
    border-bottom: 1px solid var(--border);
    background: var(--card);
    padding: 0.75rem 1rem 0.5rem;
    flex-shrink: 0;
  }

  .assistant-lock-note {
    margin: 0.35rem 0 0;
    color: var(--muted-foreground);
    font-size: 0.8rem;
  }
</style>
