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

  function handleChange(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    selected = value;
    onSelect?.(value);
  }
</script>

<select
  value={selected}
  onchange={handleChange}
  class="rounded-lg px-3 py-1.5 text-sm outline-none cursor-pointer"
  style="
    background-color: var(--bg-secondary);
    color: var(--text-primary);
    border: 1px solid var(--border);
  "
>
  <option value="" disabled>Select a model</option>
  {#each models as model (model.id)}
    <option value={model.id}>{model.name}</option>
  {/each}
</select>
