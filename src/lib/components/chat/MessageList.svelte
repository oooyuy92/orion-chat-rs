<script lang="ts">
  import type { Message } from '$lib/types';
  import MessageBubble from './MessageBubble.svelte';
  import { i18n } from '$lib/stores/i18n.svelte';

  type MessageAction =
    | { type: 'delete'; messageId: string }
    | { type: 'resend'; messageId: string }
    | { type: 'editResend'; messageId: string; content: string }
    | { type: 'regenerate'; messageId: string; modelId: string | null }
    | { type: 'generateVersion'; messageId: string }
    | { type: 'switchVersion'; versionGroupId: string; versionNumber: number };

  let {
    messages,
    onAction,
    disabled = false,
  }: {
    messages: Message[];
    onAction?: (action: MessageAction) => void;
    disabled?: boolean;
  } = $props();

  let container: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (messages.length > 0 && container) {
      container.scrollTop = container.scrollHeight;
    }
  });
</script>

<div class="conversation-viewport" bind:this={container}>
  <div class="conversation-stack">
    {#if messages.length === 0}
      <div class="empty-message-state">
        <h2>{i18n.t.noMessagesYet}</h2>
        <p>{i18n.t.askAnything}</p>
      </div>
    {:else}
      {#each messages as message (message.id)}
        <MessageBubble {message} {onAction} {disabled} />
      {/each}
    {/if}
  </div>
</div>

<style>
  .conversation-viewport {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: oklch(0.85 0 0) transparent;
  }

  .conversation-viewport::-webkit-scrollbar {
    width: 2px;
  }

  .conversation-viewport::-webkit-scrollbar-track {
    background: transparent;
  }

  .conversation-viewport::-webkit-scrollbar-thumb {
    background: oklch(0.85 0 0);
    border-radius: 9999px;
  }

  .conversation-viewport::-webkit-scrollbar-thumb:hover {
    background: oklch(0.72 0 0);
  }

  .conversation-stack {
    width: min(56rem, 100%);
    margin: 0 auto;
    padding: 1rem 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
    box-sizing: border-box;
  }

  .empty-message-state {
    border: 1px dashed var(--border);
    border-radius: 0.9rem;
    background: var(--card);
    padding: 1rem 1.1rem;
  }

  .empty-message-state h2 {
    margin: 0;
    font-size: 0.95rem;
    color: var(--foreground);
    font-weight: 620;
  }

  .empty-message-state p {
    margin: 0.35rem 0 0;
    font-size: 0.84rem;
    color: var(--muted-foreground);
  }

  @media (max-width: 640px) {
    .conversation-stack {
      padding: 0.75rem 0.7rem 1rem;
      gap: 0.95rem;
    }
  }
</style>
