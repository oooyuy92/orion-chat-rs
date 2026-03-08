<script lang="ts">
  import type {
    ProviderType,
    CommonParams,
    ProviderParams,
    AnthropicThinking,
    AnthropicEffort,
    ReasoningEffort,
    GeminiThinkingLevel,
  } from '$lib/types';
  import {
    alignProviderParams,
    defaultProviderParams,
    getModelParams,
    setModelParams,
    deleteModelParams,
  } from '$lib/stores/modelParams';
  import { Popover, PopoverContent, PopoverTrigger } from '$lib/components/ui/popover';
  import { Slider } from '$lib/components/ui/slider';
  import { Input } from '$lib/components/ui/input';
  import { Button } from '$lib/components/ui/button';
  import SlidersHorizontalIcon from '@lucide/svelte/icons/sliders-horizontal';
  import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
  import { i18n } from '$lib/stores/i18n.svelte';

  let {
    modelId,
    providerType,
    disabled = false,
  }: {
    modelId: string;
    providerType: ProviderType;
    disabled?: boolean;
  } = $props();

  let common = $state<CommonParams>({});
  let providerParams = $state<ProviderParams>({ provider_type: 'openaiCompat' });
  let loaded = $state(false);

  // Reload params when modelId changes
  $effect(() => {
    if (modelId && providerType) {
      loaded = false;
      common = {};
      providerParams = defaultProviderParams(providerType);
      let cancelled = false;
      getModelParams(modelId, providerType).then((p) => {
        if (cancelled) return;
        common = p.common;
        providerParams = alignProviderParams(providerType, p.providerParams);
        loaded = true;
      });
      return () => {
        cancelled = true;
      };
    }
    common = {};
    providerParams = { provider_type: 'openaiCompat' };
    loaded = false;
  });

  function save() {
    if (!loaded || !modelId) return;
    setModelParams(modelId, { common, providerParams });
  }

  async function reset() {
    await deleteModelParams(modelId);
    common = {};
    providerParams = defaultProviderParams(providerType);
  }

  function providerTypeTag(): ProviderParams['provider_type'] {
    switch (providerType) {
      case 'anthropic': return 'anthropic';
      case 'gemini': return 'gemini';
      case 'ollama': return 'ollama';
      default: return 'openaiCompat';
    }
  }

  // ---- Common handlers ----

  function thinkingModeLabel(mode: string) {
    switch (mode) {
      case 'adaptive':
        return i18n.t.adaptive;
      case 'enabled':
        return i18n.t.enabled;
      default:
        return i18n.t.disabled;
    }
  }

  function levelLabel(value: string) {
    switch (value) {
      case 'low':
        return i18n.t.low;
      case 'medium':
        return i18n.t.medium;
      case 'high':
        return i18n.t.high;
      default:
        return i18n.t.default;
    }
  }

  function thinkLabel(value: boolean | null) {
    if (value === null) return i18n.t.default;
    return value ? i18n.t.on : i18n.t.off;
  }

  function setTemperature(v: number) { common = { ...common, temperature: v }; save(); }
  function setTopP(v: number) { common = { ...common, topP: v }; save(); }
  function setMaxTokens(e: Event) {
    const val = (e.target as HTMLInputElement).valueAsNumber;
    common = { ...common, maxTokens: Number.isNaN(val) ? null : val };
    save();
  }

  // ---- Anthropic helpers ----
  function getThinkingMode(): string {
    if (providerParams.provider_type !== 'anthropic') return 'disabled';
    const t = providerParams.thinking;
    if (!t) return 'disabled';
    return t.type;
  }
  function setThinkingMode(mode: string) {
    if (providerParams.provider_type !== 'anthropic') return;
    let thinking: AnthropicThinking | null;
    switch (mode) {
      case 'adaptive': thinking = { type: 'adaptive' }; break;
      case 'enabled': thinking = { type: 'enabled', budgetTokens: 10000 }; break;
      default: thinking = { type: 'disabled' }; break;
    }
    providerParams = { ...providerParams, thinking };
    save();
  }
  function setBudgetTokens(e: Event) {
    if (providerParams.provider_type !== 'anthropic') return;
    const val = (e.target as HTMLInputElement).valueAsNumber;
    if (Number.isNaN(val)) return;
    providerParams = { ...providerParams, thinking: { type: 'enabled', budgetTokens: val } };
    save();
  }
  function setAnthropicEffort(val: string) {
    if (providerParams.provider_type !== 'anthropic') return;
    providerParams = { ...providerParams, effort: (val || null) as AnthropicEffort | null };
    save();
  }
  function setAnthropicTopK(e: Event) {
    if (providerParams.provider_type !== 'anthropic') return;
    const val = (e.target as HTMLInputElement).valueAsNumber;
    providerParams = { ...providerParams, topK: Number.isNaN(val) ? null : val };
    save();
  }

  // ---- Gemini helpers ----
  function setGeminiThinkingLevel(val: string) {
    if (providerParams.provider_type !== 'gemini') return;
    providerParams = { ...providerParams, thinkingLevel: (val || null) as GeminiThinkingLevel | null };
    save();
  }
  function setGeminiThinkingBudget(e: Event) {
    if (providerParams.provider_type !== 'gemini') return;
    const val = (e.target as HTMLInputElement).valueAsNumber;
    providerParams = { ...providerParams, thinkingBudget: Number.isNaN(val) ? null : val };
    save();
  }

  // ---- OpenAI helpers ----
  function setFreqPenalty(v: number) {
    if (providerParams.provider_type !== 'openaiCompat') return;
    providerParams = { ...providerParams, frequencyPenalty: v };
    save();
  }
  function setPresPenalty(v: number) {
    if (providerParams.provider_type !== 'openaiCompat') return;
    providerParams = { ...providerParams, presencePenalty: v };
    save();
  }
  function setReasoningEffort(val: string) {
    if (providerParams.provider_type !== 'openaiCompat') return;
    providerParams = { ...providerParams, reasoningEffort: (val || null) as ReasoningEffort | null };
    save();
  }
  function setOpenAISeed(e: Event) {
    if (providerParams.provider_type !== 'openaiCompat') return;
    const val = (e.target as HTMLInputElement).valueAsNumber;
    providerParams = { ...providerParams, seed: Number.isNaN(val) ? null : val };
    save();
  }
  function setMaxCompletionTokens(e: Event) {
    if (providerParams.provider_type !== 'openaiCompat') return;
    const val = (e.target as HTMLInputElement).valueAsNumber;
    providerParams = { ...providerParams, maxCompletionTokens: Number.isNaN(val) ? null : val };
    save();
  }

  // ---- Ollama helpers ----
  function setOllamaThink(val: boolean) {
    if (providerParams.provider_type !== 'ollama') return;
    providerParams = { ...providerParams, think: val };
    save();
  }
  function setNumCtx(e: Event) {
    if (providerParams.provider_type !== 'ollama') return;
    const val = (e.target as HTMLInputElement).valueAsNumber;
    providerParams = { ...providerParams, numCtx: Number.isNaN(val) ? null : val };
    save();
  }
  function setRepeatPenalty(v: number) {
    if (providerParams.provider_type !== 'ollama') return;
    providerParams = { ...providerParams, repeatPenalty: v };
    save();
  }
  function setMinP(v: number) {
    if (providerParams.provider_type !== 'ollama') return;
    providerParams = { ...providerParams, minP: v };
    save();
  }
  function setKeepAlive(e: Event) {
    if (providerParams.provider_type !== 'ollama') return;
    const val = (e.target as HTMLInputElement).value;
    providerParams = { ...providerParams, keepAlive: val || null };
    save();
  }
</script>

<Popover>
  <PopoverTrigger>
    {#snippet child({ props })}
      <button {...props} class="params-trigger" disabled={disabled} aria-label={i18n.t.modelParameters}>
        <SlidersHorizontalIcon class="h-3.5 w-3.5" />
      </button>
    {/snippet}
  </PopoverTrigger>
  <PopoverContent class="params-popover" align="start">
    <div class="params-scroll">
      <div class="params-header">
        <span class="params-title">{i18n.t.parameters}</span>
        <button class="reset-btn" onclick={reset} aria-label={i18n.t.resetToDefaults}>
          <RotateCcwIcon class="h-3 w-3" />
          <span>{i18n.t.reset}</span>
        </button>
      </div>

      <!-- Common params -->
      <fieldset class="param-section">
        <legend class="section-label">{i18n.t.common}</legend>

        <div class="param-row">
          <div class="param-label-row">
            <span>{i18n.t.temperature}</span>
            <span class="param-value">{common.temperature ?? '—'}</span>
          </div>
          <Slider
            type="single"
            value={common.temperature ?? 1}
            min={0} max={2} step={0.1}
            onValueChange={setTemperature}
          />
        </div>

        <div class="param-row">
          <div class="param-label-row">
            <span>{i18n.t.topP}</span>
            <span class="param-value">{common.topP ?? '—'}</span>
          </div>
          <Slider
            type="single"
            value={common.topP ?? 1}
            min={0} max={1} step={0.01}
            onValueChange={setTopP}
          />
        </div>

        <div class="param-row">
          <span>{i18n.t.maxTokens}</span>
          <Input
            type="number"
            placeholder={i18n.t.default}
            value={common.maxTokens ?? ''}
            onchange={setMaxTokens}
            class="param-input"
          />
        </div>
      </fieldset>

      <!-- Anthropic-specific -->
      {#if providerParams.provider_type === 'anthropic'}
        <fieldset class="param-section">
          <legend class="section-label">Anthropic</legend>

          <div class="param-row">
            <span>{i18n.t.thinking}</span>
            <div class="btn-group">
              {#each ['disabled', 'adaptive', 'enabled'] as mode}
                <button
                  class="btn-option"
                  class:active={getThinkingMode() === mode}
                  onclick={() => setThinkingMode(mode)}
                >{thinkingModeLabel(mode)}</button>
              {/each}
            </div>
          </div>

          {#if getThinkingMode() === 'enabled'}
            <div class="param-row">
              <span>{i18n.t.budgetTokens}</span>
              <Input
                type="number"
                placeholder="10000"
                value={providerParams.thinking?.type === 'enabled' ? providerParams.thinking.budgetTokens : ''}
                onchange={setBudgetTokens}
                class="param-input"
              />
            </div>
          {/if}

          <div class="param-row">
            <span>{i18n.t.effort}</span>
            <div class="btn-group">
              {#each ['', 'low', 'medium', 'high'] as val}
                <button
                  class="btn-option"
                  class:active={String(providerParams.effort ?? '') === val}
                  onclick={() => setAnthropicEffort(val)}
                >{levelLabel(val)}</button>
              {/each}
            </div>
          </div>

          <div class="param-row">
            <span>Top K</span>
            <Input
              type="number"
              placeholder={i18n.t.default}
              value={providerParams.topK ?? ''}
              onchange={setAnthropicTopK}
              class="param-input"
            />
          </div>
        </fieldset>
      {/if}

      <!-- Gemini-specific -->
      {#if providerParams.provider_type === 'gemini'}
        <fieldset class="param-section">
          <legend class="section-label">Gemini</legend>

          <div class="param-row">
            <span>{i18n.t.thinkingLevel}</span>
            <div class="btn-group">
              {#each ['', 'low', 'medium', 'high'] as val}
                <button
                  class="btn-option"
                  class:active={String(providerParams.thinkingLevel ?? '') === val}
                  onclick={() => setGeminiThinkingLevel(val)}
                >{levelLabel(val)}</button>
              {/each}
            </div>
          </div>

          <div class="param-row">
            <span>{i18n.t.thinkingBudget}</span>
            <Input
              type="number"
              placeholder={i18n.t.default}
              value={providerParams.thinkingBudget ?? ''}
              onchange={setGeminiThinkingBudget}
              class="param-input"
            />
          </div>
        </fieldset>
      {/if}

      <!-- OpenAI-specific -->
      {#if providerParams.provider_type === 'openaiCompat'}
        <fieldset class="param-section">
          <legend class="section-label">OpenAI</legend>

          <div class="param-row">
            <div class="param-label-row">
              <span>{i18n.t.frequencyPenalty}</span>
              <span class="param-value">{providerParams.frequencyPenalty ?? '—'}</span>
            </div>
            <Slider
              type="single"
              value={providerParams.frequencyPenalty ?? 0}
              min={-2} max={2} step={0.1}
              onValueChange={setFreqPenalty}
            />
          </div>

          <div class="param-row">
            <div class="param-label-row">
              <span>{i18n.t.presencePenalty}</span>
              <span class="param-value">{providerParams.presencePenalty ?? '—'}</span>
            </div>
            <Slider
              type="single"
              value={providerParams.presencePenalty ?? 0}
              min={-2} max={2} step={0.1}
              onValueChange={setPresPenalty}
            />
          </div>

          <div class="param-row">
            <span>{i18n.t.reasoningEffort}</span>
            <div class="btn-group">
              {#each ['', 'low', 'medium', 'high'] as val}
                <button
                  class="btn-option"
                  class:active={String(providerParams.reasoningEffort ?? '') === val}
                  onclick={() => setReasoningEffort(val)}
                >{levelLabel(val)}</button>
              {/each}
            </div>
          </div>

          <div class="param-row">
            <span>{i18n.t.seed}</span>
            <Input
              type="number"
              placeholder={i18n.t.default}
              value={providerParams.seed ?? ''}
              onchange={setOpenAISeed}
              class="param-input"
            />
          </div>

          <div class="param-row">
            <span>{i18n.t.maxCompletionTokens}</span>
            <Input
              type="number"
              placeholder={i18n.t.default}
              value={providerParams.maxCompletionTokens ?? ''}
              onchange={setMaxCompletionTokens}
              class="param-input"
            />
          </div>
        </fieldset>
      {/if}

      <!-- Ollama-specific -->
      {#if providerParams.provider_type === 'ollama'}
        <fieldset class="param-section">
          <legend class="section-label">Ollama</legend>

          <div class="param-row">
            <span>{i18n.t.think}</span>
            <div class="btn-group">
              {#each [null, true, false] as val}
                <button
                  class="btn-option"
                  class:active={providerParams.think === val}
                  onclick={() => setOllamaThink(val as boolean)}
                >{thinkLabel(val)}</button>
              {/each}
            </div>
          </div>

          <div class="param-row">
            <span>{i18n.t.numCtx}</span>
            <Input
              type="number"
              placeholder={i18n.t.default}
              value={providerParams.numCtx ?? ''}
              onchange={setNumCtx}
              class="param-input"
            />
          </div>

          <div class="param-row">
            <div class="param-label-row">
              <span>{i18n.t.repeatPenalty}</span>
              <span class="param-value">{providerParams.repeatPenalty ?? '—'}</span>
            </div>
            <Slider
              type="single"
              value={providerParams.repeatPenalty ?? 1.1}
              min={0} max={2} step={0.05}
              onValueChange={setRepeatPenalty}
            />
          </div>

          <div class="param-row">
            <div class="param-label-row">
              <span>{i18n.t.minP}</span>
              <span class="param-value">{providerParams.minP ?? '—'}</span>
            </div>
            <Slider
              type="single"
              value={providerParams.minP ?? 0}
              min={0} max={1} step={0.01}
              onValueChange={setMinP}
            />
          </div>

          <div class="param-row">
            <span>{i18n.t.keepAlive}</span>
            <Input
              type="text"
              placeholder="5m"
              value={providerParams.keepAlive ?? ''}
              onchange={setKeepAlive}
              class="param-input"
            />
          </div>
        </fieldset>
      {/if}
    </div>
  </PopoverContent>
</Popover>

<style>
  .params-trigger {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    background: var(--background);
    color: var(--muted-foreground);
    border-radius: 0.5rem;
    padding: 0.3rem 0.45rem;
    cursor: pointer;
    transition: background-color 0.15s ease, color 0.15s ease;
  }

  .params-trigger:hover:not(:disabled) {
    background: var(--muted);
    color: var(--foreground);
  }

  .params-trigger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  :global(.params-popover) {
    width: 320px !important;
    max-height: 70vh;
    padding: 0 !important;
  }

  .params-scroll {
    max-height: 70vh;
    overflow-y: auto;
    padding: 0.75rem;
  }

  .params-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.6rem;
  }

  .params-title {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--foreground);
  }

  .reset-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    border: none;
    background: none;
    color: var(--muted-foreground);
    font-size: 0.75rem;
    cursor: pointer;
    padding: 0.2rem 0.35rem;
    border-radius: 0.25rem;
    transition: color 0.15s, background-color 0.15s;
  }

  .reset-btn:hover {
    color: var(--foreground);
    background: var(--muted);
  }

  .param-section {
    border: none;
    padding: 0;
    margin: 0 0 0.6rem;
  }

  .section-label {
    font-size: 0.6875rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted-foreground);
    margin-bottom: 0.4rem;
  }

  .param-row {
    margin-bottom: 0.55rem;
  }

  .param-row span {
    display: block;
    font-size: 0.8125rem;
    color: var(--foreground);
    margin-bottom: 0.25rem;
  }

  .param-label-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }

  .param-value {
    font-size: 0.75rem;
    color: var(--muted-foreground);
    font-variant-numeric: tabular-nums;
  }

  :global(.param-input) {
    height: 2rem !important;
    font-size: 0.8125rem !important;
  }

  .btn-group {
    display: flex;
    gap: 0;
    border: 1px solid var(--border);
    border-radius: 0.375rem;
    overflow: hidden;
  }

  .btn-option {
    flex: 1;
    border: none;
    background: var(--background);
    color: var(--muted-foreground);
    font-size: 0.75rem;
    padding: 0.3rem 0.5rem;
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
    text-transform: capitalize;
    border-right: 1px solid var(--border);
  }

  .btn-option:last-child {
    border-right: none;
  }

  .btn-option:hover {
    background: var(--muted);
  }

  .btn-option.active {
    background: var(--primary);
    color: var(--primary-foreground);
  }
</style>
