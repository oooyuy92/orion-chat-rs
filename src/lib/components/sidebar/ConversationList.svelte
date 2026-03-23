<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { loadStore } from '$lib/stores/kvStore';
  import type { Assistant, Conversation, SearchSidebarResult } from '$lib/types';
  import { api } from '$lib/utils/invoke';
  import { waitForTauriReady } from '$lib/api/platform';
  import { groupConversationsByTime } from '$lib/utils/date';
  import { i18n, type ConversationGroupKey } from '$lib/stores/i18n.svelte';
  import { titleUpdates, assistantUpdates, conversationCreated, streamingConversations } from '$lib/stores/conversations';
  import {
    SidebarGroup,
    SidebarGroupLabel,
    SidebarMenu,
    SidebarMenuItem,
    SidebarMenuButton,
  } from '$lib/components/ui/sidebar';
  import { Button } from '$lib/components/ui/button';

  type ConversationSelection = {
    conversationId: string;
    messageId?: string | null;
  };

  type SearchResultView = SearchSidebarResult & {
    source: 'message' | 'title' | 'assistant';
    score: number;
  };

  let {
    activeId = $bindable(''),
    onSelect,
  }: {
    activeId: string;
    onSelect: (selection: ConversationSelection) => void;
  } = $props();

  let conversations = $state<Conversation[]>([]);
  let assistants = $state<Assistant[]>([]);
  let loading = $state(true);
  let prefixes = $state<Record<string, string>>({});
  let searchQuery = $state('');
  let isSearching = $state(false);
  let searchResults = $state<SearchResultView[]>([]);
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let searchRequestId = 0;
  let streamingIds = $state<Set<string>>(new Set());

  $effect(() => {
    const unsub = streamingConversations.subscribe((ids) => { streamingIds = ids; });
    return unsub;
  });

  const pinned = $derived(conversations.filter((c) => c.isPinned));
  const unpinned = $derived(conversations.filter((c) => !c.isPinned));
  const grouped = $derived(groupConversationsByTime(unpinned));
  const isSearchMode = $derived(searchQuery.trim().length > 0);

  $effect(() => {
    const unsub = titleUpdates.subscribe((upd) => {
      if (!upd) return;
      conversations = conversations.map((c) =>
        c.id === upd.id ? { ...c, title: upd.title } : c,
      );
      if (searchQuery.trim()) {
        void runSearch(searchQuery.trim());
      }
    });
    return unsub;
  });

  $effect(() => {
    const unsub = assistantUpdates.subscribe((upd) => {
      if (!upd) return;
      conversations = conversations.map((c) =>
        c.id === upd.id ? { ...c, assistantId: upd.assistantId } : c,
      );
    });
    return unsub;
  });

  $effect(() => {
    const unsub = conversationCreated.subscribe((conv) => {
      if (!conv) return;
      if (!conversations.some((c) => c.id === conv.id)) {
        conversations = [conv, ...conversations];
      }
    });
    return unsub;
  });

  let contextMenu = $state<{ x: number; y: number; id: string } | null>(null);

  function openContextMenu(e: MouseEvent, id: string) {
    e.preventDefault();
    e.stopPropagation();
    contextMenu = { x: e.clientX, y: e.clientY, id };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

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
      if (searchQuery.trim()) {
        void runSearch(searchQuery.trim());
      }
    } catch (e) {
      console.error(e);
    }
  }

  function handleRenameKey(e: KeyboardEvent) {
    if (e.key === 'Enter') commitRename();
    if (e.key === 'Escape') renamingId = '';
  }

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

  function assistantNameFor(conversation: Conversation): string {
    if (!conversation.assistantId) return '';
    return assistants.find((assistant) => assistant.id === conversation.assistantId)?.name ?? '';
  }

  function assistantNameForConversationId(conversationId: string): string {
    const conversation = conversations.find((item) => item.id === conversationId);
    return conversation ? assistantNameFor(conversation) : '';
  }

  function normalizeText(value: string): string {
    return value.trim().toLowerCase();
  }

  function fuzzyScore(query: string, target: string): number {
    const needle = normalizeText(query);
    const haystack = normalizeText(target);
    if (!needle || !haystack) return -1;

    const includeIndex = haystack.indexOf(needle);
    if (includeIndex !== -1) {
      return 1200 - includeIndex * 4 - Math.max(0, haystack.length - needle.length);
    }

    let queryIndex = 0;
    let streak = 0;
    let score = 0;
    for (let index = 0; index < haystack.length; index += 1) {
      if (haystack[index] === needle[queryIndex]) {
        queryIndex += 1;
        streak += 1;
        score += 8 + streak;
        if (queryIndex === needle.length) {
          return 500 + score - index;
        }
      } else {
        streak = 0;
      }
    }

    return -1;
  }

  function buildLocalSearchResults(query: string): SearchResultView[] {
    const results: SearchResultView[] = [];

    for (const conversation of conversations) {
      const assistantName = assistantNameFor(conversation);
      const titleScore = fuzzyScore(query, conversation.title);
      const assistantScore = assistantName ? fuzzyScore(query, assistantName) : -1;

      if (titleScore < 0 && assistantScore < 0) continue;

      if (titleScore >= assistantScore) {
        results.push({
          conversationId: conversation.id,
          messageId: null,
          snippet: conversation.title,
          createdAt: conversation.updatedAt,
          source: 'title',
          score: titleScore,
        });
      } else {
        results.push({
          conversationId: conversation.id,
          messageId: null,
          snippet: `#${assistantName}`,
          createdAt: conversation.updatedAt,
          source: 'assistant',
          score: assistantScore - 40,
        });
      }
    }

    return results;
  }

  function mergeSearchResults(
    localResults: SearchResultView[],
    remoteResults: SearchSidebarResult[],
  ): SearchResultView[] {
    const merged = new Map<string, SearchResultView>();

    for (const result of localResults) {
      const key = `${result.conversationId}:local`;
      const current = merged.get(key);
      if (!current || result.score > current.score) {
        merged.set(key, result);
      }
    }

    for (const result of remoteResults) {
      const key = `${result.conversationId}:${result.messageId ?? 'message'}`;
      if (!merged.has(key)) {
        merged.set(key, {
          ...result,
          source: 'message',
          score: 800,
        });
      }
    }

    return [...merged.values()].sort((left, right) => {
      if (right.score !== left.score) {
        return right.score - left.score;
      }
      return right.createdAt.localeCompare(left.createdAt);
    });
  }

  async function runSearch(query: string) {
    const trimmed = query.trim();
    if (!trimmed) {
      searchResults = [];
      isSearching = false;
      return;
    }

    const requestId = ++searchRequestId;
    isSearching = true;
    const localResults = buildLocalSearchResults(trimmed);

    try {
      const remoteResults = await api.searchSidebarResults(trimmed);
      if (requestId !== searchRequestId) return;
      searchResults = mergeSearchResults(localResults, remoteResults);
    } catch (e) {
      if (requestId !== searchRequestId) return;
      console.error('Failed to search sidebar results:', e);
      searchResults = localResults;
    } finally {
      if (requestId === searchRequestId) {
        isSearching = false;
      }
    }
  }

  function handleSearchInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    searchQuery = value;
    clearTimeout(searchTimer);

    if (!value.trim()) {
      searchRequestId += 1;
      isSearching = false;
      searchResults = [];
      return;
    }

    searchTimer = setTimeout(() => {
      void runSearch(value);
    }, 300);
  }

  function conversationTitleFor(conversationId: string): string {
    return conversations.find((conversation) => conversation.id === conversationId)?.title ?? conversationId;
  }

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

  async function handleDelete(id: string) {
    closeContextMenu();
    // Optimistic: remove from UI immediately
    const previous = conversations;
    conversations = conversations.filter((c) => c.id !== id);
    searchResults = searchResults.filter((result) => result.conversationId !== id);
    if (activeId === id) activeId = '';
    try {
      await api.deleteConversation(id);
    } catch (e) {
      console.error(e);
      // Rollback on failure
      conversations = previous;
    }
  }

  async function loadConversations() {
    try {
      await waitForTauriReady();
      conversations = await api.listConversations();
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  }

  async function loadAssistants() {
    try {
      assistants = await api.listAssistants();
    } catch (e) {
      console.error(e);
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
      const conversation = await api.createConversation(i18n.t.newChatTitle);
      conversations = [conversation, ...conversations];
      activeId = conversation.id;
      onSelect({ conversationId: conversation.id, messageId: null });
    } catch (e) {
      console.error(e);
    }
  }

  onMount(() => {
    loadConversations();
    loadAssistants();
    loadPrefixes();
  });

  onDestroy(() => {
    clearTimeout(searchTimer);
  });
</script>

{#snippet convItem(conversation: Conversation)}
  <SidebarMenuItem>
    <SidebarMenuButton
      class="h-auto min-h-8 items-start py-1.5"
      isActive={conversation.id === activeId}
      onclick={() => {
        activeId = conversation.id;
        onSelect({ conversationId: conversation.id, messageId: null });
      }}
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
          placeholder={i18n.t.prefixPlaceholder}
          onblur={commitPrefix}
          onkeydown={handlePrefixKey}
          onclick={(e: MouseEvent) => e.stopPropagation()}
        />
        <span class="conversation-title muted">{conversation.title}</span>
      {:else}
        {#if streamingIds.has(conversation.id)}
          <svg class="streaming-indicator" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 12a9 9 0 1 1-6.219-8.56"/>
          </svg>
        {/if}
        <div class="conversation-main">
          <div class="conversation-title-row">
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
          </div>
          {#if assistantNameFor(conversation)}
            <span class="assistant-tag">#{assistantNameFor(conversation)}</span>
          {/if}
        </div>
      {/if}
    </SidebarMenuButton>
  </SidebarMenuItem>
{/snippet}

<div class="conversation-sidebar">
  <div class="sidebar-head">
    <Button onclick={handleNewChat} class="w-full">{i18n.t.newChat}</Button>
    <input
      type="text"
      class="search-input"
      placeholder={i18n.t.searchMessages}
      value={searchQuery}
      oninput={handleSearchInput}
    />
  </div>

  <div class="sidebar-list">
    {#if isSearchMode}
      {#if isSearching}
        <p class="sidebar-status">{i18n.t.searching}</p>
      {:else if searchResults.length === 0}
        <p class="sidebar-status">{i18n.t.noResultsFound}</p>
      {:else}
        <SidebarMenu>
          {#each searchResults as result (`${result.conversationId}:${result.messageId ?? result.source}`)}
            <SidebarMenuItem>
              <SidebarMenuButton
                class="h-auto min-h-8 items-start py-1.5"
                isActive={result.conversationId === activeId}
                onclick={() => {
                  activeId = result.conversationId;
                  onSelect({ conversationId: result.conversationId, messageId: result.messageId });
                }}
              >
                <div class="search-result-main">
                  <div class="search-result-top">
                    <span class="search-result-title">{conversationTitleFor(result.conversationId)}</span>
                    <span class="search-result-time">{i18n.formatRelativeTime(result.createdAt)}</span>
                  </div>
                  <span class="search-result-snippet">{result.snippet}</span>
                  {#if assistantNameForConversationId(result.conversationId)}
                    <span class="assistant-tag">#{assistantNameForConversationId(result.conversationId)}</span>
                  {/if}
                </div>
              </SidebarMenuButton>
            </SidebarMenuItem>
          {/each}
        </SidebarMenu>
      {/if}
    {:else if loading}
      <p class="sidebar-status">{i18n.t.loadingConversations}</p>
    {:else if conversations.length === 0}
      <p class="sidebar-status">{i18n.t.noConversationsYet}</p>
    {:else}
      {#if pinned.length > 0}
        <SidebarGroup>
          <SidebarGroupLabel>{i18n.t.pinned}</SidebarGroupLabel>
          <SidebarMenu>
            {#each pinned as conversation (conversation.id)}
              {@render convItem(conversation)}
            {/each}
          </SidebarMenu>
        </SidebarGroup>
      {/if}

      {#each [
        { key: 'today', items: grouped.today },
        { key: 'yesterday', items: grouped.yesterday },
        { key: 'last7Days', items: grouped.last7Days },
        { key: 'last30Days', items: grouped.last30Days },
        { key: 'older', items: grouped.older },
      ] as group (group.key)}
        {#if group.items.length > 0}
          <SidebarGroup>
            <SidebarGroupLabel>{i18n.conversationGroupLabel(group.key as ConversationGroupKey)}</SidebarGroupLabel>
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
    role="button"
    aria-label={i18n.t.close}
    tabindex="0"
    onclick={closeContextMenu}
    onkeydown={(e) => {
      if (e.key === 'Escape' || e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        closeContextMenu();
      }
    }}
    oncontextmenu={(e) => { e.preventDefault(); closeContextMenu(); }}
  >
    <div class="context-menu" style="left: {contextMenu.x}px; top: {contextMenu.y}px;">
      <button class="context-item" onclick={() => startRename(contextMenu!.id)}>{i18n.t.rename}</button>
      <button class="context-item" onclick={() => startPrefix(contextMenu!.id)}>{i18n.t.addPrefix}</button>
      <button class="context-item" onclick={() => handlePin(contextMenu!.id)}>
        {conversations.find((c) => c.id === contextMenu?.id)?.isPinned ? i18n.t.unpin : i18n.t.pin}
      </button>
      <button class="context-item danger" onclick={() => handleDelete(contextMenu!.id)}>{i18n.t.delete}</button>
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
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }

  .search-input {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 0.65rem;
    background: var(--background);
    color: var(--foreground);
    font-size: 0.84rem;
    padding: 0.58rem 0.7rem;
    box-sizing: border-box;
    outline: none;
  }

  .search-input:focus {
    border-color: hsl(var(--primary) / 0.45);
    box-shadow: 0 0 0 3px hsl(var(--primary) / 0.08);
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

  .conversation-main,
  .search-result-main {
    min-width: 0;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .conversation-title-row,
  .search-result-top {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .search-result-top {
    justify-content: space-between;
    gap: 0.6rem;
  }

  .conversation-title,
  .search-result-title {
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

  .assistant-tag,
  .search-result-snippet,
  .search-result-time {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted-foreground);
    font-size: 0.7rem;
    line-height: 1.15;
  }

  .search-result-snippet {
    white-space: nowrap;
  }

  .search-result-time {
    flex: 0 0 auto;
  }

  .pin-icon {
    flex-shrink: 0;
    opacity: 0.5;
    color: var(--muted-foreground);
  }

  .streaming-indicator {
    flex-shrink: 0;
    color: hsl(var(--primary));
    animation: spin 1.2s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
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
