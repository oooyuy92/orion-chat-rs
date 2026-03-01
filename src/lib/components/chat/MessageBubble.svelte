<script lang="ts">
  import type { Message } from '$lib/types';
  import { renderMarkdown } from '$lib/utils/markdown';

  let { message }: { message: Message } = $props();

  let showReasoning = $state(false);
  let isUser = $derived(message.role === 'user');
  let renderedContent = $derived(renderMarkdown(message.content));
  let renderedReasoning = $derived(message.reasoning ? renderMarkdown(message.reasoning) : '');
</script>

<div class="message-row" class:is-user={isUser}>
  <div class="message-group" class:is-user={isUser} class:is-assistant={!isUser}>
    {#if message.reasoning}
      <button class="reasoning-toggle" onclick={() => (showReasoning = !showReasoning)}>
        {showReasoning ? 'Hide reasoning' : 'Thought process'}
      </button>

      {#if showReasoning}
        <div class="reasoning-panel">
          <div class="reasoning-markdown">{@html renderedReasoning}</div>
        </div>
      {/if}
    {/if}

    <div class="message-surface" class:user-surface={isUser} class:assistant-surface={!isUser}>
      <div class="message-markdown">{@html renderedContent}</div>
    </div>

    {#if message.tokenCount}
      <div class="message-meta">{message.tokenCount} tokens</div>
    {/if}

    {#if message.status === 'error'}
      <div class="message-error">Message generation failed.</div>
    {/if}
  </div>
</div>

<style>
  .message-row {
    display: flex;
    width: 100%;
    justify-content: flex-start;
  }

  .message-row.is-user {
    justify-content: flex-end;
  }

  .message-group {
    max-width: 95%;
    display: flex;
    flex-direction: column;
    gap: 0.42rem;
  }

  .message-group.is-user {
    align-items: flex-end;
  }

  .message-surface {
    min-width: 0;
    font-size: 0.9rem;
    line-height: 1.55;
    color: var(--foreground);
  }

  .message-surface.user-surface {
    width: fit-content;
    max-width: 100%;
    border-radius: 0.7rem;
    background: var(--muted);
    padding: 0.65rem 0.85rem;
  }

  .message-surface.assistant-surface {
    width: 100%;
    padding: 0;
  }

  .reasoning-toggle {
    width: fit-content;
    border: 1px solid var(--border);
    border-radius: 9999px;
    background: var(--background);
    color: var(--muted-foreground);
    font-size: 0.74rem;
    padding: 0.24rem 0.62rem;
    cursor: pointer;
  }

  .reasoning-toggle:hover {
    background: var(--muted);
    color: var(--foreground);
  }

  .reasoning-panel {
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    background: var(--muted);
    padding: 0.65rem 0.8rem;
    font-size: 0.78rem;
    color: var(--muted-foreground);
  }

  .message-meta {
    color: var(--muted-foreground);
    font-size: 0.72rem;
    padding: 0 0.1rem;
  }

  .message-error {
    color: #b91c1c;
    font-size: 0.74rem;
  }

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
