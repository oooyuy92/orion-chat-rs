<script lang="ts">
  import { onMount } from 'svelte';
  import type { Conversation } from '$lib/types';
  import { api } from '$lib/utils/invoke';

  let {
    activeId = $bindable(''),
    onSelect,
  }: {
    activeId: string;
    onSelect: (id: string) => void;
  } = $props();

  let conversations = $state<Conversation[]>([]);
  let loading = $state(true);

  async function loadConversations() {
    try {
      conversations = await api.listConversations();
    } catch (e) {
      console.error('Failed to load conversations:', e);
    } finally {
      loading = false;
    }
  }

  async function handleNewChat() {
    try {
      const conv = await api.createConversation('New Chat');
      conversations = [conv, ...conversations];
      activeId = conv.id;
      onSelect(conv.id);
    } catch (e) {
      console.error('Failed to create conversation:', e);
    }
  }

  async function handleDelete(e: Event, id: string) {
    e.stopPropagation();
    try {
      await api.deleteConversation(id);
      conversations = conversations.filter((c) => c.id !== id);
      if (activeId === id) {
        activeId = '';
      }
    } catch (err) {
      console.error('Failed to delete conversation:', err);
    }
  }

  onMount(() => {
    loadConversations();
  });
</script>

<div class="flex flex-col h-full" style="background-color: var(--bg-sidebar);">
  <div class="p-3">
    <button
      onclick={handleNewChat}
      class="w-full py-2 px-3 rounded-lg text-sm font-medium cursor-pointer transition-colors"
      style="background-color: var(--accent); color: #fff; border: none;"
    >
      + New Chat
    </button>
  </div>

  <div class="flex-1 overflow-y-auto px-2 pb-2">
    {#if loading}
      <p class="text-sm px-3 py-2" style="color: var(--text-secondary);">Loading...</p>
    {:else if conversations.length === 0}
      <p class="text-sm px-3 py-2" style="color: var(--text-secondary);">No conversations yet</p>
    {:else}
      {#each conversations as conv (conv.id)}
        <button
          onclick={() => { activeId = conv.id; onSelect(conv.id); }}
          class="w-full text-left px-3 py-2 rounded-lg mb-0.5 text-sm cursor-pointer transition-colors flex items-center justify-between group"
          style="background-color: {conv.id === activeId ? 'var(--accent)' : 'transparent'}; color: {conv.id === activeId ? '#fff' : 'var(--text-primary)'}; border: none;"
        >
          <span class="truncate flex-1">{conv.title}</span>
          <span
            role="button"
            tabindex="0"
            class="opacity-0 group-hover:opacity-100 ml-2 text-xs transition-opacity"
            onclick={(e: MouseEvent) => handleDelete(e, conv.id)}
            onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') handleDelete(e, conv.id); }}
            style="color: {conv.id === activeId ? '#ddd' : 'var(--text-secondary)'};"
          >
            &#x2715;
          </span>
        </button>
      {/each}
    {/if}
  </div>
</div>
