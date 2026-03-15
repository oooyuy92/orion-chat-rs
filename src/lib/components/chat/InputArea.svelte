<script lang="ts">
  import type { ModelGroup, ProviderType } from '$lib/types';
  import ModelSelector from './ModelSelector.svelte';
  import ModelParamsPopover from './ModelParamsPopover.svelte';
  import ComboSelector from './ComboSelector.svelte';
  import { i18n } from '$lib/stores/i18n.svelte';

  const PASTE_THRESHOLD = 500;

  let {
    disabled = false,
    disabledReason = '',
    onSend,
    onGroupSend,
    onModelSelect,
    onStop,
    suggestions = [],
    modelGroups = [],
    selectedModelId = $bindable(''),
  }: {
    disabled?: boolean;
    disabledReason?: string;
    onSend: (content: string) => void;
    onGroupSend?: (content: string, modelIds: string[]) => void;
    onModelSelect?: (modelId: string) => void;
    onStop?: () => void;
    suggestions?: string[];
    modelGroups?: ModelGroup[];
    selectedModelId?: string;
  } = $props();

  let editorEl: HTMLDivElement | undefined = $state();
  let hasContent = $state(false);
  const pastedBlocks = new Map<string, string>();
  let activeComboModelIds = $state<string[] | null>(null);

  const currentProviderType = $derived.by(() => {
    for (const group of modelGroups) {
      if (group.models.some((m) => m.id === selectedModelId)) {
        return group.providerType;
      }
    }
    return 'openaiCompat' as ProviderType;
  });

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      submit();
    }
  }

  function handlePaste(event: ClipboardEvent) {
    event.preventDefault();
    const text = event.clipboardData?.getData('text/plain') ?? '';
    if (!text) return;

    if (text.length > PASTE_THRESHOLD) {
      const id = crypto.randomUUID();
      pastedBlocks.set(id, text);

      const span = document.createElement('span');
      span.className = 'paste-ref';
      span.contentEditable = 'false';
      span.dataset.pasteId = id;
      span.textContent = i18n.pasteLabel(text.length);

      insertNodeAtCursor(span);
    } else {
      document.execCommand('insertText', false, text);
    }
    updateHasContent();
  }

  function insertNodeAtCursor(node: Node) {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return;
    const range = sel.getRangeAt(0);
    range.deleteContents();
    range.insertNode(node);
    range.setStartAfter(node);
    range.collapse(true);
    sel.removeAllRanges();
    sel.addRange(range);
  }

  function handleInput() {
    updateHasContent();
  }

  function updateHasContent() {
    hasContent = !!(editorEl?.textContent?.trim());
  }

  function getContent(): string {
    if (!editorEl) return '';
    let result = '';
    function walk(node: Node) {
      if (node.nodeType === Node.TEXT_NODE) {
        result += node.textContent ?? '';
      } else if (node.nodeType === Node.ELEMENT_NODE) {
        const el = node as HTMLElement;
        if (el.classList.contains('paste-ref')) {
          const id = el.dataset.pasteId;
          if (id && pastedBlocks.has(id)) {
            const text = pastedBlocks.get(id)!;
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
    editorEl.childNodes.forEach((child) => walk(child));
    return result;
  }

  function submit() {
    const content = getContent().trim();
    if (!content || disabled) return;
    if (activeComboModelIds && activeComboModelIds.length > 1) {
      onGroupSend?.(content, activeComboModelIds);
    } else {
      onSend(content);
    }
    if (editorEl) editorEl.innerHTML = '';
    pastedBlocks.clear();
    hasContent = false;
  }

  function submitSuggestion(prompt: string) {
    if (disabled) return;
    if (editorEl) editorEl.textContent = prompt;
    submit();
  }
</script>

<div class="composer-shell">
  {#if suggestions.length > 0}
    <div class="suggestion-row">
      {#each suggestions as suggestion}
        <button
          class="suggestion-chip"
          disabled={disabled}
          onclick={() => submitSuggestion(suggestion)}
        >
          {suggestion}
        </button>
      {/each}
    </div>
  {/if}

  <div class="model-row">
    <ModelSelector {modelGroups} bind:selected={selectedModelId} onSelect={onModelSelect} />
    <ModelParamsPopover modelId={selectedModelId} providerType={currentProviderType} {disabled} />
    <ComboSelector
      {modelGroups}
      {disabled}
      {activeComboModelIds}
      onSelectCombo={(modelIds) => (activeComboModelIds = modelIds)}
      onClearCombo={() => (activeComboModelIds = null)}
    />
  </div>

  <div class="input-group">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      bind:this={editorEl}
      contenteditable={!disabled}
      class="input-field"
      class:empty={!hasContent}
      role="textbox"
      tabindex="0"
      oninput={handleInput}
      onpaste={handlePaste}
      onkeydown={handleKeydown}
      data-placeholder={i18n.t.inputPlaceholder}
    ></div>

    <div class="input-actions">
      {#if disabled}
        <button
          class="send-button stop"
          type="button"
          onclick={() => onStop?.()}
          aria-label={i18n.t.stop}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
            <rect x="4" y="4" width="16" height="16" rx="2" />
          </svg>
        </button>
      {:else}
        <button
          class="send-button"
          type="button"
          onclick={submit}
          disabled={!hasContent}
          aria-label={i18n.t.send}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="19" x2="12" y2="5"/>
            <polyline points="5 12 12 5 19 12"/>
          </svg>
        </button>
      {/if}
    </div>

    {#if disabledReason}
      <p class="input-hint">{disabledReason}</p>
    {/if}
  </div>
</div>

<style>
  .composer-shell {
    border-top: 1px solid var(--border);
    background: var(--card);
    padding: 0.75rem 1rem 1rem;
    flex-shrink: 0;
  }

  .suggestion-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    overflow-x: auto;
    padding-bottom: 0.75rem;
    scrollbar-width: none;
  }

  .suggestion-row::-webkit-scrollbar {
    display: none;
  }

  .suggestion-chip {
    white-space: nowrap;
    border: 1px solid var(--border);
    background: var(--background);
    color: var(--foreground);
    border-radius: 9999px;
    font-size: 0.8125rem;
    font-weight: 500;
    padding: 0.42rem 0.82rem;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .suggestion-chip:hover:enabled {
    background: var(--muted);
  }

  .suggestion-chip:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .model-row {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding-bottom: 0.5rem;
  }

  .input-group {
    border: 1px solid var(--border);
    border-radius: 0.9rem;
    background: var(--secondary);
    box-shadow: 0 1px 2px 0 rgb(0 0 0 / 0.03);
    overflow: hidden;
    transition: all 0.15s ease;
  }

  .input-group:has(.input-field:focus) {
    border-color: var(--ring);
  }

  .input-field {
    width: 100%;
    min-height: 4rem;
    max-height: 12rem;
    overflow-y: auto;
    padding: 0.85rem 0.9rem 0.45rem;
    font-size: 0.9rem;
    line-height: 1.4;
    color: var(--foreground);
    outline: none;
    box-sizing: border-box;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .input-field.empty::before {
    content: attr(data-placeholder);
    color: var(--muted-foreground);
    pointer-events: none;
  }

  .input-field :global(.paste-ref) {
    color: oklch(0.5 0.18 250);
  }

  .input-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 0 0.5rem 0.55rem 0.55rem;
  }

  .send-button {
    border: none;
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
    background: var(--primary);
    color: var(--primary-foreground);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: opacity 0.15s ease;
    flex-shrink: 0;
  }

  .send-button:hover:enabled {
    opacity: 0.85;
  }

  .send-button:disabled {
    cursor: not-allowed;
    opacity: 0.35;
  }

  .send-button.stop {
    background: var(--muted);
    color: var(--foreground);
  }

  .input-hint {
    margin: 0;
    padding: 0 0.75rem 0.65rem;
    color: var(--muted-foreground);
    font-size: 0.76rem;
    line-height: 1.35;
  }

  @media (max-width: 640px) {
    .composer-shell {
      padding: 0.65rem 0.7rem 0.75rem;
    }

    .suggestion-chip {
      font-size: 0.75rem;
      padding: 0.38rem 0.7rem;
    }
  }
</style>
