<script lang="ts">
  import type { Message, ModelGroup } from '$lib/types';
  import MessageList from './MessageList.svelte';
  import InputArea from './InputArea.svelte';

  type ChatEvent = {
    type: 'send';
    content: string;
  };

  let {
    conversationId,
    modelId = $bindable(''),
    messages,
    modelGroups = [],
    disabled = false,
    disabledReason = '',
    suggestions = [],
    onEvent,
  }: {
    conversationId: string;
    modelId?: string;
    messages: Message[];
    modelGroups?: ModelGroup[];
    disabled?: boolean;
    disabledReason?: string;
    suggestions?: string[];
    onEvent?: (event: ChatEvent) => void;
  } = $props();

  function handleSend(content: string) {
    onEvent?.({ type: 'send', content });
  }
</script>

<div class="flex flex-col h-full">
  <div class="flex-1 overflow-y-auto">
    <MessageList {messages} />
  </div>

  <InputArea
    {disabled}
    {disabledReason}
    {suggestions}
    {modelGroups}
    bind:selectedModel={modelId}
    onSend={handleSend}
  />
</div>
