<script lang="ts">
  import type { Assistant, Message, ModelGroup } from '$lib/types';
  import { i18n } from '$lib/stores/i18n.svelte';
  import { api } from '$lib/utils/invoke';
  import AssistantTabs from './AssistantTabs.svelte';
  import MessageList from './MessageList.svelte';
  import type { ScrollState } from './MessageList.svelte';
  import MessageNavRail from './MessageNavRail.svelte';
  import InputArea from './InputArea.svelte';
  import VersionCompareView from './VersionCompareView.svelte';

  type MessageAction =
    | { type: 'delete'; messageId: string }
    | { type: 'resend'; messageId: string }
    | { type: 'editResend'; messageId: string; content: string }
    | { type: 'regenerate'; messageId: string; modelId: string | null }
    | { type: 'generateVersion'; messageId: string }
    | { type: 'switchVersion'; versionGroupId: string; versionNumber: number }
    | { type: 'expandVersions'; versionGroupId: string }
    | { type: 'fork'; messageId: string };

  type ChatEvent =
    | { type: 'send'; content: string }
    | { type: 'groupSend'; content: string; modelIds: string[] }
    | { type: 'stop' }
    | { type: 'exitGroupCompare' }
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
    groupStreamingMessages = [],
    selectedModelId = $bindable(''),
    onAssistantSelect,
    onModelSelect,
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
    groupStreamingMessages?: Message[];
    selectedModelId?: string;
    onAssistantSelect?: (assistantId: string | null) => void;
    onModelSelect?: (modelId: string) => void;
    onLoadOlderMessages?: () => void | Promise<void>;
    onEvent?: (event: ChatEvent) => void;
  } = $props();

  let compareVersionGroupId = $state<string | null>(null);
  let compareMessages = $state<Message[]>([]);
  let headerCollapsed = $state(false);
  let messageListRef = $state<ReturnType<typeof MessageList> | undefined>();
  let navScrollState = $state<ScrollState>({
    scrollTop: 0,
    viewportHeight: 0,
    totalHeight: 0,
    heightCache: new Map(),
    estimatedRowHeight: 180,
  });

  const HEADER_COLLAPSE_THRESHOLD = 32;

  $effect(() => {
    // Reset compare mode when conversation changes
    void conversationId;
    compareVersionGroupId = null;
    compareMessages = [];
  });

  function handleSend(content: string) {
    onEvent?.({ type: 'send', content });
  }

  function handleGroupSend(content: string, modelIds: string[]) {
    onEvent?.({ type: 'groupSend', content, modelIds });
  }

  function handleStop() {
    onEvent?.({ type: 'stop' });
  }

  async function handleAction(action: MessageAction) {
    console.log('[ChatArea] handleAction:', action);
    if (action.type === 'expandVersions') {
      try {
        compareMessages = await api.listVersionMessages(action.versionGroupId);
        compareVersionGroupId = action.versionGroupId;
      } catch (e) {
        console.error('Failed to load version messages:', e);
      }
      return;
    }
    if (action.type === 'fork') {
      onEvent?.(action);
      return;
    }
    onEvent?.(action);
  }

  function handleBackFromCompare() {
    compareVersionGroupId = null;
    compareMessages = [];
  }

  function handleScrollTopChange(st: number) {
    headerCollapsed = st > HEADER_COLLAPSE_THRESHOLD;
  }

  function handleScrollStateChange(state: ScrollState) {
    navScrollState = state;
  }
</script>

<div class="flex flex-col h-full min-h-0">
  {#if groupStreamingMessages.length > 0}
    <VersionCompareView
      versionMessages={groupStreamingMessages}
      {modelGroups}
      onBack={() => onEvent?.({ type: 'exitGroupCompare' })}
    />
  {:else if compareVersionGroupId}
    <VersionCompareView
      versionMessages={compareMessages}
      {modelGroups}
      onBack={handleBackFromCompare}
    />
  {:else}
    <div class="chat-header" class:collapsed={headerCollapsed}>
      <AssistantTabs
        {assistants}
        {selectedAssistantId}
        disabled={assistantSelectionLocked}
        onSelect={onAssistantSelect}
      />
    </div>

    <div class="flex-1 min-h-0 flex">
      <MessageList bind:this={messageListRef} {conversationId} {messages} {disabled} {hasMoreMessages} {isLoadingMoreMessages} {canLoadOlderMessages} {focusedMessageId} onLoadOlder={onLoadOlderMessages} onAction={handleAction} onScrollTopChange={handleScrollTopChange} onScrollStateChange={handleScrollStateChange} />
      {#if messages.length > 0}
        <MessageNavRail
          {messages}
          scrollTop={navScrollState.scrollTop}
          viewportHeight={navScrollState.viewportHeight}
          totalHeight={navScrollState.totalHeight}
          heightCache={navScrollState.heightCache}
          estimatedRowHeight={navScrollState.estimatedRowHeight}
          onJumpToMessage={(index) => messageListRef?.jumpToIndex(index)}
          onJumpToTop={() => messageListRef?.jumpToTop()}
          onJumpToBottom={() => messageListRef?.jumpToBottom()}
        />
      {/if}
    </div>

    <InputArea
      {disabled}
      {disabledReason}
      {conversationId}
      {suggestions}
      {modelGroups}
      bind:selectedModelId
      {onModelSelect}
      onSend={handleSend}
      onGroupSend={handleGroupSend}
      onStop={handleStop}
    />
  {/if}
</div>

<style>
  .chat-header {
    border-bottom: 1px solid var(--border);
    background: var(--card);
    padding: 0.75rem 1rem 0.5rem;
    flex-shrink: 0;
    overflow: hidden;
    max-height: 6rem;
    transition: max-height 0.25s ease, padding 0.25s ease, border-color 0.25s ease, opacity 0.2s ease;
  }
  .chat-header.collapsed {
    max-height: 0;
    padding-top: 0;
    padding-bottom: 0;
    border-color: transparent;
    opacity: 0;
    pointer-events: none;
  }
</style>
