<script lang="ts">
  import { Sidebar, SidebarContent } from '$lib/components/ui/sidebar';
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
  <SidebarContent class="px-2 pt-2">
    <ConversationList
      bind:activeId={activeConversationId}
      onSelect={onConversationSelect}
    />
  </SidebarContent>

  <UserMenu />
</Sidebar>
