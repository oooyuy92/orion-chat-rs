<script lang="ts">
  import { Sidebar, SidebarContent } from '$lib/components/ui/sidebar';
  import ConversationList from '$lib/components/sidebar/ConversationList.svelte';
  import UserMenu from '$lib/components/sidebar/UserMenu.svelte';

  export type ConversationSelection = {
    conversationId: string;
    messageId?: string | null;
  };

  let {
    activeConversationId = $bindable(''),
    onConversationSelect,
  }: {
    activeConversationId?: string;
    onConversationSelect: (selection: ConversationSelection) => void;
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
