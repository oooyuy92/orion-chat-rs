<script lang="ts">
  import { Check, ChevronDown } from '@lucide/svelte';
  import { Select } from 'bits-ui';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { cn } from '$lib/utils.js';

  type McpTransport = 'stdio' | 'http';

  let {
    onSubmit,
    onCancel,
  }: {
    onSubmit: (config: {
      name: string;
      transport: McpTransport;
      commandOrUrl: string;
      args: string[];
    }) => void;
    onCancel: () => void;
  } = $props();

  const options: { value: McpTransport; label: string }[] = [
    { value: 'stdio', label: 'stdio' },
    { value: 'http', label: 'http' },
  ];

  const items = options.map((option) => ({
    value: option.value,
    label: option.label,
  }));

  let name = $state('');
  let transport = $state<McpTransport>('stdio');
  let commandOrUrl = $state('');
  let argsInput = $state('');

  function handleTransportChange(value: string) {
    transport = value as McpTransport;

    if (transport === 'http') {
      argsInput = '';
    }
  }

  function submitForm(event: SubmitEvent) {
    event.preventDefault();

    onSubmit({
      name: name.trim(),
      transport,
      commandOrUrl: commandOrUrl.trim(),
      args: transport === 'stdio' ? argsInput.trim().split(/\s+/).filter(Boolean) : [],
    });
  }
</script>

<form class="space-y-4 rounded-lg border p-4" onsubmit={submitForm}>
  <div class="space-y-2">
    <label class="text-sm font-medium" for="mcp-server-name">Name</label>
    <Input id="mcp-server-name" bind:value={name} placeholder="filesystem" required />
  </div>

  <div class="space-y-2">
    <label class="text-sm font-medium" for="mcp-server-transport">Transport type</label>
    <Select.Root
      type="single"
      value={transport}
      items={items}
      onValueChange={handleTransportChange}
    >
      <Select.Trigger
        id="mcp-server-transport"
        class={cn(
          'border-input bg-background hover:bg-accent hover:text-accent-foreground inline-flex h-9 w-full items-center justify-between rounded-md border px-3 text-sm shadow-xs outline-none transition-colors',
          'focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] disabled:pointer-events-none disabled:opacity-50',
        )}
        aria-label="MCP transport type"
      >
        <span class="truncate">{options.find((option) => option.value === transport)?.label ?? transport}</span>
        <ChevronDown class="size-4 shrink-0 opacity-60" />
      </Select.Trigger>
      <Select.Content
        sideOffset={4}
        class="bg-popover text-popover-foreground data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 z-50 min-w-[var(--bits-combobox-anchor-width)] rounded-md border p-1 shadow-md outline-none"
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

  <div class="space-y-2">
    <label class="text-sm font-medium" for="mcp-server-command">
      {transport === 'stdio' ? 'Command' : 'URL'}
    </label>
    <Input
      id="mcp-server-command"
      bind:value={commandOrUrl}
      placeholder={transport === 'stdio' ? 'npx' : 'http://localhost:3000/mcp'}
      required
    />
  </div>

  {#if transport === 'stdio'}
    <div class="space-y-2">
      <label class="text-sm font-medium" for="mcp-server-args">Args</label>
      <Input
        id="mcp-server-args"
        bind:value={argsInput}
        placeholder="-y @modelcontextprotocol/server-filesystem /tmp"
      />
      <p class="text-xs text-muted-foreground">使用空格分隔参数</p>
    </div>
  {/if}

  <div class="flex justify-end gap-2">
    <Button variant="outline" onclick={onCancel}>Cancel</Button>
    <Button type="submit">Submit</Button>
  </div>
</form>
