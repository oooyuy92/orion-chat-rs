<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { load as loadStore } from '@tauri-apps/plugin-store';
  import type { Conversation } from '$lib/types';
  import { api } from '$lib/utils/invoke';
  import { groupConversationsByTime } from '$lib/utils/date';
  import { titleUpdates } from '$lib/stores/conversations';
  import {
    SidebarGroup,
    SidebarGroupLabel,
    SidebarMenu,
    SidebarMenuItem,
    SidebarMenuButton,
  } from '$lib/components/ui/sidebar';

  let {
    activeId = $bindable(''),
    onSelect,
  }: {
    activeId: string;
    onSelect: (id: string) => void;
  } = $props();

  let conversations = $state<Conversation[]>([]);
  let loading = $state(true);
  let prefixes = $state<Record<string, string>>({});

  const pinned = $derived(conversations.filter((c) => c.isPinned));
  const unpinned = $derived(conversations.filter((c) => !c.isPinned));
  const grouped = $derived(groupConversationsByTime(unpinned));

  // Subscribe to external title updates (from auto-rename in +page.svelte)
  $effect(() => {
    const unsub = titleUpdates.subscribe((upd) => {
      if (!upd) return;
      conversations = conversations.map((c) =>
        c.id === upd.id ? { ...c, title: upd.title } : c,
      );
    });
    return unsub;
  });

  // ── Context menu ──
  let contextMenu = $state<{ x: number; y: number; id: string } | null>(null);

  function openContextMenu(e: MouseEvent, id: string) {
    e.preventDefault();
    e.stopPropagation();
    contextMenu = { x: e.clientX, y: e.clientY, id };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  // ── Rename ──
  let renamingId = $state('');
  let renameValue = $state('');
  let renameInput = $state<HTMLInputElement | null>(null);

  async function startRename(id: string) {
    closeContextMenu();
    const conv = conversations.find((c) => c.id === id);
    if (!conv) return;
    renamingId = id;
    renameValue = conv.title;
    await tick();
    renameInput?.focus();
    renameInput?.select();
  }

  async function commitRename() {
    const id = renamingId;
    const title = renameValue.trim();
    renamingId = '';
    if (!title) return;
    const conv = conversations.find((c) => c.id === id);
    if (!conv || conv.title === title) return;
    conversations = conversations.map((c) => (c.id === id ? { ...c, title } : c));
    try {
      await api.updateConversationTitle(id, title);
    } catch (e) {
      console.error(e);
    }
  }

  function handleRenameKey(e: KeyboardEvent) {
    if (e.key === 'Enter') commitRename();
    if (e.key === 'Escape') renamingId = '';
  }

  // ── Prefix ──
  let prefixingId = $state('');
  let prefixValue = $state('');
  let prefixInput = $state<HTMLInputElement | null>(null);

  async function startPrefix(id: string) {
    closeContextMenu();
    prefixingId = id;
    prefixValue = prefixes[id] ?? '';
    await tick();
    prefixInput?.focus();
    prefixInput?.select();
  }

  async function commitPrefix() {
    const id = prefixingId;
    const val = prefixValue.trim();
    prefixingId = '';
    prefixes = { ...prefixes, [id]: val };
    try {
      const store = await loadStore('settings.json');
      await store.set('conversationPrefixes', prefixes);
      await store.save();
    } catch (e) {
      console.error(e);
    }
  }

  function handlePrefixKey(e: KeyboardEvent) {
    if (e.key === 'Enter') commitPrefix();
    if (e.key === 'Escape') prefixingId = '';
  }

  function displayTitle(conv: Conversation): string {
    const prefix = prefixes[conv.id];
    return prefix ? `${prefix} ${conv.title}` : conv.title;
  }

  // ── Pin ──
  async function handlePin(id: string) {
    closeContextMenu();
    const conv = conversations.find((c) => c.id === id);
    if (!conv) return;
    const next = !conv.isPinned;
    conversations = conversations.map((c) => (c.id === id ? { ...c, isPinned: next } : c));
    try {
      await api.pinConversation(id, next);
    } catch (e) {
      console.error(e);
    }
  }

  // ── Delete ──
  async function handleDelete(id: string) {
    closeContextMenu();
    try {
      await api.deleteConversation(id);
      conversations = conversations.filter((c) => c.id !== id);
      if (activeId === id) activeId = '';
    } catch (e) {
      console.error(e);
    }
  }

  // ── Load / New ──
  async function loadConversations() {
    try {
      conversations = await api.listConversations();
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  }

  async function loadPrefixes() {
    try {
      const store = await loadStore('settings.json');
      prefixes = (await store.get<Record<string, string>>('conversationPrefixes')) ?? {};
    } catch (e) {
      console.error(e);
    }
  }

  async function handleNewChat() {
    try {
      const conversation = await api.createConversation('New Chat');
      conversations = [conversation, ...conversations];
      activeId = conversation.id;
      onSelect(conversation.id);
    } catch (e) {
      console.error(e);
    }
  }

  onMount(() => {
    loadConversations();
    loadPrefixes();
  });
</script>

{#snippet convItem(conversation: Conversation)}
  <SidebarMenuItem>
    <SidebarMenuButton
      isActive={conversation.id === activeId}
      onclick={() => { activeId = conversation.id; onSelect(conversation.id); }}
      oncontextmenu={(e: MouseEvent) => openContextMenu(e, conversation.id)}
    >
      {#if renamingId === conversation.id}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          bind:this={renameInput}
          bind:value={renameValue}
          class="rename-input"
          onblur={commitRename}
          onkeydown={handleRenameKey}
          onclick={(e: MouseEvent) => e.stopPropagation()}
        />
      {:else if prefixingId === conversation.id}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          bind:this={prefixInput}
          bind:value={prefixValue}
          class="rename-input prefix-input"
          placeholder="前缀…"
          onblur={commitPrefix}
          onkeydown={handlePrefixKey}
          onclick={(e: MouseEvent) => e.stopPropagation()}
        />
        <span class="conversation-title muted">{conversation.title}</span>
      {:else}
        {#if conversation.isPinned}
          <svg class="pin-icon" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="17" x2="12" y2="22"/>
            <path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"/>
          </svg>
        {/if}
        {#if prefixes[conversation.id]}
          <span class="prefix-tag">{prefixes[conversation.id]}</span>
        {/if}
        <span class="conversation-title">{conversation.title}</span>
      {/if}
    </SidebarMenuButton>
  </SidebarMenuItem>
{/snippet}

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
      {#if pinned.length > 0}
        <SidebarGroup>
          <SidebarGroupLabel>已固定</SidebarGroupLabel>
          <SidebarMenu>
            {#each pinned as conversation (conversation.id)}
              {@render convItem(conversation)}
            {/each}
          </SidebarMenu>
        </SidebarGroup>
      {/if}

      {#each [
        { label: 'Today', items: grouped.today },
        { label: 'Yesterday', items: grouped.yesterday },
        { label: 'Last 7 days', items: grouped.last7Days },
        { label: 'Last 30 days', items: grouped.last30Days },
        { label: 'Older', items: grouped.older },
      ] as group (group.label)}
        {#if group.items.length > 0}
          <SidebarGroup>
            <SidebarGroupLabel>{group.label}</SidebarGroupLabel>
            <SidebarMenu>
              {#each group.items as conversation (conversation.id)}
                {@render convItem(conversation)}
              {/each}
            </SidebarMenu>
          </SidebarGroup>
        {/if}
      {/each}
    {/if}
  </div>
</div>

{#if contextMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="context-overlay"
    onclick={closeContextMenu}
    oncontextmenu={(e) => { e.preventDefault(); closeContextMenu(); }}
  >
    <div class="context-menu" style="left: {contextMenu.x}px; top: {contextMenu.y}px;">
      <button class="context-item" onclick={() => startRename(contextMenu!.id)}>重命名</button>
      <button class="context-item" onclick={() => startPrefix(contextMenu!.id)}>添加前缀</button>
      <button class="context-item" onclick={() => handlePin(contextMenu!.id)}>
        {conversations.find((c) => c.id === contextMenu?.id)?.isPinned ? '取消固定' : '固定'}
      </button>
      <button class="context-item danger" onclick={() => handleDelete(contextMenu!.id)}>删除</button>
    </div>
  </div>
{/if}

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

  .conversation-title {
    min-width: 0;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.83rem;
  }

  .conversation-title.muted {
    opacity: 0.45;
  }

  .pin-icon {
    flex-shrink: 0;
    opacity: 0.5;
    color: var(--muted-foreground);
  }

  .prefix-tag {
    flex-shrink: 0;
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--primary);
    background: hsl(var(--primary) / 0.1);
    border-radius: 0.25rem;
    padding: 0.05rem 0.3rem;
    white-space: nowrap;
  }

  .rename-input {
    flex: 1;
    min-width: 0;
    font-size: 0.83rem;
    background: transparent;
    border: none;
    outline: none;
    color: var(--foreground);
    padding: 0;
  }

  .prefix-input {
    flex: 0 1 5rem;
  }

  /* ── Context menu ── */
  .context-overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
  }

  .context-menu {
    position: fixed;
    z-index: 201;
    background: var(--popover);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    padding: 0.25rem;
    min-width: 9rem;
    display: flex;
    flex-direction: column;
  }

  .context-item {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    border-radius: 0.3rem;
    padding: 0.42rem 0.7rem;
    font-size: 0.83rem;
    color: var(--foreground);
    cursor: pointer;
  }

  .context-item:hover {
    background: var(--accent);
  }

  .context-item.danger {
    color: hsl(var(--destructive));
  }

  .context-item.danger:hover {
    background: hsl(var(--destructive) / 0.1);
  }
</style>
