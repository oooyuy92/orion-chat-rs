<script lang="ts">
  import type { Message } from '$lib/types';
  import MessageBubble from './MessageBubble.svelte';

  let { messages }: { messages: Message[] } = $props();

  let container: HTMLDivElement | undefined = $state();

  $effect(() => {
    // Scroll to bottom whenever messages change
    if (messages.length && container) {
      container.scrollTop = container.scrollHeight;
    }
  });
</script>

<div
  bind:this={container}
  class="flex-1 overflow-y-auto px-4 py-4"
>
  {#if messages.length === 0}
    <div class="flex items-center justify-center h-full">
      <p class="text-sm" style="color: var(--text-secondary);">
        No messages yet. Start the conversation!
      </p>
    </div>
  {:else}
    {#each messages as msg (msg.id)}
      <MessageBubble message={msg} />
    {/each}
  {/if}
</div>
