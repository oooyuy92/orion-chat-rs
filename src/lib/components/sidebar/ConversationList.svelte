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
    } catch (error) {
      console.error('Failed to load conversations:', error);
    } finally {
      loading = false;
    }
  }

  async function handleNewChat() {
    try {
      const conversation = await api.createConversation('New Chat');
      conversations = [conversation, ...conversations];
      activeId = conversation.id;
      onSelect(conversation.id);
    } catch (error) {
      console.error('Failed to create conversation:', error);
    }
  }

  async function handleDelete(event: Event, id: string) {
    event.stopPropagation();
    try {
      await api.deleteConversation(id);
      conversations = conversations.filter((conversation) => conversation.id !== id);
      if (activeId === id) {
        activeId = '';
      }
    } catch (error) {
      console.error('Failed to delete conversation:', error);
    }
  }

  onMount(() => {
    loadConversations();
  });
</script>

<div class="conversation-sidebar">
  <div class="sidebar-head">
    <button onclick={handleNewChat} class="new-chat-button">+ New Chat</button>
  </div>

  <div class="sidebar-list">
    {#if loading}
      <p class="sidebar-status">Loading conversations...</p>
    {:else if conversations.length === 0}
      <p class="sidebar-status">No conversations yet</p>
    {:else}
      {#each conversations as conversation (conversation.id)}
        <button
          class="conversation-item"
          class:is-active={conversation.id === activeId}
          onclick={() => {
            activeId = conversation.id;
            onSelect(conversation.id);
          }}
        >
          <span class="conversation-title">{conversation.title}</span>
          <span
            role="button"
            tabindex="0"
            class="conversation-delete"
            onclick={(event: MouseEvent) => handleDelete(event, conversation.id)}
            onkeydown={(event: KeyboardEvent) => {
              if (event.key === 'Enter') {
                handleDelete(event, conversation.id);
              }
            }}
          >
            &#10005;
          </span>
        </button>
      {/each}
    {/if}
  </div>
</div>

<style>
  .conversation-sidebar {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--sidebar-bg);
  }

  .sidebar-head {
    padding: 0.75rem;
  }

  .new-chat-button {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 0.65rem;
    background: var(--background);
    color: var(--foreground);
    font-size: 0.84rem;
    font-weight: 580;
    padding: 0.58rem 0.7rem;
    cursor: pointer;
  }

  .new-chat-button:hover {
    background: var(--muted);
  }

  .sidebar-list {
    min-height: 0;
    flex: 1;
    overflow-y: auto;
    padding: 0 0.55rem 0.65rem;
  }

  .sidebar-status {
    margin: 0;
    color: var(--muted-foreground);
    font-size: 0.82rem;
    padding: 0.5rem 0.45rem;
  }

  .conversation-item {
    width: 100%;
    border: 1px solid transparent;
    background: transparent;
    color: var(--foreground);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    border-radius: 0.65rem;
    padding: 0.5rem 0.62rem;
    margin-bottom: 0.18rem;
    text-align: left;
    cursor: pointer;
  }

  .conversation-item:hover {
    background: var(--sidebar-hover);
  }

  .conversation-item.is-active {
    background: var(--sidebar-active);
    border-color: var(--border);
  }

  .conversation-title {
    min-width: 0;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.83rem;
  }

  .conversation-delete {
    opacity: 0;
    color: var(--muted-foreground);
    font-size: 0.7rem;
    padding: 0.1rem;
    border-radius: 0.2rem;
  }

  .conversation-item:hover .conversation-delete {
    opacity: 1;
  }

  .conversation-delete:hover {
    color: var(--foreground);
    background: rgba(255, 255, 255, 0.7);
  }
</style>
