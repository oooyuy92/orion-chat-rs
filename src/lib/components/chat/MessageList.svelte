<script lang="ts">
  import { tick } from 'svelte';
  import type { Message } from '$lib/types';
  import MessageBubble from './MessageBubble.svelte';
  import { i18n } from '$lib/stores/i18n.svelte';
  import {
    calculateVirtualWindow,
    getAdjustedScrollTopAfterPrepend,
    isNearBottom,
  } from './messageVirtualization.js';

  const ESTIMATED_ROW_HEIGHT = 180;
  const OVERSCAN = 4;
  const LOAD_MORE_THRESHOLD = 24;
  const BOTTOM_FOLLOW_THRESHOLD = 64;

  type MessageAction =
    | { type: 'delete'; messageId: string }
    | { type: 'resend'; messageId: string }
    | { type: 'editResend'; messageId: string; content: string }
    | { type: 'regenerate'; messageId: string; modelId: string | null }
    | { type: 'generateVersion'; messageId: string }
    | { type: 'switchVersion'; versionGroupId: string; versionNumber: number }
    | { type: 'expandVersions'; versionGroupId: string };

  let {
    conversationId = '',
    messages,
    onAction,
    disabled = false,
    hasMoreMessages = false,
    isLoadingMoreMessages = false,
    canLoadOlderMessages = true,
    focusedMessageId = null,
    onLoadOlder,
  }: {
    conversationId?: string;
    messages: Message[];
    onAction?: (action: MessageAction) => void;
    disabled?: boolean;
    hasMoreMessages?: boolean;
    isLoadingMoreMessages?: boolean;
    canLoadOlderMessages?: boolean;
    focusedMessageId?: string | null;
    onLoadOlder?: () => void | Promise<void>;
  } = $props();

  let container: HTMLDivElement | undefined = $state();
  let viewportHeight = $state(0);
  let scrollTop = $state(0);
  let shouldFollowBottom = $state(true);
  let previousConversationId = $state<string | null>(null);
  let previousMessageCount = $state(0);
  let previousLastMessageId = $state<string | null>(null);
  let pendingPrependAnchor = $state<{ scrollHeight: number; scrollTop: number } | null>(null);
  let heightCache = $state(new Map<string, number>());
  let highlightedMessageId = $state('');
  let lastFocusedMessageId = $state('');
  let highlightTimer = $state<ReturnType<typeof setTimeout> | null>(null);

  const messageIds = $derived(messages.map((message) => message.id));
  const virtualWindow = $derived(
    calculateVirtualWindow({
      itemKeys: messageIds,
      heightCache,
      viewportHeight: Math.max(viewportHeight, 1),
      scrollTop,
      overscan: OVERSCAN,
      estimatedItemHeight: ESTIMATED_ROW_HEIGHT,
    }),
  );
  const visibleMessages = $derived(messages.slice(virtualWindow.startIndex, virtualWindow.endIndex));

  function syncScrollMetrics() {
    if (!container) return;
    scrollTop = container.scrollTop;
    shouldFollowBottom = isNearBottom({
      scrollTop: container.scrollTop,
      scrollHeight: container.scrollHeight,
      clientHeight: container.clientHeight,
      threshold: BOTTOM_FOLLOW_THRESHOLD,
    });
  }

function scrollToBottom() {
  if (!container) return;
  container.scrollTop = container.scrollHeight;
  syncScrollMetrics();
}

function estimateOffsetForIndex(index: number) {
  return messageIds
    .slice(0, index)
    .reduce((sum, messageId) => sum + (heightCache.get(messageId) ?? ESTIMATED_ROW_HEIGHT), 0);
}

function highlightFocusedMessage(messageId: string) {
  highlightedMessageId = messageId;
  if (highlightTimer) {
    clearTimeout(highlightTimer);
  }
  highlightTimer = setTimeout(() => {
    if (highlightedMessageId === messageId) {
      highlightedMessageId = '';
    }
  }, 1800);
}

function scrollToFocusedMessage(messageId: string) {
  if (!container) return;
  const index = messages.findIndex((message) => message.id === messageId);
  if (index === -1) return;

  const estimatedHeight = heightCache.get(messageId) ?? ESTIMATED_ROW_HEIGHT;
  const estimatedTop = estimateOffsetForIndex(index);
  container.scrollTop = Math.max(
    0,
    estimatedTop - Math.max((container.clientHeight - estimatedHeight) / 2, 0),
  );
  syncScrollMetrics();

  void tick().then(() => {
    if (!container) return;
    const row = container.querySelector(`[data-message-id="${messageId}"]`) as HTMLDivElement | null;
    row?.scrollIntoView({ block: 'center' });
    syncScrollMetrics();
    highlightFocusedMessage(messageId);
  });
}

  function handleScroll() {
    if (!container) return;

    syncScrollMetrics();

    if (
      !hasMoreMessages ||
      isLoadingMoreMessages ||
      !canLoadOlderMessages ||
      !onLoadOlder ||
      container.scrollTop > LOAD_MORE_THRESHOLD
    ) {
      return;
    }

    pendingPrependAnchor = {
      scrollHeight: container.scrollHeight,
      scrollTop: container.scrollTop,
    };
    void onLoadOlder?.();
  }

  function updateMeasuredHeight(messageId: string, node: HTMLDivElement) {
    const nextHeight = Math.ceil(node.getBoundingClientRect().height);
    const currentHeight = heightCache.get(messageId);

    if (currentHeight === nextHeight) {
      return;
    }

    const nextCache = new Map(heightCache);
    nextCache.set(messageId, nextHeight);
    heightCache = nextCache;

    if (shouldFollowBottom) {
      requestAnimationFrame(() => {
        if (shouldFollowBottom) {
          scrollToBottom();
        }
      });
    }
  }

  function measureRow(node: HTMLDivElement, messageId: string) {
    updateMeasuredHeight(messageId, node);

    const observer = new ResizeObserver(() => {
      updateMeasuredHeight(messageId, node);
    });
    observer.observe(node);

    return {
      update(nextMessageId: string) {
        messageId = nextMessageId;
        updateMeasuredHeight(messageId, node);
      },
      destroy() {
        observer.disconnect();
      },
    };
  }

  $effect(() => {
    const nextConversationId = conversationId || null;
    if (nextConversationId === previousConversationId) {
      return;
    }

    previousConversationId = nextConversationId;
    previousMessageCount = 0;
    previousLastMessageId = null;
    pendingPrependAnchor = null;
    shouldFollowBottom = true;
    scrollTop = 0;
    heightCache = new Map();

    if (container) {
      container.scrollTop = 0;
      syncScrollMetrics();
    }
  });

  $effect(() => {
    if (!messageIds.length) {
      if (heightCache.size > 0) {
        heightCache = new Map();
      }
      return;
    }

    const activeIds = new Set(messageIds);
    let changed = false;
    const nextCache = new Map<string, number>();

    for (const [messageId, measuredHeight] of heightCache) {
      if (activeIds.has(messageId)) {
        nextCache.set(messageId, measuredHeight);
      } else {
        changed = true;
      }
    }

    if (changed || nextCache.size !== heightCache.size) {
      heightCache = nextCache;
    }
  });

  $effect(() => {
    viewportHeight;
    if (container) {
      syncScrollMetrics();
    }
  });

$effect(() => {
  if (!focusedMessageId || !container || focusedMessageId === lastFocusedMessageId) {
    return;
  }

  if (!messages.some((message) => message.id === focusedMessageId)) {
    return;
  }

  lastFocusedMessageId = focusedMessageId;
  scrollToFocusedMessage(focusedMessageId);
});

$effect(() => {
  const currentCount = messages.length;
    const currentLastMessageId = messages.at(-1)?.id ?? null;

    if (!container) {
      previousMessageCount = currentCount;
      previousLastMessageId = currentLastMessageId;
      return;
    }

    if (pendingPrependAnchor && !isLoadingMoreMessages) {
      const anchor = pendingPrependAnchor;
      pendingPrependAnchor = null;
      previousMessageCount = currentCount;
      previousLastMessageId = currentLastMessageId;
      void tick().then(() => {
        if (!container) return;
        container.scrollTop = getAdjustedScrollTopAfterPrepend({
          previousScrollHeight: anchor.scrollHeight,
          previousScrollTop: anchor.scrollTop,
          nextScrollHeight: container.scrollHeight,
        });
        syncScrollMetrics();
      });
      return;
    }

    if (currentCount === 0) {
      previousMessageCount = 0;
      previousLastMessageId = null;
      return;
    }

    const didAppendMessages = currentCount > previousMessageCount;
    const didReplaceTail = currentLastMessageId !== previousLastMessageId;

    if (previousMessageCount === 0 || ((didAppendMessages || didReplaceTail) && shouldFollowBottom)) {
      void tick().then(scrollToBottom);
    }

    previousMessageCount = currentCount;
    previousLastMessageId = currentLastMessageId;
  });
</script>

<div class="conversation-viewport" bind:this={container} bind:clientHeight={viewportHeight} onscroll={handleScroll}>
  <div class="conversation-stack">
    {#if isLoadingMoreMessages}
      <div class="loading-older-state">{i18n.t.loadingOlderMessages}</div>
    {/if}

    {#if messages.length === 0}
      <div class="empty-message-state">
        <h2>{i18n.t.noMessagesYet}</h2>
        <p>{i18n.t.askAnything}</p>
      </div>
    {:else}
      {#if virtualWindow.topSpacerHeight > 0}
        <div class="conversation-spacer" style:height="{virtualWindow.topSpacerHeight}px"></div>
      {/if}

      {#each visibleMessages as message (message.id)}
        <div class="message-row" class:message-row-highlighted={highlightedMessageId === message.id} data-message-id={message.id} use:measureRow={message.id}>
          <MessageBubble {message} {onAction} {disabled} />
        </div>
      {/each}

      {#if virtualWindow.bottomSpacerHeight > 0}
        <div class="conversation-spacer" style:height="{virtualWindow.bottomSpacerHeight}px"></div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .conversation-viewport {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: oklch(0.85 0 0) transparent;
  }

  .conversation-viewport::-webkit-scrollbar {
    width: 2px;
  }

  .conversation-viewport::-webkit-scrollbar-track {
    background: transparent;
  }

  .conversation-viewport::-webkit-scrollbar-thumb {
    background: oklch(0.85 0 0);
    border-radius: 9999px;
  }

  .conversation-viewport::-webkit-scrollbar-thumb:hover {
    background: oklch(0.72 0 0);
  }

  .conversation-stack {
    width: min(56rem, 100%);
    margin: 0 auto;
    padding: 1rem 1rem 1.25rem;
    box-sizing: border-box;
  }

  .conversation-spacer {
    width: 100%;
    pointer-events: none;
  }

  .message-row {
    box-sizing: border-box;
    padding-bottom: 1.2rem;
    transition: background-color 180ms ease, box-shadow 180ms ease;
    border-radius: 0.9rem;
  }

  .message-row-highlighted {
    background: hsl(var(--primary) / 0.08);
    box-shadow: 0 0 0 1px hsl(var(--primary) / 0.18);
  }

  .loading-older-state {
    margin: 0 auto 0.8rem;
    font-size: 0.78rem;
    color: var(--muted-foreground);
    text-align: center;
  }

  .empty-message-state {
    border: 1px dashed var(--border);
    border-radius: 0.9rem;
    background: var(--card);
    padding: 1rem 1.1rem;
  }

  .empty-message-state h2 {
    margin: 0;
    font-size: 0.95rem;
    color: var(--foreground);
    font-weight: 620;
  }

  .empty-message-state p {
    margin: 0.35rem 0 0;
    font-size: 0.84rem;
    color: var(--muted-foreground);
  }

  @media (max-width: 640px) {
    .conversation-stack {
      padding: 0.75rem 0.7rem 1rem;
    }

    .message-row {
      padding-bottom: 0.95rem;
    }
  }
</style>
