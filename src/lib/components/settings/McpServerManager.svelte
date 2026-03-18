<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Plus, RadioTower, Trash2 } from '@lucide/svelte';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import McpServerForm from './McpServerForm.svelte';

  type McpTransport = 'stdio' | 'http';

  interface McpServerConfig {
    name: string;
    transport: McpTransport;
    commandOrUrl: string;
    args: string[];
  }

  interface McpServerStatus {
    config: McpServerConfig;
    connected: boolean;
  }

  let servers = $state<McpServerStatus[]>([]);
  let loading = $state(true);
  let showForm = $state(false);
  let saving = $state(false);
  let deletingName = $state<string | null>(null);
  let error = $state('');

  onMount(() => {
    void loadServers();
  });

  async function loadServers() {
    try {
      servers = await invoke<McpServerStatus[]>('list_mcp_servers');
      error = '';
    } catch (e) {
      console.error('Failed to load MCP servers:', e);
      error = `加载失败：${e}`;
    } finally {
      loading = false;
    }
  }

  async function handleAdd(config: McpServerConfig) {
    saving = true;
    error = '';

    try {
      await invoke('add_mcp_server', { config });
      showForm = false;
      await loadServers();
    } catch (e) {
      console.error('Failed to add MCP server:', e);
      error = `添加失败：${e}`;
    } finally {
      saving = false;
    }
  }

  async function handleRemove(name: string) {
    deletingName = name;
    error = '';

    try {
      await invoke('remove_mcp_server', { name });
      await loadServers();
    } catch (e) {
      console.error('Failed to remove MCP server:', e);
      error = `删除失败：${e}`;
    } finally {
      deletingName = null;
    }
  }
</script>

<div class="space-y-3">
  <div class="flex items-center justify-between gap-3">
    <h3 class="text-sm font-semibold">MCP 服务器</h3>
    <Button
      size="sm"
      variant="outline"
      class="gap-1.5"
      disabled={saving}
      onclick={() => (showForm = !showForm)}
    >
      <Plus class="size-3.5" />
      添加
    </Button>
  </div>

  {#if error}
    <p class="text-sm text-destructive">{error}</p>
  {/if}

  {#if showForm}
    <McpServerForm
      onSubmit={(config) => void handleAdd(config)}
      onCancel={() => (showForm = false)}
    />
  {/if}

  {#if loading}
    <p class="text-sm text-muted-foreground">加载中...</p>
  {:else if servers.length === 0}
    <p class="text-sm text-muted-foreground">暂无 MCP 服务器</p>
  {:else}
    <div class="divide-y rounded-lg border">
      {#each servers as server}
        <div class="flex items-center justify-between gap-4 p-4">
          <div class="flex min-w-0 items-center gap-3">
            <div class="bg-muted text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-md border">
              <RadioTower class="size-4" />
            </div>
            <div class="min-w-0 space-y-1">
              <div class="flex items-center gap-2">
                <span class="truncate text-sm font-medium">{server.config.name}</span>
                <span class="text-xs text-muted-foreground">{server.config.transport}</span>
              </div>
              <p class="truncate text-xs text-muted-foreground">{server.config.commandOrUrl}</p>
            </div>
          </div>

          <div class="flex shrink-0 items-center gap-2">
            <Badge variant={server.connected ? 'default' : 'outline'}>
              {server.connected ? '已连接' : '未连接'}
            </Badge>
            <Button
              size="icon-sm"
              variant="ghost"
              disabled={deletingName === server.config.name}
              aria-label={`Delete ${server.config.name}`}
              onclick={() => void handleRemove(server.config.name)}
            >
              <Trash2 class="size-4" />
            </Button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
