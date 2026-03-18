<script lang="ts">
  import { Check, ChevronDown } from '@lucide/svelte';
  import { Select } from 'bits-ui';
  import type { PermissionLevel } from '$lib/types';
  import { cn } from '$lib/utils.js';

  let {
    toolName,
    level,
    onLevelChange,
  }: {
    toolName: string;
    level: PermissionLevel;
    onLevelChange: (level: PermissionLevel) => void;
  } = $props();

  const options: { value: PermissionLevel; label: string }[] = [
    { value: 'auto', label: '自动执行' },
    { value: 'ask', label: '需要确认' },
    { value: 'deny', label: '禁用' },
  ];

  const items = options.map((option) => ({
    value: option.value,
    label: option.label,
  }));

  function handleValueChange(value: string) {
    onLevelChange(value as PermissionLevel);
  }
</script>

<div class="flex items-center justify-between gap-4 py-2">
  <span class="font-mono text-sm">{toolName}</span>
  <Select.Root type="single" value={level} items={items} onValueChange={handleValueChange}>
    <Select.Trigger
      class={cn(
        'border-input bg-background hover:bg-accent hover:text-accent-foreground inline-flex h-8 w-32 items-center justify-between rounded-md border px-3 text-xs shadow-xs outline-none transition-colors',
        'focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] disabled:pointer-events-none disabled:opacity-50',
      )}
      aria-label={`${toolName} permission`}
    >
      <span class="truncate">{options.find((option) => option.value === level)?.label ?? level}</span>
      <ChevronDown class="size-3.5 shrink-0 opacity-60" />
    </Select.Trigger>
    <Select.Content
      sideOffset={4}
      class="bg-popover text-popover-foreground data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 z-50 min-w-32 rounded-md border p-1 shadow-md outline-none"
    >
      {#each options as option}
        <Select.Item
          value={option.value}
          label={option.label}
          class="data-highlighted:bg-accent data-highlighted:text-accent-foreground relative flex cursor-default items-center rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50"
        >
          {#snippet children({ selected })}
            {#if selected}
              <span class="absolute left-2 flex size-4 items-center justify-center">
                <Check class="size-3.5" />
              </span>
            {/if}
            {option.label}
          {/snippet}
        </Select.Item>
      {/each}
    </Select.Content>
  </Select.Root>
</div>
