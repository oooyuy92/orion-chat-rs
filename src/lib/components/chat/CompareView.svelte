<script lang="ts">
  import type { Message } from '$lib/types';
  import MessageBubble from './MessageBubble.svelte';

  let {
    responses,
  }: {
    responses: { modelId: string; modelName: string; message: Message }[];
  } = $props();
</script>

<div
  class="flex gap-4 overflow-x-auto p-4"
  style="min-height: 0;"
>
  {#each responses as resp (resp.modelId)}
    <div
      class="flex-shrink-0 rounded-xl flex flex-col"
      style="
        width: 380px;
        border: 1px solid var(--border);
        background-color: var(--bg-secondary);
      "
    >
      <div
        class="px-4 py-2 text-sm font-semibold"
        style="
          border-bottom: 1px solid var(--border);
          color: var(--text-primary);
        "
      >
        {resp.modelName}
      </div>
      <div class="p-3 flex-1 overflow-y-auto">
        <MessageBubble message={resp.message} />
      </div>
    </div>
  {/each}
</div>
