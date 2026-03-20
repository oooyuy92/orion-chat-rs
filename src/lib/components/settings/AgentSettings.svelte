<script lang="ts">
  import { onMount } from 'svelte';
  import { load as loadStore } from '@tauri-apps/plugin-store';
  import McpServerManager from './McpServerManager.svelte';
  import ToolPermissionRow from './ToolPermissionRow.svelte';
  import SkillsManager from './SkillsManager.svelte';
  import type { PermissionLevel, ToolPermissions } from '$lib/types';
  import { getToolPermissions, setToolPermissions } from '$lib/api/agent';
  import { devMode } from '$lib/stores/agent';

  const builtinTools = ['read_file', 'list_files', 'search', 'edit_file', 'write_file', 'bash'];

  let permissions = $state<ToolPermissions>({});
  let loading = $state(true);
  let devModeOn = $state(false);

  onMount(() => {
    void loadPermissions();
    void loadDevMode();
  });

  async function loadPermissions() {
    try {
      permissions = await getToolPermissions();
    } finally {
      loading = false;
    }
  }

  async function loadDevMode() {
    try {
      const store = await loadStore('settings.json');
      const val = await store.get<boolean>('devMode');
      if (val != null) {
        devModeOn = val;
        devMode.set(val);
      }
    } catch {
      // ignore – settings file may not exist yet
    }
  }

  async function toggleDevMode() {
    devModeOn = !devModeOn;
    devMode.set(devModeOn);
    try {
      const store = await loadStore('settings.json');
      await store.set('devMode', devModeOn);
      await store.save();
    } catch {
      // persist best-effort
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

    <section>
      <h3 class="mb-3 text-sm font-semibold">开发者模式</h3>
      <div class="general-switch-row">
        <span class="field-label">显示事件日志</span>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <button type="button" class="toggle-switch" class:is-on={devModeOn} aria-pressed={devModeOn} aria-label="开发者模式" onclick={toggleDevMode}>
          <span class="toggle-thumb"></span>
        </button>
      </div>
      <p class="field-hint">开启后，工具时间线下方会显示原始 Agent 事件日志。</p>
    </section>
  </div>
{/if}

<style>
  .general-switch-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .field-label {
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--foreground);
  }

  .field-hint {
    font-size: 0.75rem;
    color: var(--muted-foreground);
    margin-top: 0.3rem;
  }

  .toggle-switch {
    appearance: none;
    padding: 0;
    position: relative;
    width: 2.4rem;
    height: 1.3rem;
    background: var(--muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .toggle-switch.is-on {
    background: #22c55e;
    border-color: #22c55e;
  }

  .toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 0.9rem;
    height: 0.9rem;
    background: white;
    border-radius: 50%;
    transition: transform 0.2s;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
  }

  .toggle-switch.is-on .toggle-thumb {
    transform: translateX(1.1rem);
  }
</style>
