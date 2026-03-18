<script lang="ts">
  import type { ToolCallState } from '$lib/types';
  import ToolTimelineItem from './ToolTimelineItem.svelte';

  let { calls }: { calls: ToolCallState[] } = $props();

  const hasRunning = $derived(calls.some((call) => call.status === 'running'));
</script>

{#if calls.length > 0}
  <div class="tool-timeline">
    <div class="timeline-label">
      {hasRunning ? 'Agent running' : 'Agent complete'}
    </div>
    {#each calls as call (call.toolCallId)}
      <ToolTimelineItem {call} />
    {/each}
  </div>
{/if}

<style>
  .tool-timeline {
    border-left: 2px solid var(--border);
    padding-left: 10px;
    margin: 4px 0 8px 3px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .timeline-label {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--muted-foreground);
    margin-bottom: 2px;
  }
</style>
