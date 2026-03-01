<script lang="ts">
  import type { ModelInfo } from '$lib/types';

  let {
    models,
    selected = $bindable(''),
    onSelect,
  }: {
    models: ModelInfo[];
    selected?: string;
    onSelect?: (modelId: string) => void;
  } = $props();

  function handleChange(event: Event) {
    const value = (event.target as HTMLSelectElement).value;
    selected = value;
    onSelect?.(value);
  }
</script>

<div class="model-selector" role="group" aria-label="Model selector">
  <span class="model-prefix" aria-hidden="true">*</span>
  <select value={selected} onchange={handleChange} class="model-select">
    <option value="" disabled>Select model</option>
    {#each models as model (model.id)}
      <option value={model.id}>{model.name}</option>
    {/each}
  </select>
</div>

<style>
  .model-selector {
    border: 1px solid var(--border);
    background: var(--background);
    border-radius: 0.5rem;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0 0.45rem;
    height: 2rem;
    min-width: 8rem;
    color: var(--foreground);
  }

  .model-prefix {
    font-size: 0.75rem;
    color: var(--muted-foreground);
  }

  .model-select {
    appearance: none;
    border: none;
    background: transparent;
    color: var(--foreground);
    font-size: 0.82rem;
    line-height: 1;
    min-width: 0;
    padding-right: 0.1rem;
    cursor: pointer;
    outline: none;
  }
</style>
