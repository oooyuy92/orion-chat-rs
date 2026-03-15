<script lang="ts">
  import type { ModelGroup } from '$lib/types';
  import { resolveModelLabel } from '$lib/utils/modelDisplay';
  import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger
  } from '$lib/components/ui/dropdown-menu';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import { i18n } from '$lib/stores/i18n.svelte';

  let {
    modelGroups,
    selected = $bindable(''),
    onSelect,
  }: {
    modelGroups: ModelGroup[];
    selected?: string;
    onSelect?: (modelId: string) => void;
  } = $props();

  const selectedModel = $derived(() => {
    for (const group of modelGroups) {
      const model = group.models.find((m) => m.id === selected);
      if (model) return model;
    }
    return null;
  });

  const selectedModelLabel = $derived.by(() => {
    const model = selectedModel();
    return model ? resolveModelLabel(model) : i18n.t.selectModel;
  });

  function handleSelect(modelId: string) {
    selected = modelId;
    onSelect?.(modelId);
  }
</script>

<DropdownMenu>
  <DropdownMenuTrigger>
    {#snippet child({ props })}
      <button
        {...props}
        class="model-trigger"
      >
        <span class="model-name">{selectedModelLabel}</span>
        <ChevronDownIcon class="h-3.5 w-3.5 opacity-50" />
      </button>
    {/snippet}
  </DropdownMenuTrigger>
  <DropdownMenuContent
    side="top"
    align="start"
    collisionPadding={8}
    class="w-[300px] bg-popover border border-border shadow-md"
  >
    {#each modelGroups as group, index (group.providerId)}
      {#if index > 0}
        <DropdownMenuSeparator />
      {/if}
      <DropdownMenuLabel>{group.providerName}</DropdownMenuLabel>
      {#each group.models as model (model.id)}
        <DropdownMenuItem
          onclick={() => handleSelect(model.id)}
          class={selected === model.id ? 'bg-accent' : ''}
        >
          {resolveModelLabel(model)}
        </DropdownMenuItem>
      {/each}
    {/each}
  </DropdownMenuContent>
</DropdownMenu>

<style>
  .model-trigger {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    border: 1px solid var(--border);
    background: var(--background);
    color: var(--foreground);
    border-radius: 0.5rem;
    padding: 0.3rem 0.6rem;
    font-size: 0.875rem;
    line-height: 1.4;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .model-trigger:hover {
    background: var(--muted);
  }

  .model-name {
    max-width: 16rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
