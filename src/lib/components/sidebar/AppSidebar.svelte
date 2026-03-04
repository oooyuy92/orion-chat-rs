<script lang="ts">
  import { Sidebar, SidebarHeader, SidebarContent } from '$lib/components/ui/sidebar';
  import ModelSelector from '$lib/components/chat/ModelSelector.svelte';
  import ConversationList from '$lib/components/sidebar/ConversationList.svelte';
  import UserMenu from '$lib/components/sidebar/UserMenu.svelte';
  import type { ModelGroup } from '$lib/types';

  let {
    modelGroups,
    selectedModelId = $bindable(''),
    activeConversationId = $bindable(''),
    onModelChange,
    onConversationSelect,
  }: {
    modelGroups: ModelGroup[];
    selectedModelId?: string;
    activeConversationId?: string;
    onModelChange?: (modelId: string) => void;
    onConversationSelect: (id: string) => void;
  } = $props();
</script>

<Sidebar>
  <SidebarHeader class="p-4">
    <ModelSelector
      {modelGroups}
      bind:selected={selectedModelId}
      onSelect={onModelChange}
    />
  </SidebarHeader>

  <SidebarContent class="px-2">
    <ConversationList
      bind:activeId={activeConversationId}
      onSelect={onConversationSelect}
    />
  </SidebarContent>

  <UserMenu />
</Sidebar>
