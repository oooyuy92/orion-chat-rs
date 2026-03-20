<script lang="ts">
  import type { ChatEvent, ToolCallState } from '$lib/types';
  import { devMode, agentEventLog } from '$lib/stores/agent';
  import ToolTimelineItem from './ToolTimelineItem.svelte';

  let { calls }: { calls: ToolCallState[] } = $props();

  const hasRunning = $derived(calls.some((call) => call.status === 'running'));
  let eventLogOpen = $state(false);
  let logContainer: HTMLDivElement | undefined = $state();

  function formatTime(ts: number): string {
    const d = new Date(ts);
    return d.toTimeString().slice(0, 8) + '.' + String(d.getMilliseconds()).padStart(3, '0');
  }

  type EventColor = 'blue' | 'orange' | 'red' | 'gray' | 'green' | 'default';

  function eventColor(type: string): EventColor {
    switch (type) {
      case 'toolCallStart':
      case 'toolCallUpdate':
      case 'toolCallEnd':
        return 'blue';
      case 'toolAuthRequest':
        return 'orange';
      case 'error':
        return 'red';
      case 'delta':
      case 'reasoning':
        return 'gray';
      case 'finished':
        return 'green';
      default:
        return 'default';
    }
  }

  function eventSummary(event: ChatEvent): string {
    switch (event.type) {
      case 'started':
        return `msg=${event.messageId.slice(0, 8)}`;
      case 'delta':
        return event.content.length > 60 ? event.content.slice(0, 60) + '…' : event.content;
      case 'reasoning':
        return event.content.length > 60 ? event.content.slice(0, 60) + '…' : event.content;
      case 'toolCallStart':
        return `${event.toolName} (${event.toolCallId.slice(0, 8)})`;
      case 'toolCallUpdate':
        return `${event.toolCallId.slice(0, 8)} partial`;
      case 'toolCallEnd':
        return `${event.toolCallId.slice(0, 8)} ${event.isError ? 'ERROR' : 'ok'}`;
      case 'toolAuthRequest':
        return `${event.toolName} needs auth`;
      case 'error':
        return event.message;
      case 'usage':
        return `prompt=${event.promptTokens} completion=${event.completionTokens}`;
      case 'finished':
        return `msg=${event.messageId.slice(0, 8)}`;
      default:
        return '';
    }
  }

  $effect(() => {
    // auto-scroll when new events arrive
    if ($agentEventLog.length && logContainer) {
      logContainer.scrollTop = logContainer.scrollHeight;
    }
  });
</script>

{#if calls.length > 0}
  <div class="tool-timeline">
    <div class="timeline-label">
      {hasRunning ? 'Agent running' : 'Agent complete'}
    </div>
    {#each calls as call (call.toolCallId)}
      <ToolTimelineItem {call} />
    {/each}

    {#if $devMode}
      <div class="event-log-section">
        <button class="event-log-toggle" onclick={() => (eventLogOpen = !eventLogOpen)}>
          <span class="event-log-arrow" class:open={eventLogOpen}>▶</span>
          Event Log ({$agentEventLog.length})
        </button>

        {#if eventLogOpen}
          <div class="event-log-list" bind:this={logContainer}>
            {#each $agentEventLog as entry, i (i)}
              <div class="event-row">
                <span class="event-time">{formatTime(entry.time)}</span>
                <span class="event-type event-color-{eventColor(entry.event.type)}">{entry.event.type}</span>
                <span class="event-summary">{eventSummary(entry.event)}</span>
              </div>
            {/each}
            {#if $agentEventLog.length === 0}
              <div class="event-row event-empty">No events yet</div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
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

  .event-log-section {
    margin-top: 4px;
    border-top: 1px solid var(--border);
    padding-top: 4px;
  }

  .event-log-toggle {
    appearance: none;
    background: none;
    border: none;
    padding: 2px 0;
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--muted-foreground);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .event-log-toggle:hover {
    color: var(--foreground);
  }

  .event-log-arrow {
    display: inline-block;
    font-size: 8px;
    transition: transform 0.15s;
  }

  .event-log-arrow.open {
    transform: rotate(90deg);
  }

  .event-log-list {
    max-height: 200px;
    overflow-y: auto;
    font-family: var(--font-mono, monospace);
    font-size: 10px;
    line-height: 1.5;
    margin-top: 2px;
  }

  .event-row {
    display: flex;
    gap: 6px;
    padding: 1px 0;
    align-items: baseline;
  }

  .event-empty {
    color: var(--muted-foreground);
    font-style: italic;
  }

  .event-time {
    color: var(--muted-foreground);
    flex-shrink: 0;
  }

  .event-type {
    font-weight: 600;
    flex-shrink: 0;
    min-width: 90px;
  }

  .event-summary {
    color: var(--muted-foreground);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .event-color-blue { color: #3b82f6; }
  .event-color-orange { color: #f59e0b; }
  .event-color-red { color: #ef4444; }
  .event-color-gray { color: var(--muted-foreground); }
  .event-color-green { color: #22c55e; }
  .event-color-default { color: var(--foreground); }
</style>
