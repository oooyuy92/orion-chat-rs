<script lang="ts">
  import type { ModelCombo, ModelGroup } from '$lib/types';
  import { resolveModelLabel } from '$lib/utils/modelDisplay';
  import { Popover, PopoverContent, PopoverTrigger } from '$lib/components/ui/popover';
  import { loadCombos } from '$lib/stores/modelCombos';
  import { i18n } from '$lib/stores/i18n.svelte';
  import UsersRoundIcon from '@lucide/svelte/icons/users-round';

  let {
    modelGroups = [],
    disabled = false,
    activeComboModelIds = null,
    onSelectCombo,
    onClearCombo,
  }: {
    modelGroups?: ModelGroup[];
    disabled?: boolean;
    activeComboModelIds?: string[] | null;
    onSelectCombo?: (modelIds: string[]) => void;
    onClearCombo?: () => void;
  } = $props();

  let combos = $state<ModelCombo[]>([]);
  let open = $state(false);

  $effect(() => {
    if (open) {
      loadCombos().then((c) => (combos = c));
    }
  });

  const isActive = $derived(activeComboModelIds !== null && activeComboModelIds.length > 0);

  function resolveModelName(modelId: string): string {
    for (const group of modelGroups) {
      const model = group.models.find((m) => m.id === modelId);
      if (model) return resolveModelLabel(model);
    }
    return modelId;
  }

  function resolveComboTitle(modelIds: string[]): string {
    return modelIds.map(resolveModelName).join(', ');
  }

  function handleSelect(combo: ModelCombo) {
    onSelectCombo?.(combo.modelIds);
    open = false;
  }

  function handleClear() {
    onClearCombo?.();
    open = false;
  }
</script>

<Popover bind:open>
  <PopoverTrigger>
    <button
      class="combo-trigger"
      class:active={isActive}
      {disabled}
      title={isActive ? i18n.t.comboActive : i18n.t.selectCombo}
      onclick={(e) => {
        if (isActive) {
          e.preventDefault();
          e.stopPropagation();
          handleClear();
        }
      }}
    >
      <UsersRoundIcon size={14} />
      {#if isActive}
        <span class="combo-label">{i18n.t.comboActive}</span>
      {/if}
    </button>
  </PopoverTrigger>
  <PopoverContent align="start" class="w-64 p-0">
    <div class="combo-popover">
      <div class="combo-header">{i18n.t.selectCombo}</div>
      {#if combos.length === 0}
        <div class="combo-empty">{i18n.t.noCombo}</div>
      {:else}
        <div class="combo-list">
          {#each combos as combo (combo.id)}
            <button
              class="combo-item"
              title={resolveComboTitle(combo.modelIds)}
              onclick={() => handleSelect(combo)}
            >
              <span class="combo-name">{combo.name}</span>
              <span class="combo-count">{combo.modelIds.length} models</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </PopoverContent>
</Popover>

<style>
  .combo-trigger {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    border: 1px solid var(--border);
    background: var(--background);
    color: var(--muted-foreground);
    border-radius: 0.5rem;
    padding: 0.3rem 0.5rem;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .combo-trigger:hover:not(:disabled) {
    background: var(--muted);
    color: var(--foreground);
  }

  .combo-trigger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .combo-trigger.active {
    background: var(--primary);
    color: var(--primary-foreground);
    border-color: var(--primary);
  }

  .combo-label {
    font-weight: 500;
  }

  .combo-popover {
    max-height: 280px;
    overflow-y: auto;
  }

  .combo-header {
    padding: 0.6rem 0.75rem;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--foreground);
    border-bottom: 1px solid var(--border);
  }

  .combo-empty {
    padding: 1.5rem 0.75rem;
    text-align: center;
    color: var(--muted-foreground);
    font-size: 0.8rem;
  }

  .combo-list {
    padding: 0.25rem;
  }

  .combo-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.5rem 0.6rem;
    border: none;
    background: none;
    color: var(--foreground);
    font-size: 0.8rem;
    cursor: pointer;
    border-radius: 0.375rem;
    transition: background-color 0.1s ease;
  }

  .combo-item:hover {
    background: var(--muted);
  }

  .combo-name {
    font-weight: 500;
  }

  .combo-count {
    color: var(--muted-foreground);
    font-size: 0.75rem;
  }
</style>
