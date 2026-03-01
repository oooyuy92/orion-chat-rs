<script lang="ts">
  import ConversationList from '$lib/components/sidebar/ConversationList.svelte';
  import { getSidebarOpen, toggleSidebar } from '$lib/stores/ui.svelte';

  let activeConversationId = $state('');

  function handleSelect(id: string) {
    activeConversationId = id;
  }
</script>

<div class="flex h-full w-full">
  {#if getSidebarOpen()}
    <aside class="w-64 flex-shrink-0 border-r" style="border-color: var(--border);">
      <ConversationList bind:activeId={activeConversationId} onSelect={handleSelect} />
    </aside>
  {/if}

  <main class="flex-1 flex flex-col min-w-0">
    <header
      class="flex items-center gap-3 px-4 py-2 border-b"
      style="border-color: var(--border); background-color: var(--bg-secondary);"
    >
      <button
        onclick={toggleSidebar}
        class="p-1 rounded cursor-pointer"
        style="background: none; border: none; color: var(--text-secondary);"
        aria-label="Toggle sidebar"
      >
        &#9776;
      </button>
      <span class="text-sm font-medium" style="color: var(--text-primary);">Orion Chat</span>
    </header>

    <div class="flex-1 flex items-center justify-center" style="color: var(--text-secondary);">
      {#if !activeConversationId}
        <p class="text-sm">Select or create a conversation to start chatting</p>
      {/if}
    </div>
  </main>
</div>
