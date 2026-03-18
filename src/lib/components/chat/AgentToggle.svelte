<script lang="ts">
  import BotIcon from '@lucide/svelte/icons/bot';
  import { agentMode } from '$lib/stores/agent';

  let { disabled = false }: { disabled?: boolean } = $props();

  function toggle() {
    if (!disabled) {
      agentMode.update((value) => !value);
    }
  }
</script>

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

<style>
  .agent-toggle {
    display: flex;
    align-items: center;
    gap: 0;
    margin-left: auto;
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

  .agent-toggle:hover {
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

  .agent-toggle:hover .agent-label {
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
