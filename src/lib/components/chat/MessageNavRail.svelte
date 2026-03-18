<script lang="ts">
  import type { Message } from '$lib/types';
  import { activeToolCalls } from '$lib/stores/agent';
  import { buildMessageRows } from './agentTimeline';
  import { getMeasuredHeight } from './messageVirtualization.js';

  function excerptFirstLine(content: string, max = 16): string {
    const line = content.replace(/\n.*/s, '').trim();
    return line.length > max ? line.slice(0, max) + '…' : line;
  }

  let {
    messages,
    scrollTop = 0,
    viewportHeight = 0,
    totalHeight = 0,
    heightCache = new Map(),
    estimatedRowHeight = 180,
    onJumpToMessage,
    onJumpToTop,
    onJumpToBottom,
  }: {
    messages: Message[];
    scrollTop?: number;
    viewportHeight?: number;
    totalHeight?: number;
    heightCache?: Map<string, number>;
    estimatedRowHeight?: number;
    onJumpToMessage?: (index: number) => void;
    onJumpToTop?: () => void;
    onJumpToBottom?: () => void;
  } = $props();

  let railEl: HTMLDivElement | undefined = $state();
  let railHeight = $state(0);
  let hoveredIndex = $state<number | null>(null);

  const BUTTON_HEIGHT = 22;
  const displayRows = $derived(buildMessageRows(messages, $activeToolCalls));
  const availableHeight = $derived(Math.max(0, railHeight - BUTTON_HEIGHT * 2 - 8));

  function fisheye(t: number, focal: number, strength: number): number {
    const d = t - focal;
    const sign = d < 0 ? -1 : 1;
    const abs = Math.abs(d);
    const warped = Math.pow(abs, strength);
    const maxPos = Math.pow(Math.max(focal, 1 - focal), strength);
    return focal + sign * (warped / maxPos) * Math.max(focal, 1 - focal);
  }

  const messagePositions = $derived.by(() => {
    if (displayRows.length === 0 || totalHeight === 0 || availableHeight <= 0) return [];

    const ids = displayRows.map((row) => row.message.id);
    let offset = 0;
    const raw: { t: number; role: string; visible: boolean }[] = [];
    const scrollBottom = scrollTop + viewportHeight;
    const focalCenter = totalHeight > 0
      ? Math.min(1, Math.max(0, (scrollTop + viewportHeight / 2) / totalHeight))
      : 0.5;

    for (let i = 0; i < displayRows.length; i++) {
      const h = getMeasuredHeight(ids[i], heightCache, estimatedRowHeight);
      const msgCenter = offset + h / 2;
      const t = msgCenter / totalHeight;
      const msgTop = offset;
      const msgBottom = offset + h;
      const visible = msgBottom > scrollTop && msgTop < scrollBottom;
      raw.push({ t, role: displayRows[i].message.role, visible });
      offset += h;
    }

    const needsFisheye = displayRows.length > 30;
    const strength = needsFisheye
      ? Math.min(2.2, 1 + (displayRows.length - 30) / 80)
      : 1;

    return raw.map(({ t, role, visible }) => {
      const y = needsFisheye
        ? fisheye(t, focalCenter, strength) * availableHeight
        : t * availableHeight;
      return { y, role, visible };
    });
  });
</script>

<div class="nav-rail" bind:this={railEl} bind:clientHeight={railHeight}>
  <button class="nav-btn" onclick={() => onJumpToTop?.()} title="Jump to top">
    <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor">
      <path d="M5 2L1 7h8z" />
    </svg>
  </button>

  <div class="rail-track">
    {#each messagePositions as pos, i}
      <button
        class="pill"
        class:pill-user={pos.role === 'user'}
        class:pill-assistant={pos.role !== 'user'}
        class:pill-visible={pos.visible}
        style:top="{pos.y}px"
        onclick={() => onJumpToMessage?.(i)}
        onmouseenter={() => (hoveredIndex = i)}
        onmouseleave={() => (hoveredIndex = null)}
      >
        {#if hoveredIndex === i}
          <span class="tooltip" class:tooltip-user={pos.role === 'user'} class:tooltip-assistant={pos.role !== 'user'}>
            <span class="tooltip-tag" class:tag-user={pos.role === 'user'} class:tag-assistant={pos.role !== 'user'}>
              {pos.role === 'user' ? 'You' : 'AI'}
            </span>
            {excerptFirstLine(displayRows[i].message.content)}
          </span>
        {/if}
      </button>
    {/each}
  </div>

  <button class="nav-btn" onclick={() => onJumpToBottom?.()} title="Jump to bottom">
    <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor">
      <path d="M5 8L1 3h8z" />
    </svg>
  </button>
</div>

<style>
  .nav-rail {
    width: 20px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 4px 0;
    user-select: none;
  }

  .nav-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 22px;
    border: none;
    background: none;
    color: var(--muted-foreground);
    cursor: pointer;
    padding: 0;
    border-radius: 4px;
    transition: color 0.15s, background 0.15s;
  }

  .nav-btn:hover {
    color: var(--foreground);
    background: hsl(var(--primary) / 0.08);
  }

  .rail-track {
    flex: 1;
    position: relative;
    width: 100%;
    min-height: 0;
  }

  .pill {
    position: absolute;
    left: 50%;
    transform: translateX(-50%) translateY(-50%);
    width: 6px;
    height: 14px;
    border-radius: 3px;
    border: none;
    padding: 0;
    cursor: pointer;
    transition: opacity 0.15s, transform 0.15s, background 0.15s;
  }

  .pill-user {
    background: var(--muted-foreground);
    opacity: 0.45;
    border-radius: 50%;
    width: 5px;
    height: 5px;
  }

  .pill-assistant {
    background: oklch(0.55 0.15 250);
    opacity: 0.7;
  }

  .pill-visible.pill-user {
    opacity: 0.8;
    width: 6px;
    height: 6px;
  }

  .pill-visible.pill-assistant {
    opacity: 1;
    transform: translateX(-50%) translateY(-50%) scaleY(1.2);
  }

  .pill:hover {
    opacity: 1;
    transform: translateX(-50%) translateY(-50%) scale(1.4);
  }

  .tooltip {
    position: absolute;
    right: calc(100% + 8px);
    top: 50%;
    transform: translateY(-50%);
    font-size: 0.65rem;
    line-height: 1.3;
    white-space: nowrap;
    background: var(--popover);
    color: var(--popover-foreground);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 3px 7px;
    pointer-events: none;
    box-shadow: 0 2px 8px hsl(0 0% 0% / 0.14);
    display: flex;
    align-items: center;
    gap: 4px;
    border-left: 2.5px solid transparent;
  }

  .tooltip-user {
    border-left-color: var(--muted-foreground);
  }

  .tooltip-assistant {
    border-left-color: oklch(0.55 0.15 250);
  }

  .tooltip-tag {
    font-size: 0.58rem;
    font-weight: 600;
    padding: 1px 4px;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .tag-user {
    background: hsl(0 0% 50% / 0.12);
    color: var(--muted-foreground);
  }

  .tag-assistant {
    background: oklch(0.55 0.15 250 / 0.15);
    color: oklch(0.55 0.15 250);
  }

  @media (max-width: 640px) {
    .nav-rail {
      display: none;
    }
  }
</style>
