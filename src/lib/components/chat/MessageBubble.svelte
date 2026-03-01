<script lang="ts">
  import type { Message } from '$lib/types';
  import { renderMarkdown } from '$lib/utils/markdown';

  let { message }: { message: Message } = $props();

  let showReasoning = $state(false);
  let isUser = $derived(message.role === 'user');
  let renderedContent = $derived(renderMarkdown(message.content));
  let renderedReasoning = $derived(
    message.reasoning ? renderMarkdown(message.reasoning) : '',
  );
</script>

<div
  class="flex w-full mb-4"
  style="justify-content: {isUser ? 'flex-end' : 'flex-start'};"
>
  <div
    class="max-w-[75%] rounded-xl px-4 py-3 text-sm leading-relaxed"
    style="background-color: {isUser ? 'var(--msg-user)' : 'var(--msg-assistant)'}; border: 1px solid var(--border);"
  >
    {#if message.reasoning}
      <button
        onclick={() => (showReasoning = !showReasoning)}
        class="text-xs mb-2 cursor-pointer rounded px-2 py-0.5"
        style="background: none; border: 1px solid var(--border); color: var(--text-secondary);"
      >
        {showReasoning ? 'Hide' : 'Show'} reasoning
      </button>
      {#if showReasoning}
        <div
          class="mb-3 pb-3 text-xs"
          style="border-bottom: 1px solid var(--border); color: var(--text-secondary);"
        >
          {@html renderedReasoning}
        </div>
      {/if}
    {/if}

    <div class="prose-sm">
      {@html renderedContent}
    </div>

    {#if message.tokenCount}
      <div class="mt-2 text-xs" style="color: var(--text-secondary);">
        {message.tokenCount} tokens
      </div>
    {/if}
  </div>
</div>
