<script lang="ts">
  import type { Message } from '$lib/types';
  import { api } from '$lib/utils/invoke';
  import { i18n } from '$lib/stores/i18n.svelte';

  let query = $state('');
  let results = $state<Message[]>([]);
  let searching = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  function handleInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    query = value;
    clearTimeout(debounceTimer);
    if (!value.trim()) {
      results = [];
      return;
    }
    debounceTimer = setTimeout(() => doSearch(value.trim()), 300);
  }

  async function doSearch(q: string) {
    searching = true;
    try {
      results = await api.searchMessages(q);
    } catch (e) {
      console.error('Search failed:', e);
      results = [];
    } finally {
      searching = false;
    }
  }

  function roleBadgeColor(role: string): string {
    switch (role) {
      case 'user': return 'var(--accent)';
      case 'assistant': return 'var(--msg-assistant)';
      default: return 'var(--text-secondary)';
    }
  }

  function truncate(text: string, max = 100): string {
    return text.length > max ? text.slice(0, max) + '...' : text;
  }
</script>

<div class="flex flex-col h-full" style="background-color: var(--bg-sidebar);">
  <div class="p-3">
    <input
      type="text"
      placeholder={i18n.t.searchMessages}
      value={query}
      oninput={handleInput}
      class="w-full py-2 px-3 rounded-lg text-sm"
      style="background-color: var(--bg-primary); color: var(--text-primary); border: 1px solid var(--border); outline: none;"
    />
  </div>

  <div class="flex-1 overflow-y-auto px-2 pb-2">
    {#if searching}
      <p class="text-sm px-3 py-2" style="color: var(--text-secondary);">{i18n.t.searching}</p>
    {:else if query.trim() && results.length === 0}
      <p class="text-sm px-3 py-2" style="color: var(--text-secondary);">{i18n.t.noResultsFound}</p>
    {:else}
      {#each results as msg (msg.id)}
        <div
          class="px-3 py-2 rounded-lg mb-1 text-sm"
          style="background-color: var(--bg-primary); border: 1px solid var(--border);"
        >
          <span
            class="inline-block text-xs font-medium rounded px-1.5 py-0.5 mb-1"
            style="background-color: {roleBadgeColor(msg.role)}; color: #fff;"
          >
            {i18n.roleLabel(msg.role)}
          </span>
          <p class="m-0 text-sm" style="color: var(--text-primary);">
            {truncate(msg.content)}
          </p>
        </div>
      {/each}
    {/if}
  </div>
</div>
