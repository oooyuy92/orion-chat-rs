<script lang="ts">
  import type { Message } from '$lib/types';
  import { renderMarkdown } from '$lib/utils/markdown';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import CheckIcon from '@lucide/svelte/icons/check';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import MessageSquarePlusIcon from '@lucide/svelte/icons/message-square-plus';
  import { i18n } from '$lib/stores/i18n.svelte';

  type MessageAction =
    | { type: 'delete'; messageId: string }
    | { type: 'resend'; messageId: string }
    | { type: 'editResend'; messageId: string; content: string }
    | { type: 'regenerate'; messageId: string; modelId: string | null }
    | { type: 'generateVersion'; messageId: string }
    | { type: 'switchVersion'; versionGroupId: string; versionNumber: number };

  type Props = {
    message: Message;
    onAction?: (action: MessageAction) => void;
    disabled?: boolean;
  };

  let { message, onAction, disabled = false }: Props = $props();

  const isUser = $derived(message.role === 'user');
  const isLoading = $derived(
    !isUser && message.status === 'streaming' && !message.content && !message.reasoning,
  );
  const markdownContent = $derived(
    message.role === 'assistant' ? renderMarkdown(message.content) : message.content,
  );
  const renderedReasoning = $derived(message.reasoning ? renderMarkdown(message.reasoning) : '');
  const showActions = $derived(isUser && !disabled && (message.status === 'done' || message.status === 'error'));
  const showAssistantActions = $derived(!isUser && !disabled && (message.status === 'done' || message.status === 'error'));

  // Parse user message for paste-ref markers: <<paste:LENGTH>>...text...<</paste>>
  const PASTE_RE = /<<paste:(\d+)>>([\s\S]*?)<\/paste>>/g;
  const hasPasteRef = $derived(isUser && PASTE_RE.test(message.content));

  type ContentSegment = { type: 'text'; value: string } | { type: 'paste'; length: number; value: string };
  const userContentSegments = $derived.by((): ContentSegment[] => {
    if (!isUser) return [];
    PASTE_RE.lastIndex = 0;
    const segments: ContentSegment[] = [];
    let lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = PASTE_RE.exec(message.content)) !== null) {
      if (match.index > lastIndex) {
        segments.push({ type: 'text', value: message.content.slice(lastIndex, match.index) });
      }
      segments.push({ type: 'paste', length: parseInt(match[1]), value: match[2] });
      lastIndex = match.index + match[0].length;
    }
    if (lastIndex < message.content.length) {
      segments.push({ type: 'text', value: message.content.slice(lastIndex) });
    }
    return segments;
  });

  const PASTE_THRESHOLD = 500;

  let showReasoning = $state(false);
  let viewingPasteText = $state('');
  let isEditing = $state(false);
  let editEditorEl: HTMLDivElement | undefined = $state();
  let copied = $state(false);
  let isResending = $state(false);
  let isHovered = $state(false);
  const editPastedBlocks = new Map<string, string>();

  // Reset resending state when streaming ends (disabled goes back to false)
  $effect(() => {
    if (!disabled && isResending) {
      isResending = false;
    }
  });


  function startEdit() {
    isEditing = true;
    editPastedBlocks.clear();
    requestAnimationFrame(() => {
      if (!editEditorEl) return;
      populateEditor(editEditorEl, message.content);
      editEditorEl.focus();
    });
  }

  function populateEditor(el: HTMLDivElement, content: string) {
    el.innerHTML = '';
    const re = /<<paste:(\d+)>>([\s\S]*?)<\/paste>>/g;
    let lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = re.exec(content)) !== null) {
      if (match.index > lastIndex) {
        el.appendChild(document.createTextNode(content.slice(lastIndex, match.index)));
      }
      const id = crypto.randomUUID();
      editPastedBlocks.set(id, match[2]);
      const span = document.createElement('span');
      span.className = 'paste-ref';
      span.contentEditable = 'false';
      span.dataset.pasteId = id;
      span.textContent = i18n.pasteLabel(parseInt(match[1], 10));
      el.appendChild(span);
      lastIndex = match.index + match[0].length;
    }
    if (lastIndex < content.length) {
      el.appendChild(document.createTextNode(content.slice(lastIndex)));
    }
  }

  function cancelEdit() {
    isEditing = false;
    editPastedBlocks.clear();
  }

  function confirmEdit() {
    const content = getEditContent().trim();
    if (!content) return;
    isEditing = false;
    isResending = true;
    onAction?.({ type: 'editResend', messageId: message.id, content });
    editPastedBlocks.clear();
  }

  function handleEditPaste(event: ClipboardEvent) {
    event.preventDefault();
    const text = event.clipboardData?.getData('text/plain') ?? '';
    if (!text) return;

    if (text.length > PASTE_THRESHOLD) {
      const id = crypto.randomUUID();
      editPastedBlocks.set(id, text);

      const span = document.createElement('span');
      span.className = 'paste-ref';
      span.contentEditable = 'false';
      span.dataset.pasteId = id;
      span.textContent = i18n.pasteLabel(text.length);

      const sel = window.getSelection();
      if (sel && sel.rangeCount > 0) {
        const range = sel.getRangeAt(0);
        range.deleteContents();
        range.insertNode(span);
        range.setStartAfter(span);
        range.collapse(true);
        sel.removeAllRanges();
        sel.addRange(range);
      }
    } else {
      document.execCommand('insertText', false, text);
    }
  }

  function getEditContent(): string {
    if (!editEditorEl) return '';
    let result = '';
    function walk(node: Node) {
      if (node.nodeType === Node.TEXT_NODE) {
        result += node.textContent ?? '';
      } else if (node.nodeType === Node.ELEMENT_NODE) {
        const el = node as HTMLElement;
        if (el.classList.contains('paste-ref')) {
          const id = el.dataset.pasteId;
          if (id && editPastedBlocks.has(id)) {
            const text = editPastedBlocks.get(id)!;
            result += `<<paste:${text.length}>>${text}<</paste>>`;
          } else {
            result += el.textContent ?? '';
          }
        } else if (el.tagName === 'BR') {
          result += '\n';
        } else if (el.tagName === 'DIV' || el.tagName === 'P') {
          if (result.length > 0 && !result.endsWith('\n')) {
            result += '\n';
          }
          el.childNodes.forEach((child) => walk(child));
        } else {
          el.childNodes.forEach((child) => walk(child));
        }
      }
    }
    editEditorEl.childNodes.forEach((child) => walk(child));
    return result;
  }

  function resend() {
    isResending = true;
    onAction?.({ type: 'resend', messageId: message.id });
  }

  function regenerate() {
    isResending = true;
    onAction?.({ type: 'regenerate', messageId: message.id, modelId: message.modelId });
  }

  function generateVersion() {
    onAction?.({ type: 'generateVersion', messageId: message.id });
  }

  function switchVersion(versionNumber: number) {
    if (versionNumber === message.versionNumber) return;
    const groupId = message.versionGroupId || message.id;
    onAction?.({ type: 'switchVersion', versionGroupId: groupId, versionNumber });
  }

  function deleteMessage() {
    onAction?.({ type: 'delete', messageId: message.id });
  }

  async function copyToClipboard() {
    try {
      await navigator.clipboard.writeText(message.content);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch (e) {
      console.error('Failed to copy:', e);
    }
  }
</script>

{#if isUser}
  <!-- 用户消息：右对齐气泡，浅灰背景 -->
  <div
    class="flex w-full max-w-[95%] ml-auto justify-end"
    role="presentation"
    onmouseenter={() => (isHovered = true)}
    onmouseleave={() => (isHovered = false)}
  >
    <div class="flex w-fit max-w-full flex-col gap-1">
      {#if isEditing}
        <!-- Edit mode -->
        <div class="flex flex-col gap-2 w-full min-w-[300px]">
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            bind:this={editEditorEl}
            contenteditable="true"
            class="edit-field"
            onpaste={handleEditPaste}
            onkeydown={(e) => {
              if (e.key === 'Escape') cancelEdit();
            }}
          ></div>
          <div class="flex justify-end gap-2">
            <button
              class="rounded-md px-3 py-1.5 text-xs text-muted-foreground hover:bg-muted cursor-pointer"
              onclick={cancelEdit}
            >
              {i18n.t.cancel}
            </button>
            <button
              class="rounded-md bg-primary px-3 py-1.5 text-xs text-primary-foreground hover:bg-primary/90 cursor-pointer disabled:opacity-50"
              onclick={confirmEdit}
            >
              {i18n.t.resend}
            </button>
          </div>
        </div>
      {:else}
        <!-- Normal mode -->
        {#if hasPasteRef}
          <div class="rounded-lg bg-secondary px-4 py-3 text-sm text-foreground">
            {#each userContentSegments as seg}
              {#if seg.type === 'text'}
                {seg.value}
              {:else}
                <button
                  class="paste-tag"
                  onclick={() => (viewingPasteText = seg.value)}
                >
                  {i18n.pasteLabel(seg.length)}
                </button>
              {/if}
            {/each}
          </div>
        {:else}
          <div class="rounded-lg bg-secondary px-4 py-3 text-sm text-foreground">
            {message.content}
          </div>
        {/if}

        {#if message.status === 'error'}
          <div class="text-xs text-destructive px-1">{i18n.t.messageGenerationFailed}</div>
        {/if}

        <!-- Action buttons -->
        {#if isResending}
          <div class="flex justify-end">
            <span class="rounded p-1 text-muted-foreground">
              <RefreshCwIcon size={14} class="animate-spin" />
            </span>
          </div>
        {:else if showActions}
          <div
            class="flex justify-end gap-1 transition-opacity duration-200 {isHovered ? '' : 'pointer-events-none'}"
            style="opacity: {isHovered ? 1 : 0}"
          >
            <button
              class="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
              title={i18n.t.edit}
              onclick={startEdit}
            >
              <PencilIcon size={14} />
            </button>
            <button
              class="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
              title={i18n.t.resend}
              onclick={resend}
            >
              <RefreshCwIcon size={14} />
            </button>
            <button
              class="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
              title={copied ? i18n.t.copied : i18n.t.copy}
              onclick={copyToClipboard}
            >
              {#if copied}
                <CheckIcon size={14} class="text-green-500" />
              {:else}
                <CopyIcon size={14} />
              {/if}
            </button>
            <button
              class="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
              title={i18n.t.delete}
              onclick={deleteMessage}
            >
              <Trash2Icon size={14} />
            </button>
          </div>
        {/if}
      {/if}
    </div>
  </div>
{:else}
  <!-- 助手消息：左对齐纯文本，无背景 -->
  <div
    class="flex w-full max-w-[95%]"
    role="presentation"
    onmouseenter={() => (isHovered = true)}
    onmouseleave={() => (isHovered = false)}
  >
    <div class="flex w-fit max-w-full flex-col gap-2">
      <!-- Version tabs -->
      {#if message.totalVersions > 1}
        <div class="flex items-center gap-1 flex-wrap">
          {#each Array.from({ length: message.totalVersions }, (_, i) => i + 1) as v}
            <button
              class="rounded px-2 py-0.5 text-xs cursor-pointer {v === message.versionNumber
                ? 'bg-foreground text-background font-medium'
                : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
              onclick={() => switchVersion(v)}
            >
              v{v}
            </button>
          {/each}
        </div>
      {/if}

      {#if isLoading}
        <div class="loading-indicator">
          <span class="dot"></span>
          <span class="dot"></span>
          <span class="dot"></span>
        </div>
      {/if}

      {#if message.reasoning}
        <button
          class="w-fit rounded-full border border-border bg-background px-3 py-1 text-xs text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
          onclick={() => (showReasoning = !showReasoning)}
        >
          {showReasoning ? i18n.t.hideReasoning : i18n.t.showReasoning}
        </button>

        {#if showReasoning}
          <div class="rounded-xl border border-border bg-muted px-3 py-2.5 text-xs text-muted-foreground">
            <div class="reasoning-markdown">{@html renderedReasoning}</div>
          </div>
        {/if}
      {/if}

      {#if message.content}
        <div class="text-sm text-foreground">
          <div class="message-markdown">{@html markdownContent}</div>
        </div>
      {/if}

      {#if message.status === 'error'}
        <div class="text-xs text-destructive px-1">{i18n.t.messageGenerationFailed}</div>
      {/if}

      <!-- Assistant action buttons -->
      {#if isResending}
        <div class="flex justify-start">
          <span class="rounded p-1 text-muted-foreground">
            <RefreshCwIcon size={14} class="animate-spin" />
          </span>
        </div>
      {:else if showAssistantActions}
        <div
          class="flex justify-start gap-1 transition-opacity duration-200 {isHovered ? '' : 'pointer-events-none'}"
          style="opacity: {isHovered ? 1 : 0}"
        >
          <button
            class="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
            title={i18n.t.regenerate}
            onclick={regenerate}
          >
            <RefreshCwIcon size={14} />
          </button>
          <button
            class="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
            title={i18n.t.generateNewVersion}
            onclick={generateVersion}
          >
            <MessageSquarePlusIcon size={14} />
          </button>
          <button
            class="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
            title={copied ? i18n.t.copied : i18n.t.copy}
            onclick={copyToClipboard}
          >
            {#if copied}
              <CheckIcon size={14} class="text-green-500" />
            {:else}
              <CopyIcon size={14} />
            {/if}
          </button>
          <button
            class="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
            title={i18n.t.delete}
            onclick={deleteMessage}
          >
            <Trash2Icon size={14} />
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

{#if viewingPasteText}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="content-overlay" onclick={() => (viewingPasteText = '')} onkeydown={(e) => { if (e.key === 'Escape') viewingPasteText = ''; }}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="content-modal" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <div class="content-body">{viewingPasteText}</div>
    </div>
  </div>
{/if}

<style>
  .loading-indicator {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 0;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--muted-foreground);
    opacity: 0.4;
    animation: dot-pulse 1.4s ease-in-out infinite;
  }

  .dot:nth-child(2) {
    animation-delay: 0.2s;
  }

  .dot:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes dot-pulse {
    0%, 80%, 100% {
      opacity: 0.4;
      transform: scale(1);
    }
    40% {
      opacity: 1;
      transform: scale(1.2);
    }
  }

  :global(.message-markdown > :first-child) {
    margin-top: 0;
  }

  :global(.message-markdown > :last-child) {
    margin-bottom: 0;
  }

  :global(.message-markdown p) {
    margin: 0 0 0.55rem;
  }

  :global(.message-markdown h1),
  :global(.message-markdown h2),
  :global(.message-markdown h3) {
    margin: 0.7rem 0 0.45rem;
    font-weight: 680;
    line-height: 1.3;
  }

  :global(.message-markdown h1) {
    font-size: 1.45rem;
  }

  :global(.message-markdown h2) {
    font-size: 1.2rem;
  }

  :global(.message-markdown h3) {
    font-size: 1.04rem;
  }

  :global(.message-markdown ul),
  :global(.message-markdown ol) {
    margin: 0.35rem 0 0.55rem;
    padding-left: 1.2rem;
  }

  :global(.message-markdown li) {
    margin: 0.22rem 0;
  }

  :global(.message-markdown code) {
    background: var(--muted);
    border: 1px solid var(--border);
    border-radius: 0.33rem;
    padding: 0.05rem 0.3rem;
    font-size: 0.82em;
  }

  :global(.message-markdown pre) {
    overflow-x: auto;
    background: #f5f5f5;
    border: 1px solid var(--border);
    border-radius: 0.65rem;
    padding: 0.7rem;
    margin: 0.65rem 0;
  }

  :global(.message-markdown pre code) {
    background: transparent;
    border: none;
    padding: 0;
    border-radius: 0;
  }

  :global(.reasoning-markdown p:last-child) {
    margin-bottom: 0;
  }

  .paste-tag {
    color: oklch(0.5 0.18 250);
    cursor: pointer;
    background: none;
    border: none;
    padding: 0;
    font: inherit;
  }

  .paste-tag:hover {
    text-decoration: underline;
  }

  .edit-field {
    width: 100%;
    max-height: 50vh;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    background: var(--background);
    padding: 0.75rem 1rem;
    font-size: 0.875rem;
    line-height: 1.4;
    color: var(--foreground);
    outline: none;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .edit-field:focus {
    border-color: var(--ring);
  }

  .edit-field :global(.paste-ref) {
    color: oklch(0.5 0.18 250);
  }

  .content-overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .content-modal {
    background: white;
    border-radius: 0.75rem;
    width: min(48rem, 90vw);
    max-height: 80vh;
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .content-body {
    padding: 1.5rem;
    font-size: 0.875rem;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
    color: #1a1a1a;
  }
</style>
