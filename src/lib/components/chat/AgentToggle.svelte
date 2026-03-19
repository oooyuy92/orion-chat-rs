<script lang="ts">
  import BotIcon from '@lucide/svelte/icons/bot';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import XIcon from '@lucide/svelte/icons/x';
  import { agentMode } from '$lib/stores/agent';
  import { api } from '$lib/utils/invoke';

  type ConversationWorkingDirApi = typeof api & {
    getConversationWorkingDirs: (conversationId: string) => Promise<string[]>;
    setConversationWorkingDirs: (conversationId: string, dirs: string[]) => Promise<void>;
  };

  const workingDirApi = api as ConversationWorkingDirApi;

  let {
    disabled = false,
    conversationId = '',
  }: {
    disabled?: boolean;
    conversationId?: string;
  } = $props();

  let workingDirs = $state<string[]>([]);
  let showPopover = $state(false);
  let hoverTimeout = $state<ReturnType<typeof setTimeout> | null>(null);

  $effect(() => {
    if (!conversationId) {
      workingDirs = [];
      return;
    }

    void loadWorkingDirs(conversationId);
  });

  $effect(() => {
    return () => {
      if (hoverTimeout) {
        clearTimeout(hoverTimeout);
      }
    };
  });

  async function loadWorkingDirs(targetConversationId: string) {
    try {
      const dirs = await workingDirApi.getConversationWorkingDirs(targetConversationId);
      if (conversationId === targetConversationId) {
        workingDirs = dirs;
      }
    } catch (error) {
      console.error('Failed to load working directories:', error);
      if (conversationId === targetConversationId) {
        workingDirs = [];
      }
    }
  }

  function toggle() {
    if (!disabled) {
      agentMode.update((value) => !value);
    }
  }

  async function addWorkingDir() {
    if (!conversationId) return;

    try {
      const dir = await api.pickDirectory();
      if (!dir || workingDirs.includes(dir)) return;

      const updated = [...workingDirs, dir];
      await workingDirApi.setConversationWorkingDirs(conversationId, updated);
      workingDirs = updated;
    } catch (error) {
      console.error('Failed to add working directory:', error);
    }
  }

  async function removeWorkingDir(dir: string) {
    if (!conversationId) return;

    const updated = workingDirs.filter((value) => value !== dir);

    try {
      await workingDirApi.setConversationWorkingDirs(conversationId, updated);
      workingDirs = updated;
    } catch (error) {
      console.error('Failed to remove working directory:', error);
    }
  }

  function handleMouseEnter() {
    if (hoverTimeout) {
      clearTimeout(hoverTimeout);
      hoverTimeout = null;
    }

    showPopover = true;
  }

  function handleMouseLeave() {
    if (hoverTimeout) {
      clearTimeout(hoverTimeout);
    }

    hoverTimeout = setTimeout(() => {
      showPopover = false;
      hoverTimeout = null;
    }, 200);
  }

  function shortenPath(path: string): string {
    const normalized = path.replace(/^\/Users\/[^/]+/, '~');
    if (normalized.length <= 32) {
      return normalized;
    }

    const parts = normalized.split('/').filter(Boolean);
    if (parts.length <= 3) {
      return normalized;
    }

    const prefix = normalized.startsWith('~') ? '~' : normalized.startsWith('/') ? '' : parts[0];
    return `${prefix}/.../${parts.slice(-2).join('/')}`;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="agent-wrapper"
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
>
  {#if showPopover}
    <div class="popover">
      {#if workingDirs.length === 0}
        <div class="popover-empty">未设置工作目录</div>
      {:else}
        {#each workingDirs as dir (dir)}
          <div class="popover-row" title={dir}>
            <span class="popover-path">{shortenPath(dir)}</span>
            <button
              type="button"
              class="popover-remove"
              aria-label="删除工作目录"
              onclick={() => void removeWorkingDir(dir)}
            >
              <XIcon class="h-3 w-3" />
            </button>
          </div>
        {/each}
      {/if}

      <button
        type="button"
        class="popover-add"
        onclick={() => void addWorkingDir()}
      >
        <PlusIcon class="h-3 w-3" />
        <span>添加目录</span>
      </button>
    </div>
  {/if}

  <button
    type="button"
    class="agent-toggle"
    class:active={$agentMode}
    title={$agentMode ? 'Agent mode on' : 'Agent mode off'}
    onclick={toggle}
    {disabled}
  >
    <BotIcon class="h-3.5 w-3.5" />
    <span class="agent-label">
      Agent
      <span class="agent-badge">{$agentMode ? 'ON' : 'OFF'}</span>
    </span>
  </button>
</div>

<style>
  .agent-wrapper {
    position: relative;
    margin-left: auto;
  }

  .popover {
    position: absolute;
    right: 0;
    bottom: calc(100% + 6px);
    min-width: 220px;
    max-width: 320px;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--card);
    box-shadow: 0 4px 12px rgb(0 0 0 / 0.12);
    z-index: 50;
  }

  .popover-empty {
    padding: 8px 10px;
    font-size: 11px;
    color: var(--muted-foreground);
    text-align: center;
  }

  .popover-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border-radius: 5px;
  }

  .popover-row:hover {
    background: var(--muted);
  }

  .popover-path {
    flex: 1;
    overflow: hidden;
    color: var(--foreground);
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .popover-remove {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border: none;
    border-radius: 3px;
    background: none;
    color: var(--muted-foreground);
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.1s ease;
  }

  .popover-row:hover .popover-remove {
    opacity: 1;
  }

  .popover-remove:hover {
    background: var(--muted);
    color: var(--destructive);
  }

  .popover-add {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    margin-top: 2px;
    padding: 6px 8px;
    border: none;
    border-top: 1px solid var(--border);
    border-radius: 5px;
    background: none;
    color: var(--muted-foreground);
    font-size: 11px;
    cursor: pointer;
  }

  .popover-add:hover {
    background: var(--muted);
    color: var(--foreground);
  }

  .agent-toggle {
    display: flex;
    align-items: center;
    gap: 0;
    border-radius: 7px;
    padding: 4px 7px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid var(--border);
    background: var(--muted);
    color: var(--muted-foreground);
    white-space: nowrap;
    overflow: hidden;
    max-width: 27px;
    transition:
      max-width 0.22s ease,
      gap 0.18s ease,
      padding 0.18s ease,
      background 0.15s ease,
      color 0.15s ease,
      border-color 0.15s ease;
  }

  .agent-wrapper:hover .agent-toggle {
    max-width: 120px;
    gap: 5px;
    padding: 4px 10px;
  }

  .agent-toggle.active {
    background: var(--primary);
    color: var(--primary-foreground);
    border-color: var(--primary);
  }

  .agent-label {
    display: flex;
    align-items: center;
    gap: 4px;
    opacity: 0;
    width: 0;
    overflow: hidden;
    transition:
      opacity 0.15s ease 0.05s,
      width 0.22s ease;
  }

  .agent-wrapper:hover .agent-label {
    opacity: 1;
    width: auto;
  }

  .agent-badge {
    border-radius: 4px;
    padding: 0 5px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.04em;
  }

  .agent-toggle.active .agent-badge {
    background: rgb(255 255 255 / 0.16);
  }

  .agent-toggle:not(.active) .agent-badge {
    background: var(--border);
  }

  .agent-toggle:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
