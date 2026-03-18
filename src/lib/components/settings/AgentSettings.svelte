<script lang="ts">
  import { onMount } from 'svelte';
  import McpServerManager from './McpServerManager.svelte';
  import ToolPermissionRow from './ToolPermissionRow.svelte';
  import SkillsManager from './SkillsManager.svelte';
  import type { PermissionLevel, ToolPermissions } from '$lib/types';
  import { getToolPermissions, setToolPermissions } from '$lib/api/agent';

  const builtinTools = ['read_file', 'list_files', 'search', 'edit_file', 'write_file', 'bash'];

  let permissions = $state<ToolPermissions>({});
  let loading = $state(true);

  onMount(() => {
    void loadPermissions();
  });

  async function loadPermissions() {
    try {
      permissions = await getToolPermissions();
    } finally {
      loading = false;
    }
  }

  async function updatePermission(toolName: string, level: PermissionLevel) {
    permissions = { ...permissions, [toolName]: level };
    await setToolPermissions({ ...permissions });
  }
</script>

{#if loading}
  <p class="text-sm text-muted-foreground">加载中...</p>
{:else}
  <div class="space-y-6">
    <section>
      <h3 class="mb-3 text-sm font-semibold">工具授权</h3>
      <div class="divide-y">
        {#each builtinTools as tool}
          <ToolPermissionRow
            toolName={tool}
            level={permissions[tool] ?? 'ask'}
            onLevelChange={(level) => void updatePermission(tool, level)}
          />
        {/each}
      </div>
    </section>

    <section>
      <h3 class="mb-3 text-sm font-semibold">Skills</h3>
      <SkillsManager />
    </section>

    <section>
      <h3 class="text-sm font-semibold mb-3">MCP 服务器</h3>
      <McpServerManager />
    </section>
  </div>
{/if}
