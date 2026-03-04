<script lang="ts">
  import type { Message } from '$lib/types';
  import { renderMarkdown } from '$lib/utils/markdown';

  type Props = {
    message: Message;
  };

  let { message }: Props = $props();

  const isUser = $derived(message.role === 'user');
  const markdownContent = $derived(
    message.role === 'assistant' ? renderMarkdown(message.content) : message.content,
  );
  const renderedReasoning = $derived(message.reasoning ? renderMarkdown(message.reasoning) : '');

  let showReasoning = $state(false);
</script>

{#if isUser}
  <!-- 用户消息：右对齐气泡，浅灰背景 -->
  <div class="group flex w-full max-w-[95%] ml-auto justify-end">
    <div class="flex w-fit max-w-full flex-col gap-2">
      <div class="rounded-lg bg-secondary px-4 py-3 text-sm text-foreground">
        {message.content}
      </div>

      {#if message.tokenCount}
        <div class="text-xs text-muted-foreground px-1">{message.tokenCount} tokens</div>
      {/if}

      {#if message.status === 'error'}
        <div class="text-xs text-destructive px-1">Message generation failed.</div>
      {/if}
    </div>
  </div>
{:else}
  <!-- 助手消息：左对齐纯文本，无背景 -->
  <div class="group flex w-full max-w-[95%]">
    <div class="flex w-fit max-w-full flex-col gap-2">
      {#if message.reasoning}
        <button
          class="w-fit rounded-full border border-border bg-background px-3 py-1 text-xs text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
          onclick={() => (showReasoning = !showReasoning)}
        >
          {showReasoning ? 'Hide reasoning' : 'Thought process'}
        </button>

        {#if showReasoning}
          <div class="rounded-xl border border-border bg-muted px-3 py-2.5 text-xs text-muted-foreground">
            <div class="reasoning-markdown">{@html renderedReasoning}</div>
          </div>
        {/if}
      {/if}

      <div class="text-sm text-foreground">
        <div class="message-markdown">{@html markdownContent}</div>
      </div>

      {#if message.tokenCount}
        <div class="text-xs text-muted-foreground px-1">{message.tokenCount} tokens</div>
      {/if}

      {#if message.status === 'error'}
        <div class="text-xs text-destructive px-1">Message generation failed.</div>
      {/if}
    </div>
  </div>
{/if}

<style>
  :global(.message-markdown > :first-child) {
    margin-top: 0;
  }

  :global(.message-markdown > :last-child) {
    margin-bottom: 0;
  }

  :global(.message-markdown p) {
    margin: 0 0 0.55rem;
  }

  :global(.message-markdown h1),
  :global(.message-markdown h2),
  :global(.message-markdown h3) {
    margin: 0.7rem 0 0.45rem;
    font-weight: 680;
    line-height: 1.3;
  }

  :global(.message-markdown h1) {
    font-size: 1.45rem;
  }

  :global(.message-markdown h2) {
    font-size: 1.2rem;
  }

  :global(.message-markdown h3) {
    font-size: 1.04rem;
  }

  :global(.message-markdown ul),
  :global(.message-markdown ol) {
    margin: 0.35rem 0 0.55rem;
    padding-left: 1.2rem;
  }

  :global(.message-markdown li) {
    margin: 0.22rem 0;
  }

  :global(.message-markdown code) {
    background: var(--muted);
    border: 1px solid var(--border);
    border-radius: 0.33rem;
    padding: 0.05rem 0.3rem;
    font-size: 0.82em;
  }

  :global(.message-markdown pre) {
    overflow-x: auto;
    background: #f5f5f5;
    border: 1px solid var(--border);
    border-radius: 0.65rem;
    padding: 0.7rem;
    margin: 0.65rem 0;
  }

  :global(.message-markdown pre code) {
    background: transparent;
    border: none;
    padding: 0;
    border-radius: 0;
  }

  :global(.reasoning-markdown p:last-child) {
    margin-bottom: 0;
  }
</style>
