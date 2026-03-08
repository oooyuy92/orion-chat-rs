<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { load as loadStore } from '@tauri-apps/plugin-store';
  import { SidebarProvider, SidebarInset } from '$lib/components/ui/sidebar';
  import AppSidebar from '$lib/components/sidebar/AppSidebar.svelte';
  import ChatArea from '$lib/components/chat/ChatArea.svelte';
  import { api } from '$lib/utils/invoke';
  import { titleUpdates } from '$lib/stores/conversations';
  import { getModelParams } from '$lib/stores/modelParams';
  import type { ChatEvent } from '$lib/utils/invoke';
  import type { Assistant, Conversation, Message, ModelGroup, ProviderType } from '$lib/types';
  import { i18n } from '$lib/stores/i18n.svelte';

  type ConversationSelection = {
    conversationId: string;
    messageId?: string | null;
  };

  let activeConversationId = $state('');
  let messages = $state<Message[]>([]);
  const pageSize = 100;
  let hasMoreMessages = $state(false);
  let isLoadingMoreMessages = $state(false);
  let isStreaming = $state(false);
  let streamingMessageId = $state('');
  let currentModelId = $state('');
  let modelGroups = $state<ModelGroup[]>([]);
  let assistants = $state<Assistant[]>([]);
  let conversations = $state<Conversation[]>([]);
  let pendingFocusMessageId = $state<string | null>(null);

  const currentConversation = $derived(
    conversations.find((conversation) => conversation.id === activeConversationId) ?? null,
  );
  const selectedAssistantId = $derived(currentConversation?.assistantId ?? null);
  const assistantSelectionLocked = $derived.by(() =>
    messages.some((message) => message.role === 'user'),
  );

  // Soft-delete restore state
  let deletedMessageIds = $state<string[]>([]);
  let canRestore = $derived(deletedMessageIds.length > 0);

  // Auto-rename settings (read from store)
  let autoRename = $state(false);
  let autoRenameModelId = $state('');

  // Auto-compress settings
  let autoCompress = $state(false);
  let autoCompressModelId = $state('');
  let autoCompressThreshold = $state(50000);
  let lastPromptTokens = $state(0);
  let isCompressing = $state(false);

  async function loadAutoRenameSettings() {
    try {
      const store = await loadStore('settings.json');
      autoRename = (await store.get<boolean>('autoRename')) ?? false;
      autoRenameModelId = (await store.get<string>('autoRenameModelId')) ?? '';
      autoCompress = (await store.get<boolean>('autoCompress')) ?? false;
      autoCompressModelId = (await store.get<string>('autoCompressModelId')) ?? '';
      autoCompressThreshold = (await store.get<number>('autoCompressThreshold')) ?? 50000;
    } catch (e) {
      console.error(e);
    }
  }

  function resolveProviderType(modelId: string): ProviderType {
    for (const group of modelGroups) {
      if (group.models.some((m) => m.id === modelId)) {
        return group.providerType;
      }
    }
    return 'openaiCompat';
  }

  async function loadParamsForModel(modelId: string) {
    const pt = resolveProviderType(modelId);
    return getModelParams(modelId, pt);
  }

  async function tryAutoRename(conversationId: string, msgCount: number) {
    if (!autoRename || !autoRenameModelId) return;
    // Rename after 1st round (2 msgs), then every 10 rounds (every 20 msgs)
    const shouldRename = msgCount === 2 || (msgCount > 2 && (msgCount - 2) % 20 === 0);
    if (!shouldRename) return;
    try {
      const title = await api.generateConversationTitle(conversationId, autoRenameModelId);
      if (!title) return;
      await api.updateConversationTitle(conversationId, title);
      titleUpdates.set({ id: conversationId, title });
    } catch (e) {
      console.error('Auto-rename failed:', e);
    }
  }

  async function tryAutoCompress(conversationId: string) {
    if (!conversationId || isCompressing) return;
    isCompressing = true;
    try {
      const newMessages = await api.compressConversation(
        conversationId,
        autoCompressModelId,
        handleEvent,
      );
      messages = newMessages;
      hasMoreMessages = false;
      isLoadingMoreMessages = false;
      lastPromptTokens = 0;
    } catch (e) {
      console.error('Auto-compress failed:', e);
      await loadLatestMessages(conversationId);
    } finally {
      isCompressing = false;
    }
  }

  async function loadModels() {
    try {
      const providers = await api.listProviders();

      // Use DB models (already on provider.models) — filter enabled providers and models only
      modelGroups = providers
        .filter((p) => p.enabled)
        .map((provider) => ({
          providerId: provider.id,
          providerName: provider.name,
          providerType: provider.providerType,
          models: provider.models.filter((m) => m.enabled),
        }))
        .filter((group) => group.models.length > 0);

      // Set default model if none selected
      const allModels = modelGroups.flatMap((g) => g.models);
      if (allModels.length > 0 && !currentModelId) {
        currentModelId = allModels[0].id;
      }
    } catch (e) {
      console.error('Failed to load models:', e);
    }
  }

  async function loadAssistants() {
    try {
      assistants = await api.listAssistants();
      syncCurrentModelToAssistantBinding();
    } catch (e) {
      console.error('Failed to load assistants:', e);
      assistants = [];
    }
  }

  async function loadConversations() {
    try {
      conversations = await api.listConversations();
    } catch (e) {
      console.error('Failed to load conversations:', e);
      conversations = [];
    }
  }

  function syncCurrentModelToAssistantBinding(assistantId: string | null | undefined = currentConversation?.assistantId) {
    const assistant = assistants.find((item) => item.id === assistantId);
    if (assistant?.modelId) {
      currentModelId = assistant.modelId;
    }
  }

  async function refreshConversationState(conversationId: string) {
    await loadConversations();
    const conversation = conversations.find((item) => item.id === conversationId) ?? null;
    syncCurrentModelToAssistantBinding(conversation?.assistantId);
  }

  function handleUndoKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'z' && !e.shiftKey) {
      if (canRestore) {
        e.preventDefault();
        handleRestore();
      }
    }
  }

  onMount(() => {
    loadModels();
    loadAssistants();
    loadConversations();
    loadAutoRenameSettings();
    window.addEventListener('keydown', handleUndoKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleUndoKeydown);
  });

async function loadMessagesUntilFocused(conversationId: string, messageId: string) {
  if (messages.some((message) => message.id === messageId)) {
    return;
  }

  isLoadingMoreMessages = true;
  try {
    while (
      activeConversationId === conversationId &&
      !messages.some((message) => message.id === messageId) &&
      hasMoreMessages
    ) {
      const beforeMessageId = messages[0]?.id ?? null;
      if (!beforeMessageId) break;
      const page = await api.getMessages(conversationId, {
        limit: pageSize,
        beforeMessageId,
      });
      messages = [...page.messages, ...messages];
      hasMoreMessages = page.hasMore;
      if (page.messages.length === 0) {
        break;
      }
    }
  } catch (e) {
    console.error('Failed to focus searched message:', e);
  } finally {
    isLoadingMoreMessages = false;
  }
}

function handleConversationSelect(selection: ConversationSelection) {
  invalidateRestore();
  activeConversationId = selection.conversationId;
  pendingFocusMessageId = selection.messageId ?? null;
  void refreshConversationState(selection.conversationId);
  void (async () => {
    await loadLatestMessages(selection.conversationId);
    if (selection.messageId) {
      await loadMessagesUntilFocused(selection.conversationId, selection.messageId);
    }
  })();
}

  async function handleAssistantSelect(assistantId: string | null) {
    if (!activeConversationId || assistantSelectionLocked) return;

    const currentAssistantId = currentConversation?.assistantId ?? null;
    if (currentAssistantId === assistantId) return;

    try {
      await api.updateConversationAssistant(activeConversationId, assistantId);
      conversations = conversations.map((conversation) =>
        conversation.id === activeConversationId
          ? { ...conversation, assistantId, updatedAt: new Date().toISOString() }
          : conversation,
      );
      syncCurrentModelToAssistantBinding(assistantId);
    } catch (e) {
      console.error('Failed to update conversation assistant:', e);
      await refreshConversationState(activeConversationId);
    }
  }

  async function loadLatestMessages(conversationId: string) {
    if (!conversationId) {
      messages = [];
      hasMoreMessages = false;
      isLoadingMoreMessages = false;
      return;
    }

    try {
      const page = await api.getMessages(conversationId, { limit: pageSize });
      messages = page.messages;
      hasMoreMessages = page.hasMore;
      isLoadingMoreMessages = false;
    } catch (e) {
      console.error('Failed to load messages:', e);
      messages = [];
      hasMoreMessages = false;
      isLoadingMoreMessages = false;
    }
  }

  async function loadOlderMessages() {
    if (
      !activeConversationId ||
      !messages.length ||
      !hasMoreMessages ||
      isLoadingMoreMessages ||
      isStreaming ||
      isCompressing
    ) {
      return;
    }

    isLoadingMoreMessages = true;
    try {
      const page = await api.getMessages(activeConversationId, {
        limit: pageSize,
        beforeMessageId: messages[0]?.id ?? null,
      });
      messages = [...page.messages, ...messages];
      hasMoreMessages = page.hasMore;
    } catch (e) {
      console.error('Failed to load older messages:', e);
    } finally {
      isLoadingMoreMessages = false;
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
        lastPromptTokens = event.promptTokens;
        break;
      }
      case 'finished': {
        const idx = messages.findIndex((m) => m.id === streamingMessageId);
        if (idx !== -1) {
          messages[idx] = { ...messages[idx], status: 'done' };
        }
        isStreaming = false;
        streamingMessageId = '';
        // Check if auto-compress needed
        if (autoCompress && autoCompressModelId && lastPromptTokens > autoCompressThreshold) {
          void tryAutoCompress(activeConversationId);
        }
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

  async function handleRestore() {
    if (!canRestore) return;
    const idToRestore = deletedMessageIds[deletedMessageIds.length - 1];
    try {
      await api.restoreMessage(idToRestore);
      deletedMessageIds = deletedMessageIds.slice(0, -1);
      await loadLatestMessages(activeConversationId);
    } catch (e) {
      console.error('Failed to restore message:', e);
    }
  }

  async function handleSend(content: string) {
    invalidateRestore();
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
      versionGroupId: null,
      versionNumber: 1,
      totalVersions: 1,
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
      versionGroupId: null,
      versionNumber: 1,
      totalVersions: 1,
    };
    messages = [...messages, assistantMsg];
    streamingMessageId = assistantMsg.id;
    isStreaming = true;

    try {
      const params = await loadParamsForModel(currentModelId);
      await api.sendMessage(
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
        params.common,
        params.providerParams,
      );
      // Reload messages from DB to sync IDs (optimistic user msg has frontend-generated ID)
      await loadLatestMessages(activeConversationId);
      // Trigger auto-rename based on message count
      void tryAutoRename(activeConversationId, messages.length);
    } catch (e) {
      console.error('Failed to send message:', e);
      const idx = messages.findIndex((m) => m.id === assistantMsg.id || m.id === streamingMessageId);
      if (idx !== -1) {
        messages[idx] = {
          ...messages[idx],
          status: 'error',
          content: messages[idx].content || String(e),
        };
      }
      isStreaming = false;
      streamingMessageId = '';
    }
  }

  function invalidateRestore() {
    deletedMessageIds = [];
  }

  async function handleStop() {
    if (!isStreaming) return;
    try {
      await api.stopGeneration();
    } catch (e) {
      console.error('Failed to stop generation:', e);
    }
  }

  async function handleDelete(messageId: string) {
    // Optimistic: update UI immediately
    const previous = messages;
    messages = messages.filter((m) => m.id !== messageId);
    deletedMessageIds = [...deletedMessageIds, messageId];
    try {
      await api.deleteMessage(messageId);
      // Reload to reflect version changes (e.g. another version activated)
      await loadLatestMessages(activeConversationId);
    } catch (e) {
      console.error('Failed to delete message:', e);
      // Rollback UI on failure
      messages = previous;
      deletedMessageIds = deletedMessageIds.filter((id) => id !== messageId);
    }
  }

  async function doResend(messageId: string) {
    console.log('[+page] doResend called, conversationId:', activeConversationId, 'modelId:', currentModelId, 'messageId:', messageId);
    invalidateRestore();
    if (!activeConversationId || !currentModelId) return;

    try {
      // Delete all messages after this one on backend
      console.log('[+page] Calling deleteMessagesAfter...');
      await api.deleteMessagesAfter(activeConversationId, messageId);

      // Truncate local messages
      const idx = messages.findIndex((m) => m.id === messageId);
      console.log('[+page] Found message at index:', idx);
      if (idx === -1) return;
      messages = messages.slice(0, idx + 1);

      // Create optimistic assistant placeholder
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
        versionGroupId: null,
        versionNumber: 1,
        totalVersions: 1,
      };
      messages = [...messages, assistantMsg];
      streamingMessageId = assistantMsg.id;
      isStreaming = true;

      console.log('[+page] Calling resendMessage...');
      const params = await loadParamsForModel(currentModelId);
      await api.resendMessage(
        activeConversationId,
        currentModelId,
        (event) => {
          if (event.type === 'started') {
            const i = messages.findIndex((m) => m.id === assistantMsg.id);
            if (i !== -1) {
              messages[i] = { ...messages[i], id: event.messageId };
            }
            streamingMessageId = event.messageId;
          } else {
            handleEvent(event);
          }
        },
        params.common,
        params.providerParams,
      );
      console.log('[+page] resendMessage completed');
    } catch (e) {
      console.error('Failed to resend:', e);
      isStreaming = false;
    }
  }

  async function handleResend(messageId: string) {
    console.log('[+page] handleResend called with messageId:', messageId);
    await doResend(messageId);
  }

  async function handleEditResend(messageId: string, content: string) {
    invalidateRestore();

    // Check if the next message (AI response) has multiple versions
    const userIdx = messages.findIndex((m) => m.id === messageId);
    let versionModels: [number, string][] = [];
    if (userIdx !== -1 && userIdx + 1 < messages.length) {
      const nextMsg = messages[userIdx + 1];
      if (nextMsg.role === 'assistant' && nextMsg.totalVersions > 1) {
        const groupId = nextMsg.versionGroupId || nextMsg.id;
        try {
          versionModels = await api.getVersionModels(groupId);
        } catch (e) {
          console.error('Failed to get version models:', e);
        }
      }
    }

    try {
      await api.updateMessageContent(messageId, content);
      // Update local message content
      const idx = messages.findIndex((m) => m.id === messageId);
      if (idx !== -1) {
        messages[idx] = { ...messages[idx], content };
      }
    } catch (e) {
      console.error('Failed to update message:', e);
      return;
    }

    if (versionModels.length > 1) {
      // Multi-version resend: regenerate all versions using their original models
      await doResendAllVersions(messageId, versionModels);
    } else {
      await doResend(messageId);
    }
  }

  async function doResendAllVersions(userMessageId: string, versionModels: [number, string][]) {
    if (!activeConversationId) return;

    // Sort by version number
    versionModels.sort((a, b) => a[0] - b[0]);
    const firstModel = versionModels[0][1];

    try {
      // Delete all messages after the user message
      await api.deleteMessagesAfter(activeConversationId, userMessageId);

      // Truncate local messages
      const idx = messages.findIndex((m) => m.id === userMessageId);
      if (idx === -1) return;
      messages = messages.slice(0, idx + 1);

      // Generate first version via resendMessage
      const assistantMsg: Message = {
        id: crypto.randomUUID(),
        conversationId: activeConversationId,
        role: 'assistant',
        content: '',
        reasoning: null,
        modelId: firstModel,
        status: 'streaming',
        tokenCount: null,
        createdAt: new Date().toISOString(),
        versionGroupId: null,
        versionNumber: 1,
        totalVersions: versionModels.length,
      };
      messages = [...messages, assistantMsg];
      streamingMessageId = assistantMsg.id;
      isStreaming = true;

      let firstRealId = '';

      const firstParams = await loadParamsForModel(firstModel);
      await api.resendMessage(
        activeConversationId,
        firstModel,
        (event) => {
          if (event.type === 'started') {
            const i = messages.findIndex((m) => m.id === assistantMsg.id);
            if (i !== -1) {
              messages[i] = { ...messages[i], id: event.messageId };
            }
            streamingMessageId = event.messageId;
            firstRealId = event.messageId;
          } else {
            handleEvent(event);
          }
        },
        firstParams.common,
        firstParams.providerParams,
      );

      // Reload to get the real message with proper IDs
      await loadLatestMessages(activeConversationId);

      // Generate subsequent versions using generateVersion
      for (let i = 1; i < versionModels.length; i++) {
        const vModel = versionModels[i][1];
        const currentAiMsg = messages.find((m) => m.id === firstRealId);
        if (!currentAiMsg) break;

        const placeholder: Message = {
          id: crypto.randomUUID(),
          conversationId: activeConversationId,
          role: 'assistant',
          content: '',
          reasoning: null,
          modelId: vModel,
          status: 'streaming',
          tokenCount: null,
          createdAt: currentAiMsg.createdAt,
          versionGroupId: currentAiMsg.versionGroupId || currentAiMsg.id,
          versionNumber: i + 1,
          totalVersions: versionModels.length,
        };

        const aiIdx = messages.findIndex((m) => m.id === currentAiMsg.id);
        messages = [...messages.slice(0, aiIdx), placeholder, ...messages.slice(aiIdx + 1)];
        streamingMessageId = placeholder.id;
        isStreaming = true;

        const vParams = await loadParamsForModel(vModel);
        await api.generateVersion(
          activeConversationId,
          firstRealId,
          vModel,
          (event) => {
            if (event.type === 'started') {
              const j = messages.findIndex((m) => m.id === placeholder.id);
              if (j !== -1) {
                messages[j] = { ...messages[j], id: event.messageId };
              }
              streamingMessageId = event.messageId;
            } else {
              handleEvent(event);
            }
          },
          vParams.common,
          vParams.providerParams,
        );

        await loadLatestMessages(activeConversationId);
      }
    } catch (e) {
      console.error('Failed to resend all versions:', e);
      isStreaming = false;
      await loadLatestMessages(activeConversationId);
    }
  }

  async function handleRegenerate(messageId: string, modelId: string | null) {
    console.log('[+page] handleRegenerate called with messageId:', messageId, 'modelId:', modelId);
    invalidateRestore();
    if (!activeConversationId) return;

    const effectiveModelId = modelId || currentModelId;
    if (!effectiveModelId) return;

    try {
      // Find the message to get its position
      const idx = messages.findIndex((m) => m.id === messageId);
      if (idx === -1) return;

      // Replace message with streaming placeholder at same position, remove everything after
      const placeholder: Message = {
        ...messages[idx],
        content: '',
        reasoning: null,
        status: 'streaming',
        tokenCount: null,
        modelId: effectiveModelId,
      };
      messages = [...messages.slice(0, idx), placeholder];
      streamingMessageId = messageId;
      isStreaming = true;

      const params = await loadParamsForModel(effectiveModelId);
      await api.regenerateMessage(
        activeConversationId,
        messageId,
        effectiveModelId,
        (event) => {
          if (event.type === 'started') {
            // The backend reuses the same message ID, so no ID swap needed
            streamingMessageId = event.messageId;
          } else {
            handleEvent(event);
          }
        },
        params.common,
        params.providerParams,
      );

      await loadLatestMessages(activeConversationId);
    } catch (e) {
      console.error('Failed to regenerate:', e);
      isStreaming = false;
      await loadLatestMessages(activeConversationId);
    }
  }

  async function handleGenerateVersion(messageId: string) {
    console.log('[+page] handleGenerateVersion called with messageId:', messageId);
    invalidateRestore();
    if (!activeConversationId || !currentModelId) return;

    try {
      // Find the current message to get its position
      const idx = messages.findIndex((m) => m.id === messageId);
      if (idx === -1) return;

      const currentMsg = messages[idx];

      // Create streaming placeholder for the new version at same position
      const placeholder: Message = {
        id: crypto.randomUUID(),
        conversationId: activeConversationId,
        role: 'assistant',
        content: '',
        reasoning: null,
        modelId: currentModelId,
        status: 'streaming',
        tokenCount: null,
        createdAt: currentMsg.createdAt,
        versionGroupId: currentMsg.versionGroupId || currentMsg.id,
        versionNumber: currentMsg.totalVersions + 1,
        totalVersions: currentMsg.totalVersions + 1,
      };

      // Replace current message with placeholder, remove everything after
      messages = [...messages.slice(0, idx), placeholder, ...messages.slice(idx + 1)];
      streamingMessageId = placeholder.id;
      isStreaming = true;

      const params = await loadParamsForModel(currentModelId);
      await api.generateVersion(
        activeConversationId,
        messageId,
        currentModelId,
        (event) => {
          if (event.type === 'started') {
            const i = messages.findIndex((m) => m.id === placeholder.id);
            if (i !== -1) {
              messages[i] = { ...messages[i], id: event.messageId };
            }
            streamingMessageId = event.messageId;
          } else {
            handleEvent(event);
          }
        },
        params.common,
        params.providerParams,
      );

      await loadLatestMessages(activeConversationId);
    } catch (e) {
      console.error('Failed to generate version:', e);
      isStreaming = false;
      await loadLatestMessages(activeConversationId);
    }
  }

  async function handleSwitchVersion(versionGroupId: string, versionNumber: number) {
    invalidateRestore();
    try {
      await api.switchVersion(versionGroupId, versionNumber);
      await loadLatestMessages(activeConversationId);
    } catch (e) {
      console.error('Failed to switch version:', e);
    }
  }

  function handleChatEvent(event: { type: string; [key: string]: any }) {
    console.log('[+page] handleChatEvent:', event);
    switch (event.type) {
      case 'send':
        handleSend(event.content);
        break;
      case 'delete':
        handleDelete(event.messageId);
        break;
      case 'resend':
        handleResend(event.messageId);
        break;
      case 'editResend':
        handleEditResend(event.messageId, event.content);
        break;
      case 'regenerate':
        handleRegenerate(event.messageId, event.modelId);
        break;
      case 'generateVersion':
        handleGenerateVersion(event.messageId);
        break;
      case 'switchVersion':
        handleSwitchVersion(event.versionGroupId, event.versionNumber);
        break;
      case 'stop':
        handleStop();
        break;
    }
  }
</script>

<SidebarProvider>
  <AppSidebar
    bind:activeConversationId
    onConversationSelect={handleConversationSelect}
  />

  <SidebarInset>
    {#if activeConversationId}
      <ChatArea
        conversationId={activeConversationId}
        {messages}
        {modelGroups}
        {assistants}
        {selectedAssistantId}
        {assistantSelectionLocked}
        {hasMoreMessages}
        {isLoadingMoreMessages}
        canLoadOlderMessages={!isStreaming && !isCompressing}
        focusedMessageId={pendingFocusMessageId}
        bind:selectedModelId={currentModelId}
        disabled={isStreaming || isCompressing}
        onAssistantSelect={handleAssistantSelect}
        onLoadOlderMessages={loadOlderMessages}
        onEvent={handleChatEvent}
      />
    {:else}
      <div class="flex h-full items-center justify-center text-muted-foreground">
        <p class="text-sm">{i18n.t.selectConversationPrompt}</p>
      </div>
    {/if}
  </SidebarInset>
</SidebarProvider>
