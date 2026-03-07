<script lang="ts">
  import type { Message, ModelGroup } from '$lib/types';
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
    selectedModelId = $bindable(''),
    onEvent,
  }: {
    conversationId: string;
    messages: Message[];
    disabled?: boolean;
    disabledReason?: string;
    suggestions?: string[];
    modelGroups?: ModelGroup[];
    selectedModelId?: string;
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

<div class="flex flex-col h-full">
  <div class="flex-1 overflow-y-auto">
    <MessageList {messages} {disabled} onAction={handleAction} />
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
