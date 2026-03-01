<script lang="ts">
  import { onMount } from 'svelte';
  import ConversationList from '$lib/components/sidebar/ConversationList.svelte';
  import MessageList from '$lib/components/chat/MessageList.svelte';
  import InputArea from '$lib/components/chat/InputArea.svelte';
  import ModelSelector from '$lib/components/chat/ModelSelector.svelte';
  import ProviderSettings from '$lib/components/settings/ProviderSettings.svelte';
  import { getSidebarOpen, toggleSidebar } from '$lib/stores/ui.svelte';
  import { api } from '$lib/utils/invoke';
  import type { ChatEvent } from '$lib/utils/invoke';
  import type { Message, ModelInfo } from '$lib/types';

  type View = 'chat' | 'settings';
  let currentView = $state<View>('chat');

  let activeConversationId = $state('');
  let messages = $state<Message[]>([]);
  let isStreaming = $state(false);
  let streamingMessageId = $state('');
  let currentModelId = $state('');
  let allModels = $state<ModelInfo[]>([]);

  async function loadModels() {
    try {
      const providers = await api.listProviders();
      const modelArrays = await Promise.all(
        providers.map((p) => api.fetchModels(p.id).catch(() => [] as ModelInfo[])),
      );
      allModels = modelArrays.flat();
      if (allModels.length > 0 && !currentModelId) {
        currentModelId = allModels[0].id;
      }
    } catch (e) {
      console.error('Failed to load models:', e);
    }
  }

  onMount(() => {
    loadModels();
  });

  function handleSelect(id: string) {
    activeConversationId = id;
    loadMessages(id);
  }

  async function loadMessages(conversationId: string) {
    if (!conversationId) {
      messages = [];
      return;
    }
    try {
      messages = await api.getMessages(conversationId);
    } catch (e) {
      console.error('Failed to load messages:', e);
      messages = [];
    }
  }

  function handleEvent(event: ChatEvent) {
    switch (event.type) {
      case 'started':
        streamingMessageId = event.messageId;
        break;
      case 'delta': {
        const idx = messages.findIndex((m) => m.id === streamingMessageId);
        if (idx !== -1) {
          messages[idx] = {
            ...messages[idx],
            content: messages[idx].content + event.content,
          };
        }
        break;
      }
      case 'reasoning': {
        const idx = messages.findIndex((m) => m.id === streamingMessageId);
        if (idx !== -1) {
          messages[idx] = {
            ...messages[idx],
            reasoning: (messages[idx].reasoning ?? '') + event.content,
          };
        }
        break;
      }
      case 'usage': {
        const idx = messages.findIndex((m) => m.id === streamingMessageId);
        if (idx !== -1) {
          messages[idx] = {
            ...messages[idx],
            tokenCount: event.promptTokens + event.completionTokens,
          };
        }
        break;
      }
      case 'finished': {
        const idx = messages.findIndex((m) => m.id === streamingMessageId);
        if (idx !== -1) {
          messages[idx] = { ...messages[idx], status: 'done' };
        }
        isStreaming = false;
        streamingMessageId = '';
        break;
      }
      case 'error': {
        console.error('Stream error:', event.message);
        const idx = messages.findIndex((m) => m.id === streamingMessageId);
        if (idx !== -1) {
          messages[idx] = {
            ...messages[idx],
            status: 'error',
            content: messages[idx].content || event.message,
          };
        }
        isStreaming = false;
        streamingMessageId = '';
        break;
      }
    }
  }

  async function handleSend(content: string) {
    if (!activeConversationId || !currentModelId) {
      console.warn('No conversation or model selected');
      return;
    }

    // Optimistic user message
    const userMsg: Message = {
      id: crypto.randomUUID(),
      conversationId: activeConversationId,
      role: 'user',
      content,
      reasoning: null,
      modelId: null,
      status: 'done',
      tokenCount: null,
      createdAt: new Date().toISOString(),
    };
    messages = [...messages, userMsg];

    // Placeholder assistant message
    const assistantMsg: Message = {
      id: crypto.randomUUID(),
      conversationId: activeConversationId,
      role: 'assistant',
      content: '',
      reasoning: null,
      modelId: currentModelId,
      status: 'streaming',
      tokenCount: null,
      createdAt: new Date().toISOString(),
    };
    messages = [...messages, assistantMsg];
    streamingMessageId = assistantMsg.id;
    isStreaming = true;

    try {
      const result = await api.sendMessage(
        activeConversationId,
        content,
        currentModelId,
        (event) => {
          // Update the streaming message ID to the real one from backend
          if (event.type === 'started') {
            const idx = messages.findIndex((m) => m.id === assistantMsg.id);
            if (idx !== -1) {
              messages[idx] = { ...messages[idx], id: event.messageId };
            }
            streamingMessageId = event.messageId;
          } else {
            handleEvent(event);
          }
        },
      );
      // Replace optimistic user message with real one from backend
      const userIdx = messages.findIndex((m) => m.id === userMsg.id);
      if (userIdx !== -1 && result) {
        // The backend returns the assistant message; user msg was already saved
      }
    } catch (e) {
      console.error('Failed to send message:', e);
      isStreaming = false;
    }
  }
</script>

<div class="flex h-full w-full">
  {#if getSidebarOpen()}
    <aside class="w-64 flex-shrink-0 border-r flex flex-col" style="border-color: var(--border);">
      <div class="flex-1 overflow-hidden">
        <ConversationList bind:activeId={activeConversationId} onSelect={(id) => { currentView = 'chat'; handleSelect(id); }} />
      </div>
      <div class="p-2 border-t" style="border-color: var(--border);">
        <button
          onclick={() => { const next = currentView === 'settings' ? 'chat' : 'settings'; if (next === 'chat') loadModels(); currentView = next; }}
          class="w-full text-left px-3 py-2 rounded-lg text-sm cursor-pointer transition-colors flex items-center gap-2"
          style="background: {currentView === 'settings' ? 'var(--accent)' : 'transparent'}; color: {currentView === 'settings' ? '#fff' : 'var(--text-secondary)'}; border: none;"
        >
          &#9881; Settings
        </button>
      </div>
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
      <span class="text-sm font-medium" style="color: var(--text-primary);">
        {currentView === 'settings' ? 'Settings' : 'Orion Chat'}
      </span>

      {#if currentView === 'chat'}
        <div class="ml-auto">
          {#if allModels.length > 0}
            <ModelSelector models={allModels} bind:selected={currentModelId} />
          {:else}
            <span class="text-xs" style="color: var(--text-secondary);">No models — add a provider in Settings</span>
          {/if}
        </div>
      {/if}
    </header>

    {#if currentView === 'settings'}
      <div class="flex-1 overflow-y-auto">
        <ProviderSettings />
      </div>
    {:else if activeConversationId}
      <MessageList {messages} />
      <InputArea disabled={isStreaming} onSend={handleSend} />
    {:else}
      <div class="flex-1 flex items-center justify-center" style="color: var(--text-secondary);">
        <p class="text-sm">Select or create a conversation to start chatting</p>
      </div>
    {/if}
  </main>
</div>
