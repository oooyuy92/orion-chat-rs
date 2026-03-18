<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import LoaderIcon from '@lucide/svelte/icons/loader';
  import XIcon from '@lucide/svelte/icons/x';
  import type { ToolCallState } from '$lib/types';

  let { call }: { call: ToolCallState } = $props();

  let expanded = $state(false);

  const elapsed = $derived(
    call.endTime
      ? `${((call.endTime - call.startTime) / 1000).toFixed(1)}s`
      : '...',
  );

  function formatArgs(args: string): string {
    try {
      const parsed = JSON.parse(args);
      const values = Object.values(parsed);
      return values.length > 0 ? String(values[0]).slice(0, 60) : '';
    } catch {
      return args.slice(0, 60);
    }
  }
</script>

<div class="timeline-item" class:expanded>
  <button type="button" class="timeline-row" onclick={() => (expanded = !expanded)}>
    <span class="status-icon">
      {#if call.status === 'completed'}
        <CheckIcon class="h-3.5 w-3.5 text-green-600" />
      {:else if call.status === 'error'}
        <XIcon class="h-3.5 w-3.5 text-red-500" />
      {:else}
        <LoaderIcon class="h-3.5 w-3.5 text-yellow-500 animate-spin" />
      {/if}
    </span>
    <span class="tool-name">{call.toolName}</span>
    <span class="tool-summary">{formatArgs(call.args)}</span>
    <span class="elapsed">{elapsed}</span>
    <span
      class="chevron"
      style={`transform: ${expanded ? 'rotate(90deg)' : 'rotate(0deg)'}`}
    >
      <ChevronRightIcon class="h-3 w-3" />
    </span>
  </button>

  {#if expanded && call.result}
    <pre class="tool-output">{call.result}</pre>
  {/if}
</div>

<style>
  .timeline-item {
    font-size: 12px;
  }

  .timeline-row {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    background: none;
    border: none;
    padding: 3px 0;
    cursor: pointer;
    text-align: left;
    color: var(--muted-foreground);
  }

  .timeline-row:hover {
    color: var(--foreground);
  }

  .status-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .tool-name {
    font-weight: 500;
    color: var(--foreground);
    flex-shrink: 0;
  }

  .tool-summary {
    color: var(--muted-foreground);
    font-family: monospace;
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 200px;
  }

  .elapsed {
    margin-left: auto;
    font-size: 10px;
    color: var(--muted-foreground);
    white-space: nowrap;
  }

  .chevron {
    transition: transform 0.15s ease;
    flex-shrink: 0;
  }

  .tool-output {
    margin: 4px 0 4px 22px;
    padding: 8px;
    background: var(--muted);
    border-radius: 6px;
    font-size: 11px;
    font-family: monospace;
    max-height: 200px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-all;
    color: var(--foreground);
  }
</style>
