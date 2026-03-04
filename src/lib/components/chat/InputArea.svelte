<script lang="ts">
  import ModelSelector from './ModelSelector.svelte';
  import type { ModelGroup } from '$lib/types';

  let {
    disabled = false,
    disabledReason = '',
    onSend,
    suggestions = [],
    modelGroups = [],
    selectedModel = $bindable(''),
  }: {
    disabled?: boolean;
    disabledReason?: string;
    onSend: (content: string) => void;
    suggestions?: string[];
    modelGroups?: ModelGroup[];
    selectedModel?: string;
  } = $props();

  let text = $state('');

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      submit();
    }
  }

  function submit() {
    const trimmed = text.trim();
    if (!trimmed || disabled) {
      return;
    }

    onSend(trimmed);
    text = '';
  }

  function submitSuggestion(prompt: string) {
    if (disabled) {
      return;
    }

    text = prompt;
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

  <!-- InputGroup structure -->
  <div class="input-group">
    <textarea
      bind:value={text}
      onkeydown={handleKeydown}
      {disabled}
      placeholder="What would you like to know?"
      class="input-field"
    ></textarea>

    <div class="input-actions">
      <div class="tool-row" role="group" aria-label="Prompt tools">
        <button class="tool-button" type="button" disabled={disabled} title="Attach">
          +
        </button>
        <button class="tool-button" type="button" disabled={disabled} title="Voice input">
          &#8961;
        </button>
        <button class="tool-button wide" type="button" disabled={disabled} title="Search">
          Search
        </button>
      </div>

      <div class="actions-row">
        {#if modelGroups.length > 0}
          <ModelSelector {modelGroups} bind:selected={selectedModel} />
        {:else}
          <span class="model-hint">No model</span>
        {/if}

        <button
          class="send-button"
          type="button"
          onclick={submit}
          disabled={disabled || !text.trim()}
          aria-label="Submit"
        >
          &#8629;
        </button>
      </div>
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

  /* InputGroup structure */
  .input-group {
    border: 1px solid var(--input);
    border-radius: 0.9rem;
    background: var(--input);
    box-shadow: 0 1px 2px 0 rgb(0 0 0 / 0.05);
    overflow: hidden;
    transition: all 0.15s ease;
  }

  .input-group:has(textarea:focus-visible) {
    border-color: var(--ring);
    outline: 3px solid rgba(var(--ring-rgb), 0.15);
    outline-offset: 0;
  }

  .input-field {
    width: 100%;
    border: none;
    resize: none;
    background: transparent;
    color: var(--foreground);
    font-size: 0.9rem;
    line-height: 1.4;
    field-sizing: content;
    min-height: 4rem;
    max-height: 12rem;
    padding: 0.85rem 0.9rem 0.45rem;
    outline: none;
    box-sizing: border-box;
  }

  .input-field::placeholder {
    color: var(--muted-foreground);
  }

  .input-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0 0.5rem 0.55rem 0.55rem;
  }

  .tool-row,
  .actions-row {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .tool-button {
    border: none;
    background: transparent;
    color: var(--muted-foreground);
    min-width: 1.8rem;
    height: 1.8rem;
    border-radius: 0.45rem;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0 0.3rem;
  }

  .tool-button.wide {
    padding: 0 0.5rem;
    font-size: 0.8rem;
  }

  .tool-button:hover:enabled {
    background: var(--muted);
    color: var(--foreground);
  }

  .tool-button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .model-hint {
    color: var(--muted-foreground);
    font-size: 0.78rem;
    padding: 0 0.4rem;
  }

  .send-button {
    border: none;
    width: 2rem;
    height: 2rem;
    border-radius: 26px;
    background: var(--primary);
    color: var(--primary-foreground);
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: background-color 0.15s ease;
  }

  .send-button:hover:enabled {
    background: #27272a;
  }

  .send-button:disabled {
    cursor: not-allowed;
    opacity: 0.45;
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

    .input-actions {
      flex-wrap: wrap;
      row-gap: 0.45rem;
    }

    .actions-row {
      margin-left: auto;
    }
  }
</style>
