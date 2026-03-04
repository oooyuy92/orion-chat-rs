<script lang="ts">
  import type { ModelGroup } from '$lib/types';
  import { Button } from '$lib/components/ui/button';
  import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger
  } from '$lib/components/ui/dropdown-menu';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';

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

  function handleSelect(modelId: string) {
    selected = modelId;
    onSelect?.(modelId);
  }
</script>

<DropdownMenu>
  <DropdownMenuTrigger>
    {#snippet child({ props })}
      <Button {...props} variant="outline" class="w-full justify-between">
        <span class="flex items-center gap-2">
          <span class="text-muted-foreground text-xs">*</span>
          <span class="text-sm">
            {selectedModel()?.name || 'Select model'}
          </span>
        </span>
        <ChevronDownIcon class="h-4 w-4 opacity-50" />
      </Button>
    {/snippet}
  </DropdownMenuTrigger>
  <DropdownMenuContent class="w-[300px]">
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
          {model.name}
        </DropdownMenuItem>
      {/each}
    {/each}
  </DropdownMenuContent>
</DropdownMenu>
