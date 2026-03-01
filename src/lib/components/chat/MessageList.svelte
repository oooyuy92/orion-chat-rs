<script lang="ts">
  import type { Message } from '$lib/types';
  import MessageBubble from './MessageBubble.svelte';

  let { messages }: { messages: Message[] } = $props();

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
        <h2>No messages yet</h2>
        <p>Ask anything to start the conversation.</p>
      </div>
    {:else}
      {#each messages as message (message.id)}
        <MessageBubble {message} />
      {/each}
    {/if}
  </div>
</div>

<style>
  .conversation-viewport {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    border-bottom: 1px solid var(--border);
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
