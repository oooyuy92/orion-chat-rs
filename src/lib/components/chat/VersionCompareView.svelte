<script lang="ts">
  import type { Message, ModelGroup } from '$lib/types';
  import { renderMarkdown } from '$lib/utils/markdown';
  import { resolveModelLabel } from '$lib/utils/modelDisplay';
  import { i18n } from '$lib/stores/i18n.svelte';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';

  let {
    versionMessages,
    modelGroups = [],
    onBack,
  }: {
    versionMessages: Message[];
    modelGroups?: ModelGroup[];
    onBack?: () => void;
  } = $props();

  function resolveModelName(modelId: string | null): string {
    if (!modelId) return 'Unknown';
    for (const group of modelGroups) {
      const model = group.models.find((m) => m.id === modelId);
      if (model) return resolveModelLabel(model);
    }
    return modelId;
  }
</script>

<div class="flex flex-col h-full min-h-0">
  <div class="flex items-center gap-3 px-4 py-3 border-b border-border bg-card flex-shrink-0">
    <button
      class="rounded p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
      title={i18n.t.backToChat}
      onclick={onBack}
    >
      <ArrowLeftIcon size={18} />
    </button>
    <span class="text-sm font-medium text-foreground">
      {i18n.t.compareVersions}
      <span class="text-muted-foreground font-normal ml-1">({versionMessages.length})</span>
    </span>
  </div>

  <div class="flex-1 min-h-0 overflow-x-auto overflow-y-hidden">
    <div class="flex gap-4 p-4 h-full min-h-0" style="min-width: max-content;">
      {#each versionMessages as msg, idx (msg.id)}
        <div
          class="flex-shrink-0 rounded-xl border border-border bg-card flex flex-col h-full min-h-0"
          style="width: 420px; max-width: 80vw;"
        >
          <div class="px-4 py-2.5 border-b border-border flex items-center justify-between flex-shrink-0">
            <span class="text-sm font-semibold text-foreground">v{msg.versionNumber}</span>
            <span class="text-xs text-muted-foreground">{resolveModelName(msg.modelId)}</span>
          </div>
          <div class="flex-1 overflow-y-auto p-4">
            {#if msg.reasoning}
              <div class="rounded-lg border border-border bg-muted px-3 py-2 text-xs text-muted-foreground mb-3">
                <div class="font-medium mb-1">{i18n.t.showReasoning}</div>
                <div class="reasoning-markdown">{@html renderMarkdown(msg.reasoning)}</div>
              </div>
            {/if}
            {#if msg.content}
              <div class="text-sm text-foreground message-markdown">{@html renderMarkdown(msg.content)}</div>
            {/if}
            {#if msg.status === 'streaming'}
              <div class="flex items-center gap-1.5 text-xs text-muted-foreground animate-pulse mt-2">
                <span class="w-1.5 h-1.5 rounded-full bg-current"></span>
                {i18n.t.generating}
              </div>
            {/if}
            {#if msg.status === 'error'}
              <div class="text-xs text-destructive mt-2">{i18n.t.messageGenerationFailed}</div>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>
