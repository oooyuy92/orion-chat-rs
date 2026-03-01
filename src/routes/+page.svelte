<script lang="ts">
  import { onMount } from 'svelte';
  import ConversationList from '$lib/components/sidebar/ConversationList.svelte';
  import MessageList from '$lib/components/chat/MessageList.svelte';
  import InputArea from '$lib/components/chat/InputArea.svelte';
  import ProviderSettings from '$lib/components/settings/ProviderSettings.svelte';
  import { getSidebarOpen, toggleSidebar } from '$lib/stores/ui.svelte';
  import { api } from '$lib/utils/invoke';
  import type { ChatEvent } from '$lib/utils/invoke';
  import type { Message, ModelInfo } from '$lib/types';

  type View = 'chat' | 'settings';

  const suggestionPrompts = [
    'What are the latest trends in AI?',
    'How does machine learning work?',
    'Explain quantum computing',
    'Best practices for React development',
  ];

  let currentView = $state<View>('chat');
  let activeConversationId = $state('');
  let messages = $state<Message[]>([]);
  let isStreaming = $state(false);
  let streamingMessageId = $state('');
  let currentModelId = $state('');
  let allModels = $state<ModelInfo[]>([]);

  let currentModelName = $derived(
    allModels.find((model) => model.id === currentModelId)?.name ?? '',
  );
  let isComposerDisabled = $derived(
    isStreaming || !activeConversationId || !currentModelId,
  );

  async function loadModels() {
    try {
      const providers = await api.listProviders();
      const modelArrays = await Promise.all(
        providers.map((provider) => api.fetchModels(provider.id).catch(() => [] as ModelInfo[])),
      );
      allModels = modelArrays.flat();

      if (allModels.length > 0 && !currentModelId) {
        currentModelId = allModels[0].id;
      }
    } catch (error) {
      console.error('Failed to load models:', error);
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
    } catch (error) {
      console.error('Failed to load messages:', error);
      messages = [];
    }
  }

  function handleEvent(event: ChatEvent) {
    switch (event.type) {
      case 'started':
        streamingMessageId = event.messageId;
        break;
      case 'delta': {
        const idx = messages.findIndex((message) => message.id === streamingMessageId);
        if (idx !== -1) {
          messages[idx] = {
            ...messages[idx],
            content: messages[idx].content + event.content,
          };
        }
        break;
      }
      case 'reasoning': {
        const idx = messages.findIndex((message) => message.id === streamingMessageId);
        if (idx !== -1) {
          messages[idx] = {
            ...messages[idx],
            reasoning: (messages[idx].reasoning ?? '') + event.content,
          };
        }
        break;
      }
      case 'usage': {
        const idx = messages.findIndex((message) => message.id === streamingMessageId);
        if (idx !== -1) {
          messages[idx] = {
            ...messages[idx],
            tokenCount: event.promptTokens + event.completionTokens,
          };
        }
        break;
      }
      case 'finished': {
        const idx = messages.findIndex((message) => message.id === streamingMessageId);
        if (idx !== -1) {
          messages[idx] = { ...messages[idx], status: 'done' };
        }
        isStreaming = false;
        streamingMessageId = '';
        break;
      }
      case 'error': {
        console.error('Stream error:', event.message);
        const idx = messages.findIndex((message) => message.id === streamingMessageId);
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
    if (!activeConversationId || !currentModelId || isStreaming) {
      return;
    }

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
      await api.sendMessage(activeConversationId, content, currentModelId, (event) => {
        if (event.type === 'started') {
          const idx = messages.findIndex((message) => message.id === assistantMsg.id);
          if (idx !== -1) {
            messages[idx] = { ...messages[idx], id: event.messageId };
          }
          streamingMessageId = event.messageId;
          return;
        }

        handleEvent(event);
      });
    } catch (error) {
      console.error('Failed to send message:', error);
      const idx = messages.findIndex((message) => message.id === streamingMessageId);
      if (idx !== -1) {
        messages[idx] = {
          ...messages[idx],
          status: 'error',
          content: messages[idx].content || 'Failed to send message. Please try again.',
        };
      }
      isStreaming = false;
      streamingMessageId = '';
    }
  }
</script>

<div class="app-shell">
  {#if getSidebarOpen()}
    <aside class="sidebar-shell">
      <div class="sidebar-content">
        <ConversationList
          bind:activeId={activeConversationId}
          onSelect={(id) => {
            currentView = 'chat';
            handleSelect(id);
          }}
        />
      </div>

      <div class="sidebar-footer">
        <button
          class="sidebar-toggle"
          onclick={() => {
            const nextView = currentView === 'settings' ? 'chat' : 'settings';
            if (nextView === 'chat') {
              loadModels();
            }
            currentView = nextView;
          }}
        >
          <span aria-hidden="true">&#9881;</span>
          <span>{currentView === 'settings' ? 'Back to Chat' : 'Settings'}</span>
        </button>
      </div>
    </aside>
  {/if}

  <main class="chat-shell">
    <header class="chat-topbar">
      <button onclick={toggleSidebar} class="menu-button" aria-label="Toggle sidebar">
        &#9776;
      </button>
      <div class="topbar-meta">
        <h1>{currentView === 'settings' ? 'Settings' : 'Orion Chat'}</h1>
        {#if currentView === 'chat'}
          <p>
            {#if allModels.length > 0 && currentModelName}
              Using {currentModelName}
            {:else}
              Add a provider in Settings to load models
            {/if}
          </p>
        {/if}
      </div>
    </header>

    {#if currentView === 'settings'}
      <div class="settings-panel">
        <ProviderSettings />
      </div>
    {:else}
      {#if activeConversationId}
        <MessageList {messages} />
      {:else}
        <section class="empty-chat">
          <div class="empty-card">
            <h2>Start a new conversation</h2>
            <p>Create or select a chat from the left sidebar to begin.</p>
          </div>
        </section>
      {/if}

      <InputArea
        disabled={isComposerDisabled}
        onSend={handleSend}
        suggestions={suggestionPrompts}
        models={allModels}
        bind:selectedModel={currentModelId}
      />
    {/if}
  </main>
</div>

<style>
  .app-shell {
    display: flex;
    height: 100%;
    width: 100%;
    background: var(--background);
  }

  .sidebar-shell {
    width: 18rem;
    border-right: 1px solid var(--border);
    background: var(--sidebar-bg);
    display: flex;
    flex-direction: column;
  }

  .sidebar-content {
    min-height: 0;
    flex: 1;
  }

  .sidebar-footer {
    border-top: 1px solid var(--border);
    padding: 0.5rem;
  }

  .sidebar-toggle {
    width: 100%;
    border: 1px solid transparent;
    background: transparent;
    color: var(--muted-foreground);
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-radius: var(--radius);
    padding: 0.625rem 0.75rem;
    cursor: pointer;
    font-size: 0.875rem;
  }

  .sidebar-toggle:hover {
    background: var(--sidebar-hover);
    color: var(--foreground);
  }

  .chat-shell {
    min-width: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--background);
  }

  .chat-topbar {
    height: 3.5rem;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0 1rem;
    background: var(--card);
    flex-shrink: 0;
  }

  .menu-button {
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    background: var(--background);
    color: var(--foreground);
    width: 2rem;
    height: 2rem;
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
  }

  .menu-button:hover {
    background: var(--muted);
  }

  .topbar-meta {
    min-width: 0;
  }

  .topbar-meta h1 {
    margin: 0;
    font-size: 0.95rem;
    line-height: 1.2;
    font-weight: 650;
  }

  .topbar-meta p {
    margin: 0.125rem 0 0;
    color: var(--muted-foreground);
    font-size: 0.75rem;
    line-height: 1.1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 28rem;
  }

  .settings-panel {
    min-height: 0;
    overflow-y: auto;
    flex: 1;
  }

  .empty-chat {
    min-height: 0;
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1.5rem;
  }

  .empty-card {
    border: 1px solid var(--border);
    background: var(--card);
    border-radius: 0.9rem;
    width: min(40rem, 100%);
    padding: 1.25rem 1.5rem;
  }

  .empty-card h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 650;
    color: var(--foreground);
  }

  .empty-card p {
    margin: 0.45rem 0 0;
    font-size: 0.875rem;
    color: var(--muted-foreground);
  }

  @media (max-width: 900px) {
    .sidebar-shell {
      width: 15rem;
    }

    .topbar-meta p {
      max-width: 16rem;
    }
  }

  @media (max-width: 640px) {
    .chat-topbar {
      padding: 0 0.75rem;
    }

    .topbar-meta p {
      display: none;
    }
  }
</style>
