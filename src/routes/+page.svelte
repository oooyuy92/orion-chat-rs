<script lang="ts">
  import { onMount } from 'svelte';
  import { SidebarProvider, SidebarInset } from '$lib/components/ui/sidebar';
  import AppSidebar from '$lib/components/sidebar/AppSidebar.svelte';
  import ChatArea from '$lib/components/chat/ChatArea.svelte';
  import { api } from '$lib/utils/invoke';
  import type { ChatEvent } from '$lib/utils/invoke';
  import type { Message, ModelInfo, ModelGroup } from '$lib/types';

  let activeConversationId = $state('');
  let messages = $state<Message[]>([]);
  let isStreaming = $state(false);
  let streamingMessageId = $state('');
  let currentModelId = $state('');
  let modelGroups = $state<ModelGroup[]>([]);

  async function loadModels() {
    try {
      const providers = await api.listProviders();
      const modelArrays = await Promise.all(
        providers.map((p) => api.fetchModels(p.id).catch(() => [] as ModelInfo[])),
      );

      // Convert to ModelGroup format
      modelGroups = providers.map((provider) => ({
        providerId: provider.id,
        providerName: provider.name,
        models: modelArrays.flat().filter((m) => m.providerId === provider.id),
      })).filter((group) => group.models.length > 0);

      // Set default model if none selected
      const allModels = modelGroups.flatMap((g) => g.models);
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

  function handleConversationSelect(id: string) {
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

  function handleChatEvent(event: { type: 'send'; content: string }) {
    if (event.type === 'send') {
      handleSend(event.content);
    }
  }
</script>

<SidebarProvider>
  <AppSidebar
    {modelGroups}
    bind:selectedModelId={currentModelId}
    bind:activeConversationId
    onConversationSelect={handleConversationSelect}
  />

  <SidebarInset>
    {#if activeConversationId}
      <ChatArea
        conversationId={activeConversationId}
        bind:modelId={currentModelId}
        {messages}
        {modelGroups}
        disabled={isStreaming}
        onEvent={handleChatEvent}
      />
    {:else}
      <div class="flex h-full items-center justify-center text-muted-foreground">
        <p class="text-sm">Select or create a conversation to start chatting</p>
      </div>
    {/if}
  </SidebarInset>
</SidebarProvider>
