<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { load as loadStore } from '@tauri-apps/plugin-store';
  import { SidebarProvider, SidebarInset, SidebarTrigger } from '$lib/components/ui/sidebar';
  import AppSidebar from '$lib/components/sidebar/AppSidebar.svelte';
  import ChatArea from '$lib/components/chat/ChatArea.svelte';
  import ToolAuthDialog from '$lib/components/chat/ToolAuthDialog.svelte';
  import { agentChat, agentStop } from '$lib/api/agent';
  import { api } from '$lib/utils/invoke';
  import {
    addToolCall,
    agentMode,
    clearToolCalls,
    clearAgentEventLog,
    completeToolCall,
    pendingAuth,
    pushAgentEvent,
    updateToolCall,
  } from '$lib/stores/agent';
  import { titleUpdates, assistantUpdates, conversationCreated, streamingConversations } from '$lib/stores/conversations';
  import { getModelParams } from '$lib/stores/modelParams';
  import type { Assistant, ChatEvent, Conversation, Message, ModelGroup, ProviderType } from '$lib/types';
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
  let streamingConvIds = $state<Set<string>>(new Set());
  const isStreaming = $derived(streamingConvIds.has(activeConversationId));
  let streamingMessageId = $state('');
  let currentModelId = $state('');
  let modelGroups = $state<ModelGroup[]>([]);
  let assistants = $state<Assistant[]>([]);
  let conversations = $state<Conversation[]>([]);
  let pendingFocusMessageId = $state<string | null>(null);

  function markStreaming(convId: string) {
    streamingConvIds = new Set([...streamingConvIds, convId]);
    streamingConversations.set(streamingConvIds);
  }
  function unmarkStreaming(convId: string) {
    const next = new Set(streamingConvIds);
    next.delete(convId);
    streamingConvIds = next;
    streamingConversations.set(streamingConvIds);
  }

  function markAgentStreaming(convId: string) {
    agentStreamingConvIds = new Set([...agentStreamingConvIds, convId]);
  }

  function unmarkAgentStreaming(convId: string) {
    const next = new Set(agentStreamingConvIds);
    next.delete(convId);
    agentStreamingConvIds = next;
  }

  type ConvCache = {
    messages: Message[];
    streamingMessageId: string;
    groupStreamingMessages: Message[];
    lastPromptTokens: number;
  };
  let messageCache = new Map<string, ConvCache>();

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
  let groupStreamingMessages = $state<Message[]>([]);

  // Auto-rename settings (read from store)
  let autoRename = $state(false);
  let autoRenameModelId = $state('');

  // Auto-compress settings
  let autoCompress = $state(false);
  let autoCompressModelId = $state('');
  let autoCompressThreshold = $state(50000);
  let lastPromptTokens = $state(0);
  let isCompressing = $state(false);
  let forkedBannerVisible = $state(false);
  let agentStreamingConvIds = $state<Set<string>>(new Set());

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
        (event) => handleEventFor(conversationId, event),
      );
      if (conversationId === activeConversationId) {
        messages = newMessages;
        hasMoreMessages = false;
        isLoadingMoreMessages = false;
        lastPromptTokens = 0;
      }
    } catch (e) {
      console.error('Auto-compress failed:', e);
      if (conversationId === activeConversationId) {
        await loadLatestMessages(conversationId);
      }
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

      if (activeConversationId) {
        syncCurrentModelToConversation();
      } else if (!currentModelId) {
        currentModelId = getFirstAvailableModelId();
      }
    } catch (e) {
      console.error('Failed to load models:', e);
    }
  }

  async function loadAssistants() {
    try {
      assistants = await api.listAssistants();
      syncCurrentModelToConversation();
    } catch (e) {
      console.error('Failed to load assistants:', e);
      assistants = [];
    }
  }

  async function loadConversations() {
    try {
      conversations = await api.listConversations();
      if (activeConversationId) {
        const conversation = conversations.find((item) => item.id === activeConversationId) ?? null;
        syncCurrentModelToConversation(conversation);
      }
    } catch (e) {
      console.error('Failed to load conversations:', e);
      conversations = [];
    }
  }

  function isAvailableModelId(modelId: string | null | undefined): modelId is string {
    return !!modelId && modelGroups.some((group) => group.models.some((model) => model.id === modelId));
  }

  function getFirstAvailableModelId(): string {
    return modelGroups.flatMap((group) => group.models)[0]?.id ?? '';
  }

  function resolveAssistantDefaultModelId(assistantId: string | null | undefined): string | null {
    const assistant = assistants.find((item) => item.id === assistantId);
    return isAvailableModelId(assistant?.modelId) ? assistant.modelId : null;
  }

  function resolveConversationModelId(conversation: Conversation | null | undefined = currentConversation): string {
    if (isAvailableModelId(conversation?.modelId)) {
      return conversation.modelId;
    }
    const assistantModelId = resolveAssistantDefaultModelId(conversation?.assistantId);
    if (assistantModelId) {
      return assistantModelId;
    }
    return getFirstAvailableModelId();
  }

  function syncCurrentModelToConversation(conversation: Conversation | null | undefined = currentConversation) {
    const nextModelId = resolveConversationModelId(conversation);
    if (nextModelId) {
      currentModelId = nextModelId;
    }
  }

  function updateLocalConversationModel(conversationId: string, modelId: string | null) {
    conversations = conversations.map((conversation) =>
      conversation.id === conversationId
        ? { ...conversation, modelId }
        : conversation,
    );
  }

  async function persistConversationModel(conversationId: string, modelId: string | null) {
    const previousModelId =
      conversations.find((conversation) => conversation.id === conversationId)?.modelId ?? null;
    updateLocalConversationModel(conversationId, modelId);
    try {
      await api.updateConversationModel(conversationId, modelId);
    } catch (e) {
      updateLocalConversationModel(conversationId, previousModelId);
      throw e;
    }
  }

  async function refreshConversationState(conversationId: string) {
    await loadConversations();
    const conversation = conversations.find((item) => item.id === conversationId) ?? null;
    syncCurrentModelToConversation(conversation);
  }

  async function handleModelSelect(modelId: string) {
    const previousModelId = resolveConversationModelId(currentConversation);
    currentModelId = modelId;
    if (!activeConversationId) return;
    try {
      await persistConversationModel(activeConversationId, modelId);
    } catch (e) {
      console.error('Failed to update conversation model:', e);
      currentModelId = previousModelId;
      await refreshConversationState(activeConversationId);
    }
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
  // Always save current conversation state if it's streaming (or has a background cache)
  if (streamingConvIds.has(activeConversationId)) {
    messageCache.set(activeConversationId, {
      messages: [...messages],
      streamingMessageId,
      groupStreamingMessages: [...groupStreamingMessages],
      lastPromptTokens,
    });
  }

  invalidateRestore();
  clearToolCalls();
  pendingAuth.set(null);
  activeConversationId = selection.conversationId;
  syncCurrentModelToConversation(
    conversations.find((conversation) => conversation.id === selection.conversationId) ?? null,
  );

  // Restore target conversation from cache (streaming or finished-in-background)
  const cached = messageCache.get(selection.conversationId);
  if (cached) {
    messages = cached.messages;
    streamingMessageId = cached.streamingMessageId;
    groupStreamingMessages = cached.groupStreamingMessages;
    lastPromptTokens = cached.lastPromptTokens;
    // If no longer streaming, clean up cache and reload from DB to sync
    if (!streamingConvIds.has(selection.conversationId)) {
      messageCache.delete(selection.conversationId);
      // Async reload to get DB-synced state (cache gives instant display)
      void (async () => {
        await loadLatestMessages(selection.conversationId);
      })();
    }
    pendingFocusMessageId = selection.messageId ?? null;
    void refreshConversationState(selection.conversationId);
  } else {
    // No cache — clear messages immediately to avoid showing stale data
    messages = [];
    groupStreamingMessages = [];
    streamingMessageId = '';
    pendingFocusMessageId = selection.messageId ?? null;
    void refreshConversationState(selection.conversationId);
    void (async () => {
      await loadLatestMessages(selection.conversationId);
      if (selection.messageId) {
        await loadMessagesUntilFocused(selection.conversationId, selection.messageId);
      }
    })();
  }
}

  async function handleAssistantSelect(assistantId: string | null) {
    if (!activeConversationId || assistantSelectionLocked) return;

    const currentAssistantId = currentConversation?.assistantId ?? null;
    if (currentAssistantId === assistantId) return;
    const assistantModelId = resolveAssistantDefaultModelId(assistantId);

    try {
      await api.updateConversationAssistant(activeConversationId, assistantId);
      conversations = conversations.map((conversation) =>
        conversation.id === activeConversationId
          ? { ...conversation, assistantId, updatedAt: new Date().toISOString() }
          : conversation,
      );
      assistantUpdates.set({ id: activeConversationId, assistantId });
      if (assistantModelId) {
        currentModelId = assistantModelId;
        await persistConversationModel(activeConversationId, assistantModelId);
      } else {
        syncCurrentModelToConversation(
          conversations.find((conversation) => conversation.id === activeConversationId) ?? null,
        );
      }
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
      // Guard: only update if still viewing this conversation
      if (conversationId !== activeConversationId) return;
      messages = page.messages;
      hasMoreMessages = page.hasMore;
      isLoadingMoreMessages = false;
    } catch (e) {
      console.error('Failed to load messages:', e);
      if (conversationId !== activeConversationId) return;
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

  function getMessages(convId: string): Message[] {
    return convId === activeConversationId
      ? messages
      : (messageCache.get(convId)?.messages ?? []);
  }

  function setMessages(convId: string, msgs: Message[]) {
    if (convId === activeConversationId) {
      messages = msgs;
    } else {
      const cached = messageCache.get(convId);
      if (cached) cached.messages = msgs;
    }
  }

  function handleEventFor(convId: string, event: ChatEvent) {
    pushAgentEvent(event);
    const msgs = getMessages(convId);
    switch (event.type) {
      case 'started':
        if (convId === activeConversationId) streamingMessageId = event.messageId;
        break;
      case 'toolCallStart':
        addToolCall({
          toolCallId: event.toolCallId,
          toolName: event.toolName,
          args: event.args,
          status: 'running',
          messageId: event.messageId,
          startTime: Date.now(),
        });
        break;
      case 'toolCallUpdate':
        updateToolCall(event.toolCallId, { result: event.partialResult });
        break;
      case 'toolCallEnd':
        completeToolCall(event.toolCallId, event.result, event.isError);
        break;
      case 'toolAuthRequest':
        if (convId === activeConversationId) {
          pendingAuth.set({
            toolCallId: event.toolCallId,
            toolName: event.toolName,
            args: event.args,
          });
        }
        break;
      case 'delta': {
        const idx = msgs.findIndex((m) => m.id === event.messageId);
        if (idx !== -1) {
          const updated = [...msgs];
          updated[idx] = { ...updated[idx], content: updated[idx].content + event.content };
          setMessages(convId, updated);
        }
        break;
      }
      case 'reasoning': {
        const idx = msgs.findIndex((m) => m.id === event.messageId);
        if (idx !== -1) {
          const updated = [...msgs];
          updated[idx] = { ...updated[idx], reasoning: (updated[idx].reasoning ?? '') + event.content };
          setMessages(convId, updated);
        }
        break;
      }
      case 'usage': {
        const idx = msgs.findIndex((m) => m.id === event.messageId);
        if (idx !== -1) {
          const updated = [...msgs];
          updated[idx] = { ...updated[idx], tokenCount: event.promptTokens + event.completionTokens };
          setMessages(convId, updated);
        }
        if (convId === activeConversationId) lastPromptTokens = event.promptTokens;
        else {
          const cached = messageCache.get(convId);
          if (cached) cached.lastPromptTokens = event.promptTokens;
        }
        break;
      }
      case 'finished': {
        const idx = msgs.findIndex((m) => m.id === event.messageId);
        if (idx !== -1) {
          const updated = [...msgs];
          updated[idx] = { ...updated[idx], status: 'done' };
          setMessages(convId, updated);
        }
        unmarkStreaming(convId);
        unmarkAgentStreaming(convId);
        // Only delete cache if this is the active conversation;
        // keep it for background conversations so we can restore on switch-back
        if (convId === activeConversationId) {
          messageCache.delete(convId);
          streamingMessageId = '';
        }
        // Check if auto-compress needed
        const tokens = convId === activeConversationId
          ? lastPromptTokens
          : (messageCache.get(convId)?.lastPromptTokens ?? 0);
        if (autoCompress && autoCompressModelId && tokens > autoCompressThreshold) {
          void tryAutoCompress(convId);
        }
        break;
      }
      case 'error': {
        console.error('Stream error:', event.message);
        const idx = msgs.findIndex((m) => m.id === event.messageId);
        if (idx !== -1) {
          const updated = [...msgs];
          updated[idx] = {
            ...updated[idx],
            status: 'error',
            content: updated[idx].content || event.message,
          };
          setMessages(convId, updated);
        }
        unmarkStreaming(convId);
        unmarkAgentStreaming(convId);
        if (convId === activeConversationId) {
          messageCache.delete(convId);
          streamingMessageId = '';
        }
        break;
      }
    }
  }

  function handleEvent(event: ChatEvent) {
    handleEventFor(activeConversationId, event);
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

    const convId = activeConversationId;
    const useAgentMode = get(agentMode);

    if (useAgentMode) {
      clearToolCalls();
      clearAgentEventLog();
      pendingAuth.set(null);
      markAgentStreaming(convId);
    }

    // Optimistic user message
    const userMsg: Message = {
      id: crypto.randomUUID(),
      conversationId: convId,
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
      messageType: 'text',
      toolError: false,
    };
    messages = [...messages, userMsg];

    // Placeholder assistant message
    const assistantMsg: Message = {
      id: crypto.randomUUID(),
      conversationId: convId,
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
      messageType: 'text',
      toolError: false,
    };
    messages = [...messages, assistantMsg];
    streamingMessageId = assistantMsg.id;
    markStreaming(convId);

    try {
      const onEvent = (event: ChatEvent) => {
        console.log('[agent] event:', event.type, event);
        if (event.type === 'started') {
          const msgs = getMessages(convId);
          const idx = msgs.findIndex((m) => m.id === assistantMsg.id);
          if (idx !== -1) {
            const updated = [...msgs];
            updated[idx] = { ...updated[idx], id: event.messageId };
            setMessages(convId, updated);
          }
          if (convId === activeConversationId) streamingMessageId = event.messageId;
        } else {
          handleEventFor(convId, event);
        }
      };

      if (useAgentMode) {
        const result = await agentChat(convId, content, currentModelId, onEvent);
        console.log('[agent] agentChat returned:', result);
        console.log('[agent] BEFORE cleanup: isStreaming=', isStreaming, 'isCompressing=', isCompressing, 'streamingConvIds=', [...streamingConvIds], 'agentStreamingConvIds=', [...agentStreamingConvIds]);
        // Agent command returned — always clear streaming state.
        // The Finished event may not have been emitted (e.g. cancelled after a prior tool turn).
        unmarkStreaming(convId);
        unmarkAgentStreaming(convId);
        if (convId === activeConversationId) streamingMessageId = '';
        console.log('[agent] AFTER cleanup: isStreaming=', isStreaming, 'isCompressing=', isCompressing);
      } else {
        const params = await loadParamsForModel(currentModelId);
        await api.sendMessage(
          convId,
          content,
          currentModelId,
          onEvent,
          params.common,
          params.providerParams,
        );
      }
      // Reload messages from DB to sync IDs (only if still active)
      if (convId === activeConversationId) {
        console.log('[agent] loading latest messages...');
        await loadLatestMessages(convId);
        console.log('[agent] loadLatestMessages done, isStreaming=', isStreaming, 'isCompressing=', isCompressing);
      }
      // Trigger auto-rename based on message count
      const msgCount = getMessages(convId).length;
      void tryAutoRename(convId, msgCount);
    } catch (e) {
      console.error('Failed to send message:', e);
      const msgs = getMessages(convId);
      const idx = msgs.findIndex((m) => m.id === assistantMsg.id || m.id === streamingMessageId);
      if (idx !== -1) {
        const updated = [...msgs];
        updated[idx] = {
          ...updated[idx],
          status: 'error',
          content: updated[idx].content || String(e),
        };
        setMessages(convId, updated);
      }
      unmarkStreaming(convId);
      unmarkAgentStreaming(convId);
      if (convId === activeConversationId) streamingMessageId = '';
    }
  }

  function invalidateRestore() {
    deletedMessageIds = [];
  }

  async function handleStop() {
    if (!isStreaming) return;
    try {
      if (agentStreamingConvIds.has(activeConversationId)) {
        pendingAuth.set(null);
        await agentStop(activeConversationId);
      } else {
        await api.stopGeneration(activeConversationId);
      }
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

    const convId = activeConversationId;

    try {
      // Delete all messages after this one on backend
      console.log('[+page] Calling deleteMessagesAfter...');
      await api.deleteMessagesAfter(convId, messageId);

      // Truncate local messages
      const idx = messages.findIndex((m) => m.id === messageId);
      console.log('[+page] Found message at index:', idx);
      if (idx === -1) return;
      messages = messages.slice(0, idx + 1);

      // Create optimistic assistant placeholder
      const assistantMsg: Message = {
        id: crypto.randomUUID(),
        conversationId: convId,
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
        messageType: 'text',
        toolError: false,
      };
      messages = [...messages, assistantMsg];
      streamingMessageId = assistantMsg.id;
      markStreaming(convId);

      console.log('[+page] Calling resendMessage...');
      const params = await loadParamsForModel(currentModelId);
      await api.resendMessage(
        convId,
        currentModelId,
        (event) => {
          if (event.type === 'started') {
            const msgs = getMessages(convId);
            const i = msgs.findIndex((m) => m.id === assistantMsg.id);
            if (i !== -1) {
              const updated = [...msgs];
              updated[i] = { ...updated[i], id: event.messageId };
              setMessages(convId, updated);
            }
            if (convId === activeConversationId) streamingMessageId = event.messageId;
          } else {
            handleEventFor(convId, event);
          }
        },
        params.common,
        params.providerParams,
      );
      console.log('[+page] resendMessage completed');
    } catch (e) {
      console.error('Failed to resend:', e);
      unmarkStreaming(convId);
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

    const convId = activeConversationId;

    // Sort by version number
    versionModels.sort((a, b) => a[0] - b[0]);
    const firstModel = versionModels[0][1];

    try {
      // Delete all messages after the user message
      await api.deleteMessagesAfter(convId, userMessageId);

      // Truncate local messages
      const idx = messages.findIndex((m) => m.id === userMessageId);
      if (idx === -1) return;
      messages = messages.slice(0, idx + 1);

      // Generate first version via resendMessage
      const assistantMsg: Message = {
        id: crypto.randomUUID(),
        conversationId: convId,
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
        messageType: 'text',
        toolError: false,
      };
      messages = [...messages, assistantMsg];
      streamingMessageId = assistantMsg.id;
      markStreaming(convId);

      let firstRealId = '';

      const firstParams = await loadParamsForModel(firstModel);
      await api.resendMessage(
        convId,
        firstModel,
        (event) => {
          if (event.type === 'started') {
            const msgs = getMessages(convId);
            const i = msgs.findIndex((m) => m.id === assistantMsg.id);
            if (i !== -1) {
              const updated = [...msgs];
              updated[i] = { ...updated[i], id: event.messageId };
              setMessages(convId, updated);
            }
            if (convId === activeConversationId) streamingMessageId = event.messageId;
            firstRealId = event.messageId;
          } else {
            handleEventFor(convId, event);
          }
        },
        firstParams.common,
        firstParams.providerParams,
      );

      // Reload to get the real message with proper IDs
      if (convId === activeConversationId) {
        await loadLatestMessages(convId);
      }

      // Generate subsequent versions using generateVersion
      for (let i = 1; i < versionModels.length; i++) {
        const vModel = versionModels[i][1];
        const currentMsgs = getMessages(convId);
        const currentAiMsg = currentMsgs.find((m) => m.id === firstRealId);
        if (!currentAiMsg) break;

        const placeholder: Message = {
          id: crypto.randomUUID(),
          conversationId: convId,
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
          messageType: 'text',
          toolError: false,
        };

        const aiIdx = currentMsgs.findIndex((m) => m.id === currentAiMsg.id);
        const updated = [...currentMsgs.slice(0, aiIdx), placeholder, ...currentMsgs.slice(aiIdx + 1)];
        setMessages(convId, updated);
        if (convId === activeConversationId) streamingMessageId = placeholder.id;
        markStreaming(convId);

        const vParams = await loadParamsForModel(vModel);
        await api.generateVersion(
          convId,
          firstRealId,
          vModel,
          (event) => {
            if (event.type === 'started') {
              const msgs = getMessages(convId);
              const j = msgs.findIndex((m) => m.id === placeholder.id);
              if (j !== -1) {
                const upd = [...msgs];
                upd[j] = { ...upd[j], id: event.messageId };
                setMessages(convId, upd);
              }
              if (convId === activeConversationId) streamingMessageId = event.messageId;
            } else {
              handleEventFor(convId, event);
            }
          },
          vParams.common,
          vParams.providerParams,
        );

        if (convId === activeConversationId) {
          await loadLatestMessages(convId);
        }
      }
    } catch (e) {
      console.error('Failed to resend all versions:', e);
      unmarkStreaming(convId);
      if (convId === activeConversationId) {
        await loadLatestMessages(convId);
      }
    }
  }

  async function handleRegenerate(messageId: string, modelId: string | null) {
    console.log('[+page] handleRegenerate called with messageId:', messageId, 'modelId:', modelId);
    invalidateRestore();
    if (!activeConversationId) return;

    const convId = activeConversationId;
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
      markStreaming(convId);

      const params = await loadParamsForModel(effectiveModelId);
      await api.regenerateMessage(
        convId,
        messageId,
        effectiveModelId,
        (event) => {
          if (event.type === 'started') {
            if (convId === activeConversationId) streamingMessageId = event.messageId;
          } else {
            handleEventFor(convId, event);
          }
        },
        params.common,
        params.providerParams,
      );

      if (convId === activeConversationId) {
        await loadLatestMessages(convId);
      }
    } catch (e) {
      console.error('Failed to regenerate:', e);
      unmarkStreaming(convId);
      if (convId === activeConversationId) {
        await loadLatestMessages(convId);
      }
    }
  }

  async function handleGenerateVersion(messageId: string) {
    console.log('[+page] handleGenerateVersion called with messageId:', messageId);
    invalidateRestore();
    if (!activeConversationId || !currentModelId) return;

    const convId = activeConversationId;

    try {
      // Find the current message to get its position
      const idx = messages.findIndex((m) => m.id === messageId);
      if (idx === -1) return;

      const currentMsg = messages[idx];

      // Create streaming placeholder for the new version at same position
      const placeholder: Message = {
        id: crypto.randomUUID(),
        conversationId: convId,
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
        messageType: 'text',
        toolError: false,
      };

      // Replace current message with placeholder, remove everything after
      messages = [...messages.slice(0, idx), placeholder, ...messages.slice(idx + 1)];
      streamingMessageId = placeholder.id;
      markStreaming(convId);

      const params = await loadParamsForModel(currentModelId);
      await api.generateVersion(
        convId,
        messageId,
        currentModelId,
        (event) => {
          if (event.type === 'started') {
            const msgs = getMessages(convId);
            const i = msgs.findIndex((m) => m.id === placeholder.id);
            if (i !== -1) {
              const updated = [...msgs];
              updated[i] = { ...updated[i], id: event.messageId };
              setMessages(convId, updated);
            }
            if (convId === activeConversationId) streamingMessageId = event.messageId;
          } else {
            handleEventFor(convId, event);
          }
        },
        params.common,
        params.providerParams,
      );

      if (convId === activeConversationId) {
        await loadLatestMessages(convId);
      }
    } catch (e) {
      console.error('Failed to generate version:', e);
      unmarkStreaming(convId);
      if (convId === activeConversationId) {
        await loadLatestMessages(convId);
      }
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

  async function handleGroupSend(content: string, modelIds: string[]) {
    invalidateRestore();
    if (!activeConversationId) return;

    const convId = activeConversationId;

    // Optimistic user message
    const userMsg: Message = {
      id: crypto.randomUUID(),
      conversationId: convId,
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
      messageType: 'text',
      toolError: false,
    };
    messages = [...messages, userMsg];

    // Create N placeholder assistant messages for compare view
    const now = new Date().toISOString();
    const placeholders: Message[] = modelIds.map((modelId, idx) => ({
      id: `placeholder-${idx}-${crypto.randomUUID()}`,
      conversationId: convId,
      role: 'assistant' as const,
      content: '',
      reasoning: null,
      modelId,
      status: 'streaming' as const,
      tokenCount: null,
      createdAt: now,
      versionGroupId: null,
      versionNumber: idx + 1,
      totalVersions: modelIds.length,
      messageType: 'text' as const,
      toolError: false,
    }));
    groupStreamingMessages = [...placeholders];
    markStreaming(convId);

    // Track mapping from placeholder index to real message IDs
    let startedCount = 0;
    let finishedCount = 0;

    try {
      await api.sendMessageGroup(
        convId,
        content,
        modelIds,
        (event) => {
          if (event.type === 'started') {
            // Map backend messageId to placeholder by order of 'started' events
            const placeholderIdx = startedCount;
            startedCount++;
            if (placeholderIdx < groupStreamingMessages.length) {
              groupStreamingMessages[placeholderIdx] = {
                ...groupStreamingMessages[placeholderIdx],
                id: event.messageId,
              };
              groupStreamingMessages = [...groupStreamingMessages];
            }
            return;
          }

          if (event.type === 'toolAuthRequest') {
            return;
          }

          // Find the message by messageId in groupStreamingMessages
          const idx = groupStreamingMessages.findIndex((m) => m.id === event.messageId);

          switch (event.type) {
            case 'delta':
              if (idx !== -1) {
                groupStreamingMessages[idx] = {
                  ...groupStreamingMessages[idx],
                  content: groupStreamingMessages[idx].content + event.content,
                };
                groupStreamingMessages = [...groupStreamingMessages];
              }
              break;
            case 'reasoning':
              if (idx !== -1) {
                groupStreamingMessages[idx] = {
                  ...groupStreamingMessages[idx],
                  reasoning: (groupStreamingMessages[idx].reasoning ?? '') + event.content,
                };
                groupStreamingMessages = [...groupStreamingMessages];
              }
              break;
            case 'usage':
              if (idx !== -1) {
                groupStreamingMessages[idx] = {
                  ...groupStreamingMessages[idx],
                  tokenCount: event.promptTokens + event.completionTokens,
                };
                groupStreamingMessages = [...groupStreamingMessages];
              }
              break;
            case 'finished':
              if (idx !== -1) {
                groupStreamingMessages[idx] = {
                  ...groupStreamingMessages[idx],
                  status: 'done',
                };
                groupStreamingMessages = [...groupStreamingMessages];
              }
              finishedCount++;
              if (finishedCount >= modelIds.length) {
                unmarkStreaming(convId);
              }
              break;
            case 'error':
              if (idx !== -1) {
                groupStreamingMessages[idx] = {
                  ...groupStreamingMessages[idx],
                  status: 'error',
                  content: groupStreamingMessages[idx].content || event.message,
                };
                groupStreamingMessages = [...groupStreamingMessages];
              }
              finishedCount++;
              if (finishedCount >= modelIds.length) {
                unmarkStreaming(convId);
              }
              break;
          }
        },
      );
      // After all done, reload messages from DB
      if (convId === activeConversationId) {
        await loadLatestMessages(convId);
      }
      const msgCount = getMessages(convId).length;
      void tryAutoRename(convId, msgCount);
    } catch (e) {
      console.error('Group send failed:', e);
      unmarkStreaming(convId);
      groupStreamingMessages = [];
      if (convId === activeConversationId) {
        await loadLatestMessages(convId);
      }
    }
  }

  function handleExitGroupCompare() {
    groupStreamingMessages = [];
    void loadLatestMessages(activeConversationId);
  }

  async function handleFork(messageId: string) {
    if (!activeConversationId) return;
    try {
      const newConv = await api.forkConversation(activeConversationId, messageId);
      conversations = [newConv, ...conversations];
      conversationCreated.set(newConv);
      handleConversationSelect({ conversationId: newConv.id });
      forkedBannerVisible = true;
      setTimeout(() => {
        forkedBannerVisible = false;
      }, 4000);
    } catch (e) {
      console.error('Failed to fork conversation:', e);
    }
  }

  function handleChatEvent(event: { type: string; [key: string]: any }) {
    console.log('[+page] handleChatEvent:', event);
    switch (event.type) {
      case 'send':
        handleSend(event.content);
        break;
      case 'groupSend':
        handleGroupSend(event.content, event.modelIds);
        break;
      case 'exitGroupCompare':
        handleExitGroupCompare();
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
      case 'fork':
        handleFork(event.messageId);
        break;
    }
  }
</script>

<SidebarProvider>
  <AppSidebar
    {activeConversationId}
    onConversationSelect={handleConversationSelect}
  />

  <SidebarInset>
    <div class="expand-sidebar-trigger-wrapper">
      <SidebarTrigger />
    </div>
    {#if activeConversationId}
      {#if forkedBannerVisible}
        <div class="fork-banner">{i18n.t.forkedBanner}</div>
      {/if}
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
        {groupStreamingMessages}
        bind:selectedModelId={currentModelId}
        onModelSelect={handleModelSelect}
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

<ToolAuthDialog />

<style>
  .expand-sidebar-trigger-wrapper {
    position: absolute;
    top: 0.55rem;
    left: 0.5rem;
    z-index: 5;
  }
  :global([data-state="expanded"] ~ [data-slot="sidebar-inset"]) .expand-sidebar-trigger-wrapper {
    display: none;
  }

  .fork-banner {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
    text-align: center;
    padding: 0.5rem;
    font-size: 0.82rem;
    font-weight: 500;
    flex-shrink: 0;
  }
</style>
