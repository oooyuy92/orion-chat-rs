<script lang="ts">
  import type { Assistant } from '$lib/types';
  import { i18n } from '$lib/stores/i18n.svelte';

  let {
    assistants = [],
    selectedAssistantId = null,
    disabled = false,
    onSelect,
  }: {
    assistants?: Assistant[];
    selectedAssistantId?: string | null;
    disabled?: boolean;
    onSelect?: (assistantId: string | null) => void;
  } = $props();

  function handleSelect(assistantId: string | null) {
    if (disabled) return;
    onSelect?.(assistantId);
  }
</script>

<div class="assistant-tabs-shell">
  <div class="assistant-tabs-scroll" role="tablist" aria-label={i18n.t.assistants}>
    <button
      type="button"
      role="tab"
      class="assistant-tab"
      class:is-active={!selectedAssistantId}
      class:is-disabled={disabled}
      aria-selected={!selectedAssistantId}
      disabled={disabled}
      onclick={() => handleSelect(null)}
    >
      {i18n.t.noAssistant}
    </button>

    {#each assistants as assistant (assistant.id)}
      <button
        type="button"
        role="tab"
        class="assistant-tab"
        class:is-active={assistant.id === selectedAssistantId}
        class:is-disabled={disabled}
        aria-selected={assistant.id === selectedAssistantId}
        disabled={disabled}
        onclick={() => handleSelect(assistant.id)}
      >
        {assistant.name}
      </button>
    {/each}
  </div>
</div>

<style>
  .assistant-tabs-shell {
    min-width: 0;
  }

  .assistant-tabs-scroll {
    display: flex;
    gap: 0.5rem;
    overflow-x: auto;
    padding-bottom: 0.25rem;
    scrollbar-width: none;
  }

  .assistant-tabs-scroll::-webkit-scrollbar {
    display: none;
  }

  .assistant-tab {
    flex: 0 0 auto;
    white-space: nowrap;
    border: 1px solid var(--border);
    background: var(--background);
    color: var(--foreground);
    border-radius: 999px;
    padding: 0.45rem 0.8rem;
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .assistant-tab:hover:enabled {
    background: var(--muted);
  }

  .assistant-tab.is-active {
    border-color: var(--primary);
    background: color-mix(in oklab, var(--primary) 12%, var(--card));
    color: var(--primary);
  }

  .assistant-tab.is-disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
</style>
