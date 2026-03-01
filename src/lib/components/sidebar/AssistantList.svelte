<script lang="ts">
  import type { Assistant } from '$lib/types';
  import { api } from '$lib/utils/invoke';

  let { onSelect }: { onSelect: (assistant: Assistant) => void } = $props();

  let assistants = $state<Assistant[]>([]);
  let loading = $state(true);

  async function loadAssistants() {
    try {
      assistants = await api.listAssistants();
    } catch (e) {
      console.error('Failed to load assistants:', e);
    } finally {
      loading = false;
    }
  }

  async function handleNew() {
    try {
      const assistant = await api.createAssistant('New Assistant');
      assistants = [...assistants, assistant];
    } catch (e) {
      console.error('Failed to create assistant:', e);
    }
  }

  async function handleDelete(e: Event, id: string) {
    e.stopPropagation();
    try {
      await api.deleteAssistant(id);
      assistants = assistants.filter((a) => a.id !== id);
    } catch (err) {
      console.error('Failed to delete assistant:', err);
    }
  }

  $effect(() => {
    loadAssistants();
  });
</script>

<div class="flex flex-col h-full" style="background-color: var(--bg-sidebar);">
  <div class="p-3">
    <button
      onclick={handleNew}
      class="w-full py-2 px-3 rounded-lg text-sm font-medium cursor-pointer transition-colors"
      style="background-color: var(--accent); color: #fff; border: none;"
    >
      + New Assistant
    </button>
  </div>

  <div class="flex-1 overflow-y-auto px-2 pb-2">
    {#if loading}
      <p class="text-sm px-3 py-2" style="color: var(--text-secondary);">Loading...</p>
    {:else if assistants.length === 0}
      <p class="text-sm px-3 py-2" style="color: var(--text-secondary);">No assistants yet</p>
    {:else}
      {#each assistants as assistant (assistant.id)}
        <button
          onclick={() => onSelect(assistant)}
          class="w-full text-left px-3 py-2 rounded-lg mb-0.5 text-sm cursor-pointer transition-colors flex items-center justify-between group"
          style="background-color: transparent; color: var(--text-primary); border: none;"
        >
          <span class="flex items-center gap-2 truncate flex-1">
            <span class="text-base">{assistant.icon ?? '🤖'}</span>
            <span class="truncate">{assistant.name}</span>
          </span>
          <span
            role="button"
            tabindex="0"
            class="opacity-0 group-hover:opacity-100 ml-2 text-xs transition-opacity"
            onclick={(e: MouseEvent) => handleDelete(e, assistant.id)}
            onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') handleDelete(e, assistant.id); }}
            style="color: var(--text-secondary);"
          >
            &#x2715;
          </span>
        </button>
      {/each}
    {/if}
  </div>
</div>
