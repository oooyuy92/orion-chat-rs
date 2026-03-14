<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Slider } from '$lib/components/ui/slider';
  import { Textarea } from '$lib/components/ui/textarea';
  import { api } from '$lib/utils/invoke';
  import { resolveModelLabel } from '$lib/utils/modelDisplay';
  import { i18n, type Language } from '$lib/stores/i18n.svelte';
  import { alignProviderParams, defaultProviderParams } from '$lib/stores/modelParams';
  import type {
    Assistant,
    ProviderConfig,
    ProviderParams,
    ProviderType,
    AnthropicThinking,
    AnthropicEffort,
    ReasoningEffort,
    GeminiThinkingLevel,
    AssistantExtraParams,
  } from '$lib/types';
  type ModelOption = {
    id: string;
    label: string;
    providerName: string;
    providerType: ProviderType;
  };

  const language = $derived(i18n.language as Language);

  const t = $derived.by(() => {
    if (language === 'en') {
      return {
        assistants: 'Assistant Settings',
        create: 'New Assistant',
        creating: 'Creating...',
        save: 'Save',
        saving: 'Saving...',
        delete: 'Delete',
        deleting: 'Deleting...',
        copy: 'Copy',
        copied: 'Assistant copied.',
        copySuffix: 'Copy',
        loading: 'Loading assistants...',
        assistantName: 'Name',
        systemPrompt: 'System Prompt',
        systemPromptPlaceholder: 'Describe the assistant behavior shown only to the model.',
        defaultModel: 'Default Model',
        noDefaultModel: 'None',
        commonParams: 'Common Parameters',
        providerParams: 'Provider Parameters',
        emptyTitle: 'Select an assistant',
        emptyHint: 'Create one from the left list, then edit its defaults here.',
        detailHint: 'These defaults apply when this assistant is selected for a conversation.',
        saved: 'Assistant saved.',
        created: 'Assistant created.',
        deleted: 'Assistant deleted.',
      };
    }

    return {
      assistants: '助手设置',
      create: '新建助手',
      creating: '创建中...',
      save: '保存',
      saving: '保存中...',
      delete: '删除',
      deleting: '删除中...',
      copy: '复制',
      copied: '助手已复制。',
      copySuffix: '副本',
      loading: '正在加载助手...',
      assistantName: '名称',
      systemPrompt: '系统提示词',
      systemPromptPlaceholder: '描述该助手的行为，这些内容只会发送给模型。',
      defaultModel: '默认模型',
      noDefaultModel: '无',
      commonParams: '通用参数',
      providerParams: 'Provider 专属参数',
      emptyTitle: '请选择一个助手',
      emptyHint: '可先在左侧新建助手，再在此编辑默认值。',
      detailHint: '当该助手被绑定到对话时，这里的默认值会参与发送请求。',
      saved: '助手已保存。',
      created: '助手已创建。',
      deleted: '助手已删除。',
    };
  });

  let assistants = $state<Assistant[]>([]);
  let providers = $state<ProviderConfig[]>([]);
  let loading = $state(true);
  let creating = $state(false);
  let saving = $state(false);
  let deleting = $state(false);
  let error = $state('');
  let success = $state('');
  let selectedAssistantId = $state('');
  let contextMenu = $state<{ x: number; y: number; assistantId: string } | null>(null);

  let draftName = $state('');
  let draftSystemPrompt = $state('');
  let draftModelId = $state('');
  let draftTemperature = $state<number | null>(null);
  let draftTopP = $state<number | null>(null);
  let draftMaxTokens = $state<number | null>(null);
  let draftProviderParams = $state<ProviderParams | null>(null);
  let commonParamsExpanded = $state(false);
  let providerParamsExpanded = $state(false);
  let titleEditing = $state(false);
  let titleInput = $state<HTMLInputElement | null>(null);
  let saveQueued = $state(false);

  const selectedAssistant = $derived(
    assistants.find((assistant) => assistant.id === selectedAssistantId) ?? null,
  );

  const modelOptions = $derived.by((): ModelOption[] => {
    return providers
      .filter((provider) => provider.enabled)
      .flatMap((provider) =>
        provider.models
          .filter((model) => model.enabled)
          .map((model) => ({
            id: model.id,
            label: resolveModelLabel(model),
            providerName: provider.name,
            providerType: provider.providerType,
          })),
      );
  });

  function getProviderTypeForModel(modelId: string | null | undefined): ProviderType | null {
    if (!modelId) return null;
    for (const provider of providers) {
      if (provider.models.some((model) => model.id === modelId)) {
        return provider.providerType;
      }
    }
    return null;
  }

  function isProviderParams(value: AssistantExtraParams): value is ProviderParams {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
    const tag = (value as { provider_type?: unknown }).provider_type;
    return tag === 'openaiCompat' || tag === 'anthropic' || tag === 'gemini' || tag === 'ollama';
  }

  function normalizeProviderParams(
    extraParams: AssistantExtraParams,
    providerType: ProviderType | null,
  ): ProviderParams | null {
    if (!providerType) return null;
    return alignProviderParams(providerType, isProviderParams(extraParams) ? extraParams : null);
  }

  function syncDraft(assistant: Assistant | null) {
    titleEditing = false;
    selectedAssistantId = assistant?.id ?? '';
    draftName = assistant?.name ?? '';
    draftSystemPrompt = assistant?.systemPrompt ?? '';
    draftModelId = assistant?.modelId ?? '';
    draftTemperature = assistant?.temperature ?? null;
    draftTopP = assistant?.topP ?? null;
    draftMaxTokens = assistant?.maxTokens ?? null;
    draftProviderParams = normalizeProviderParams(
      assistant?.extraParams ?? {},
      getProviderTypeForModel(assistant?.modelId),
    );
  }

  async function loadData() {
    loading = true;
    error = '';

    try {
      const [nextAssistants, nextProviders] = await Promise.all([
        api.listAssistants(),
        api.listProviders(),
      ]);
      assistants = nextAssistants;
      providers = nextProviders;
      const nextSelected = nextAssistants.find((assistant) => assistant.id === selectedAssistantId) ?? nextAssistants[0] ?? null;
      syncDraft(nextSelected);
    } catch (e) {
      console.error('Failed to load assistants:', e);
      error = language === 'en' ? `Load failed: ${e}` : `加载失败：${e}`;
      syncDraft(null);
    } finally {
      loading = false;
    }
  }

  async function handleCreate() {
    creating = true;
    error = '';
    success = '';

    try {
      const assistant = await api.createAssistant(language === 'en' ? 'New Assistant' : '新助手');
      assistants = [assistant, ...assistants];
      syncDraft(assistant);
      success = t.created;
    } catch (e) {
      console.error('Failed to create assistant:', e);
      error = language === 'en' ? `Create failed: ${e}` : `创建失败：${e}`;
    } finally {
      creating = false;
    }
  }

  function buildUpdatedAssistant(base: Assistant): Assistant {
    return {
      ...base,
      name: draftName.trim() || base.name,
      systemPrompt: draftSystemPrompt.trim() || null,
      modelId: draftModelId || null,
      temperature: draftTemperature,
      topP: draftTopP,
      maxTokens: draftMaxTokens,
      extraParams: draftProviderParams ?? {},
    };
  }

  function hasAssistantChanges(base: Assistant, updated: Assistant): boolean {
    return (
      base.name !== updated.name ||
      base.systemPrompt !== updated.systemPrompt ||
      base.modelId !== updated.modelId ||
      base.temperature !== updated.temperature ||
      base.topP !== updated.topP ||
      base.maxTokens !== updated.maxTokens ||
      JSON.stringify(base.extraParams ?? {}) !== JSON.stringify(updated.extraParams ?? {})
    );
  }

  async function saveCurrentAssistant(showSuccess = false) {
    if (!selectedAssistant) return;

    const updated = buildUpdatedAssistant(selectedAssistant);
    if (!hasAssistantChanges(selectedAssistant, updated)) {
      return;
    }

    if (saving) {
      saveQueued = true;
      return;
    }

    saving = true;
    error = '';
    if (!showSuccess) success = '';

    try {
      await api.updateAssistant(updated);
      assistants = assistants.map((assistant) =>
        assistant.id === updated.id ? updated : assistant,
      );
      syncDraft(updated);
      if (showSuccess) success = t.saved;
    } catch (e) {
      console.error('Failed to save assistant:', e);
      error = language === 'en' ? `Save failed: ${e}` : `保存失败：${e}`;
    } finally {
      saving = false;
      if (saveQueued) {
        saveQueued = false;
        void saveCurrentAssistant(showSuccess);
      }
    }
  }

  async function startTitleEdit() {
    if (!selectedAssistant) return;
    titleEditing = true;
    await tick();
    titleInput?.focus();
    titleInput?.select();
  }

  function cancelTitleEdit() {
    draftName = selectedAssistant?.name ?? '';
    titleEditing = false;
  }

  async function commitTitleEdit() {
    titleEditing = false;
    await saveCurrentAssistant();
  }

  function handleTitleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      void commitTitleEdit();
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      cancelTitleEdit();
    }
  }

  async function handleDelete() {
    if (!selectedAssistant) return;

    deleting = true;
    error = '';
    success = '';

    try {
      const currentId = selectedAssistant.id;
      const currentIndex = assistants.findIndex((assistant) => assistant.id === currentId);
      await api.deleteAssistant(currentId);
      const nextAssistants = assistants.filter((assistant) => assistant.id !== currentId);
      assistants = nextAssistants;
      const nextSelected = nextAssistants[currentIndex] ?? nextAssistants[currentIndex - 1] ?? null;
      syncDraft(nextSelected);
      success = t.deleted;
    } catch (e) {
      console.error('Failed to delete assistant:', e);
      error = language === 'en' ? `Delete failed: ${e}` : `删除失败：${e}`;
    } finally {
      deleting = false;
    }
  }

  function handleSelectAssistant(id: string) {
    const assistant = assistants.find((item) => item.id === id) ?? null;
    syncDraft(assistant);
    error = '';
    success = '';
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function openContextMenu(event: MouseEvent, assistantId: string) {
    event.preventDefault();
    handleSelectAssistant(assistantId);
    contextMenu = { x: event.clientX, y: event.clientY, assistantId };
  }

  async function handleCopy(assistantId: string) {
    const source = assistants.find((assistant) => assistant.id === assistantId);
    if (!source) return;

    creating = true;
    error = '';
    success = '';
    closeContextMenu();

    try {
      const copyName = `${source.name} ${t.copySuffix}`.trim();
      const created = await api.createAssistant(
        copyName,
        source.systemPrompt ?? undefined,
        source.modelId ?? undefined,
        source.temperature ?? undefined,
        source.topP ?? undefined,
        source.maxTokens ?? undefined,
      );
      const duplicated: Assistant = {
        ...created,
        icon: source.icon,
        extraParams: source.extraParams,
      };
      await api.updateAssistant(duplicated);
      assistants = [duplicated, ...assistants];
      syncDraft(duplicated);
      success = t.copied;
    } catch (e) {
      console.error('Failed to copy assistant:', e);
      error = language === 'en' ? `Copy failed: ${e}` : `复制失败：${e}`;
    } finally {
      creating = false;
    }
  }

  async function handleDeleteById(assistantId: string) {
    handleSelectAssistant(assistantId);
    closeContextMenu();
    await handleDelete();
  }

  function handleModelChange(nextModelId: string) {
    draftModelId = nextModelId;
    const nextProviderType = getProviderTypeForModel(nextModelId || null);

    if (!nextProviderType) {
      draftProviderParams = null;
      void saveCurrentAssistant();
      return;
    }

    const expectedTag = defaultProviderParams(nextProviderType).provider_type;
    if (draftProviderParams?.provider_type !== expectedTag) {
      draftProviderParams = defaultProviderParams(nextProviderType);
    }

    void saveCurrentAssistant();
  }

  function setTemperature(value: number) {
    draftTemperature = value;
    void saveCurrentAssistant();
  }

  function setTopP(value: number) {
    draftTopP = value;
    void saveCurrentAssistant();
  }

  function setMaxTokens(event: Event) {
    const value = (event.target as HTMLInputElement).valueAsNumber;
    draftMaxTokens = Number.isNaN(value) ? null : value;
    void saveCurrentAssistant();
  }

  function thinkingModeLabel(mode: string) {
    switch (mode) {
      case 'adaptive':
        return i18n.t.adaptive;
      case 'enabled':
        return i18n.t.enabled;
      default:
        return i18n.t.disabled;
    }
  }

  function levelLabel(value: string) {
    switch (value) {
      case 'low':
        return i18n.t.low;
      case 'medium':
        return i18n.t.medium;
      case 'high':
        return i18n.t.high;
      default:
        return i18n.t.default;
    }
  }

  function thinkLabel(value: boolean | null) {
    if (value === null) return i18n.t.default;
    return value ? i18n.t.enabled : i18n.t.disabled;
  }

  function getThinkingMode(): string {
    if (draftProviderParams?.provider_type !== 'anthropic') return 'disabled';
    const thinking = draftProviderParams.thinking;
    if (!thinking) return 'disabled';
    return thinking.type;
  }

  function setThinkingMode(mode: string) {
    if (draftProviderParams?.provider_type !== 'anthropic') return;

    let thinking: AnthropicThinking | null;
    switch (mode) {
      case 'adaptive':
        thinking = { type: 'adaptive' };
        break;
      case 'enabled':
        thinking = { type: 'enabled', budgetTokens: 10000 };
        break;
      default:
        thinking = { type: 'disabled' };
        break;
    }

    draftProviderParams = { ...draftProviderParams, thinking };
    void saveCurrentAssistant();
  }

  function setBudgetTokens(event: Event) {
    if (draftProviderParams?.provider_type !== 'anthropic') return;
    const value = (event.target as HTMLInputElement).valueAsNumber;
    if (Number.isNaN(value)) return;
    draftProviderParams = {
      ...draftProviderParams,
      thinking: { type: 'enabled', budgetTokens: value },
    };
    void saveCurrentAssistant();
  }

  function setAnthropicEffort(value: string) {
    if (draftProviderParams?.provider_type !== 'anthropic') return;
    draftProviderParams = {
      ...draftProviderParams,
      effort: (value || null) as AnthropicEffort | null,
    };
    void saveCurrentAssistant();
  }

  function setAnthropicTopK(event: Event) {
    if (draftProviderParams?.provider_type !== 'anthropic') return;
    const value = (event.target as HTMLInputElement).valueAsNumber;
    draftProviderParams = {
      ...draftProviderParams,
      topK: Number.isNaN(value) ? null : value,
    };
    void saveCurrentAssistant();
  }

  function setGeminiThinkingLevel(value: string) {
    if (draftProviderParams?.provider_type !== 'gemini') return;
    draftProviderParams = {
      ...draftProviderParams,
      thinkingLevel: (value || null) as GeminiThinkingLevel | null,
    };
    void saveCurrentAssistant();
  }

  function setGeminiThinkingBudget(event: Event) {
    if (draftProviderParams?.provider_type !== 'gemini') return;
    const value = (event.target as HTMLInputElement).valueAsNumber;
    draftProviderParams = {
      ...draftProviderParams,
      thinkingBudget: Number.isNaN(value) ? null : value,
    };
    void saveCurrentAssistant();
  }

  function setFreqPenalty(value: number) {
    if (draftProviderParams?.provider_type !== 'openaiCompat') return;
    draftProviderParams = { ...draftProviderParams, frequencyPenalty: value };
    void saveCurrentAssistant();
  }

  function setPresPenalty(value: number) {
    if (draftProviderParams?.provider_type !== 'openaiCompat') return;
    draftProviderParams = { ...draftProviderParams, presencePenalty: value };
    void saveCurrentAssistant();
  }

  function setReasoningEffort(value: string) {
    if (draftProviderParams?.provider_type !== 'openaiCompat') return;
    draftProviderParams = {
      ...draftProviderParams,
      reasoningEffort: (value || null) as ReasoningEffort | null,
    };
    void saveCurrentAssistant();
  }

  function setOpenAISeed(event: Event) {
    if (draftProviderParams?.provider_type !== 'openaiCompat') return;
    const value = (event.target as HTMLInputElement).valueAsNumber;
    draftProviderParams = {
      ...draftProviderParams,
      seed: Number.isNaN(value) ? null : value,
    };
    void saveCurrentAssistant();
  }

  function setMaxCompletionTokens(event: Event) {
    if (draftProviderParams?.provider_type !== 'openaiCompat') return;
    const value = (event.target as HTMLInputElement).valueAsNumber;
    draftProviderParams = {
      ...draftProviderParams,
      maxCompletionTokens: Number.isNaN(value) ? null : value,
    };
    void saveCurrentAssistant();
  }

  function setOllamaThink(value: boolean | null) {
    if (draftProviderParams?.provider_type !== 'ollama') return;
    draftProviderParams = { ...draftProviderParams, think: value };
    void saveCurrentAssistant();
  }

  function setNumCtx(event: Event) {
    if (draftProviderParams?.provider_type !== 'ollama') return;
    const value = (event.target as HTMLInputElement).valueAsNumber;
    draftProviderParams = {
      ...draftProviderParams,
      numCtx: Number.isNaN(value) ? null : value,
    };
    void saveCurrentAssistant();
  }

  function setRepeatPenalty(value: number) {
    if (draftProviderParams?.provider_type !== 'ollama') return;
    draftProviderParams = { ...draftProviderParams, repeatPenalty: value };
    void saveCurrentAssistant();
  }

  function setMinP(value: number) {
    if (draftProviderParams?.provider_type !== 'ollama') return;
    draftProviderParams = { ...draftProviderParams, minP: value };
    void saveCurrentAssistant();
  }

  function setKeepAlive(event: Event) {
    if (draftProviderParams?.provider_type !== 'ollama') return;
    const value = (event.target as HTMLInputElement).value;
    draftProviderParams = { ...draftProviderParams, keepAlive: value || null };
    void saveCurrentAssistant();
  }

  onMount(() => {
    loadData();
  });
</script>

<div class="assistant-settings-root">
  <section class="assistant-list-panel">
    <div class="panel-head">
      <h2>{t.assistants}</h2>
      <Button variant="outline" size="sm" onclick={handleCreate} disabled={creating}>
        {creating ? t.creating : t.create}
      </Button>
    </div>

    {#if loading}
      <p class="panel-status">{t.loading}</p>
    {:else if assistants.length === 0}
      <p class="panel-status">{i18n.t.noAssistantsYet}</p>
    {:else}
      <div class="assistant-list-scroll">
        {#each assistants as assistant (assistant.id)}
          <button
            class="assistant-card"
            class:is-active={assistant.id === selectedAssistantId}
            onclick={() => handleSelectAssistant(assistant.id)}
            oncontextmenu={(event) => openContextMenu(event, assistant.id)}
          >
            <span class="assistant-card-name">{assistant.name}</span>
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <section class="assistant-detail-panel">
    {#if error}
      <p class="status-banner error">{error}</p>
    {/if}
    {#if success}
      <p class="status-banner success">{success}</p>
    {/if}

    {#if selectedAssistant}
      <div class="detail-header">
        <div class="detail-title-block">
          {#if titleEditing}
            <!-- svelte-ignore a11y_autofocus -->
            <input
              bind:this={titleInput}
              bind:value={draftName}
              class="editable-title-input"
              onblur={commitTitleEdit}
              onkeydown={handleTitleKeydown}
            />
          {:else}
            <button type="button" class="editable-title" onclick={() => void startTitleEdit()}>
              {draftName || selectedAssistant.name}
            </button>
          {/if}
          <p>{t.detailHint}</p>
        </div>
        <div class="header-actions">
          <Button variant="outline" onclick={handleDelete} disabled={deleting}>
            {deleting ? t.deleting : t.delete}
          </Button>
        </div>
      </div>

      <div class="detail-section">
        <label class="field-label">
          <span>{t.defaultModel}</span>
          <select
            class="form-input"
            value={draftModelId}
            onchange={(event) => handleModelChange((event.target as HTMLSelectElement).value)}
          >
            <option value="">{t.noDefaultModel}</option>
            {#each modelOptions as model (model.id)}
              <option value={model.id}>{model.providerName} / {model.label}</option>
            {/each}
          </select>
        </label>
      </div>

      <div class="detail-section">
        <label class="field-label">
          <span>{t.systemPrompt}</span>
          <Textarea
            bind:value={draftSystemPrompt}
            rows={14}
            placeholder={t.systemPromptPlaceholder}
            class="system-prompt"
            onblur={() => void saveCurrentAssistant()}
          />
        </label>
      </div>

      <section class="param-section detail-section collapsible-section">
        <button
          type="button"
          class="collapsible-trigger"
          aria-expanded={commonParamsExpanded}
          onclick={() => (commonParamsExpanded = !commonParamsExpanded)}
        >
          <span class="section-label collapsible-label">{t.commonParams}</span>
          <span class="collapse-icon" class:is-open={commonParamsExpanded}>⌄</span>
        </button>

        {#if commonParamsExpanded}
          <div class="collapsible-body">
            <div class="param-row">
              <div class="param-label-row">
                <span>{i18n.t.temperature}</span>
                <span class="param-value">{draftTemperature ?? '—'}</span>
              </div>
              <Slider
                type="single"
                value={draftTemperature ?? 1}
                min={0}
                max={2}
                step={0.1}
                onValueChange={setTemperature}
              />
            </div>

            <div class="param-row">
              <div class="param-label-row">
                <span>{i18n.t.topP}</span>
                <span class="param-value">{draftTopP ?? '—'}</span>
              </div>
              <Slider
                type="single"
                value={draftTopP ?? 1}
                min={0}
                max={1}
                step={0.01}
                onValueChange={setTopP}
              />
            </div>

            <div class="param-row">
              <span>{i18n.t.maxTokens}</span>
              <Input
                type="number"
                placeholder={i18n.t.default}
                value={draftMaxTokens ?? ''}
                onchange={setMaxTokens}
                class="param-input"
              />
            </div>
          </div>
        {/if}
      </section>

      {#if draftProviderParams}
        <section class="param-section detail-section collapsible-section">
          <button
            type="button"
            class="collapsible-trigger"
            aria-expanded={providerParamsExpanded}
            onclick={() => (providerParamsExpanded = !providerParamsExpanded)}
          >
            <span class="section-label collapsible-label">{t.providerParams}</span>
            <span class="collapse-icon" class:is-open={providerParamsExpanded}>⌄</span>
          </button>

          {#if providerParamsExpanded}
            <div class="collapsible-body">
              {#if draftProviderParams.provider_type === 'anthropic'}
            <div class="param-row">
              <span>{i18n.t.thinking}</span>
              <div class="btn-group">
                {#each ['disabled', 'adaptive', 'enabled'] as mode}
                  <button
                    type="button"
                    class="btn-option"
                    class:active={getThinkingMode() === mode}
                    onclick={() => setThinkingMode(mode)}
                  >{thinkingModeLabel(mode)}</button>
                {/each}
              </div>
            </div>

            {#if getThinkingMode() === 'enabled'}
              <div class="param-row">
                <span>{i18n.t.budgetTokens}</span>
                <Input
                  type="number"
                  placeholder="10000"
                  value={draftProviderParams.thinking?.type === 'enabled' ? draftProviderParams.thinking.budgetTokens : ''}
                  onchange={setBudgetTokens}
                  class="param-input"
                />
              </div>
            {/if}

            <div class="param-row">
              <span>{i18n.t.effort}</span>
              <div class="btn-group">
                {#each ['', 'low', 'medium', 'high'] as value}
                  <button
                    type="button"
                    class="btn-option"
                    class:active={String(draftProviderParams.effort ?? '') === value}
                    onclick={() => setAnthropicEffort(value)}
                  >{levelLabel(value)}</button>
                {/each}
              </div>
            </div>

            <div class="param-row">
              <span>Top K</span>
              <Input
                type="number"
                placeholder={i18n.t.default}
                value={draftProviderParams.topK ?? ''}
                onchange={setAnthropicTopK}
                class="param-input"
              />
            </div>
          {/if}

          {#if draftProviderParams.provider_type === 'gemini'}
            <div class="param-row">
              <span>{i18n.t.thinkingLevel}</span>
              <div class="btn-group">
                {#each ['', 'low', 'medium', 'high'] as value}
                  <button
                    type="button"
                    class="btn-option"
                    class:active={String(draftProviderParams.thinkingLevel ?? '') === value}
                    onclick={() => setGeminiThinkingLevel(value)}
                  >{levelLabel(value)}</button>
                {/each}
              </div>
            </div>

            <div class="param-row">
              <span>{i18n.t.thinkingBudget}</span>
              <Input
                type="number"
                placeholder={i18n.t.default}
                value={draftProviderParams.thinkingBudget ?? ''}
                onchange={setGeminiThinkingBudget}
                class="param-input"
              />
            </div>
          {/if}

          {#if draftProviderParams.provider_type === 'openaiCompat'}
            <div class="param-row">
              <div class="param-label-row">
                <span>{i18n.t.frequencyPenalty}</span>
                <span class="param-value">{draftProviderParams.frequencyPenalty ?? '—'}</span>
              </div>
              <Slider
                type="single"
                value={draftProviderParams.frequencyPenalty ?? 0}
                min={-2}
                max={2}
                step={0.1}
                onValueChange={setFreqPenalty}
              />
            </div>

            <div class="param-row">
              <div class="param-label-row">
                <span>{i18n.t.presencePenalty}</span>
                <span class="param-value">{draftProviderParams.presencePenalty ?? '—'}</span>
              </div>
              <Slider
                type="single"
                value={draftProviderParams.presencePenalty ?? 0}
                min={-2}
                max={2}
                step={0.1}
                onValueChange={setPresPenalty}
              />
            </div>

            <div class="param-row">
              <span>{i18n.t.reasoningEffort}</span>
              <div class="btn-group">
                {#each ['', 'low', 'medium', 'high'] as value}
                  <button
                    type="button"
                    class="btn-option"
                    class:active={String(draftProviderParams.reasoningEffort ?? '') === value}
                    onclick={() => setReasoningEffort(value)}
                  >{levelLabel(value)}</button>
                {/each}
              </div>
            </div>

            <div class="param-row param-grid-two">
              <label class="field-label compact">
                <span>{i18n.t.seed}</span>
                <Input
                  type="number"
                  placeholder={i18n.t.default}
                  value={draftProviderParams.seed ?? ''}
                  onchange={setOpenAISeed}
                  class="param-input"
                />
              </label>

              <label class="field-label compact">
                <span>{i18n.t.maxCompletionTokens}</span>
                <Input
                  type="number"
                  placeholder={i18n.t.default}
                  value={draftProviderParams.maxCompletionTokens ?? ''}
                  onchange={setMaxCompletionTokens}
                  class="param-input"
                />
              </label>
            </div>
          {/if}

          {#if draftProviderParams.provider_type === 'ollama'}
            <div class="param-row">
              <span>Think</span>
              <div class="btn-group">
                <button
                  type="button"
                  class="btn-option"
                  class:active={draftProviderParams.think === null || draftProviderParams.think === undefined}
                  onclick={() => setOllamaThink(null)}
                >{thinkLabel(null)}</button>
                <button
                  type="button"
                  class="btn-option"
                  class:active={draftProviderParams.think === false}
                  onclick={() => setOllamaThink(false)}
                >{thinkLabel(false)}</button>
                <button
                  type="button"
                  class="btn-option"
                  class:active={draftProviderParams.think === true}
                  onclick={() => setOllamaThink(true)}
                >{thinkLabel(true)}</button>
              </div>
            </div>

            <div class="param-row param-grid-two">
              <label class="field-label compact">
                <span>{i18n.t.numCtx}</span>
                <Input
                  type="number"
                  placeholder={i18n.t.default}
                  value={draftProviderParams.numCtx ?? ''}
                  onchange={setNumCtx}
                  class="param-input"
                />
              </label>

              <label class="field-label compact">
                <span>{i18n.t.keepAlive}</span>
                <Input
                  type="text"
                  placeholder="5m"
                  value={draftProviderParams.keepAlive ?? ''}
                  onchange={setKeepAlive}
                  class="param-input"
                />
              </label>
            </div>

            <div class="param-row">
              <div class="param-label-row">
                <span>{i18n.t.repeatPenalty}</span>
                <span class="param-value">{draftProviderParams.repeatPenalty ?? '—'}</span>
              </div>
              <Slider
                type="single"
                value={draftProviderParams.repeatPenalty ?? 1}
                min={0}
                max={2}
                step={0.1}
                onValueChange={setRepeatPenalty}
              />
            </div>

            <div class="param-row">
              <div class="param-label-row">
                <span>{i18n.t.minP}</span>
                <span class="param-value">{draftProviderParams.minP ?? '—'}</span>
              </div>
              <Slider
                type="single"
                value={draftProviderParams.minP ?? 0}
                min={0}
                max={1}
                step={0.01}
                onValueChange={setMinP}
              />
            </div>
              {/if}
            </div>
          {/if}
        </section>
      {/if}
    {:else}
      <div class="empty-state">
        <h3>{t.emptyTitle}</h3>
        <p>{t.emptyHint}</p>
      </div>
    {/if}
  </section>
</div>

{#if contextMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="context-overlay"
    role="button"
    tabindex="0"
    aria-label={i18n.t.close}
    onclick={closeContextMenu}
    onkeydown={(event) => {
      if (event.key === 'Escape' || event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        closeContextMenu();
      }
    }}
    oncontextmenu={(event) => {
      event.preventDefault();
      closeContextMenu();
    }}
  >
    <div class="context-menu" style={`left: ${contextMenu.x}px; top: ${contextMenu.y}px;`}>
      <button class="context-item" onclick={() => handleCopy(contextMenu!.assistantId)}>{t.copy}</button>
      <button class="context-item danger" onclick={() => handleDeleteById(contextMenu!.assistantId)}>{t.delete}</button>
    </div>
  </div>
{/if}

<style>
  .assistant-settings-root {
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr);
    gap: 0.875rem;
    min-height: 0;
    min-width: 0;
    width: 100%;
    height: 100%;
    align-self: stretch;
  }

  .assistant-list-panel,
  .assistant-detail-panel {
    border: 1px solid var(--border);
    border-radius: 1rem;
    background: var(--card);
    min-height: 0;
    min-width: 0;
    height: 100%;
  }

  .assistant-list-panel {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel-head,
  .detail-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 1rem;
    border-bottom: 1px solid var(--border);
  }

  .panel-head h2,
  .empty-state h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }

  .detail-title-block {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .editable-title {
    width: fit-content;
    max-width: 100%;
    padding: 0;
    margin: 0;
    border: none;
    background: transparent;
    color: var(--foreground);
    font-size: 1rem;
    font-weight: 600;
    line-height: 1.3;
    text-align: left;
    cursor: text;
  }

  .editable-title-input {
    width: min(100%, 24rem);
    padding: 0.1rem 0;
    border: none;
    border-bottom: 1px solid var(--primary);
    background: transparent;
    color: var(--foreground);
    font-size: 1rem;
    font-weight: 600;
    line-height: 1.3;
    outline: none;
  }

  .detail-header p,
  .empty-state p,
  .panel-status,
  .status-banner {
    margin: 0;
    color: var(--muted-foreground);
    font-size: 0.875rem;
  }

  .assistant-list-scroll {
    display: flex;
    flex: 1;
    min-height: 0;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.75rem;
    overflow: auto;
  }

  .assistant-card {
    display: flex;
    align-items: center;
    width: 100%;
    min-height: 2.5rem;
    padding: 0.6rem 0.75rem;
    border-radius: 0.75rem;
    border: 1px solid transparent;
    background: var(--muted);
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;
  }

  .assistant-card:hover {
    border-color: var(--border);
    background: color-mix(in oklab, var(--muted) 85%, var(--background));
  }

  .assistant-card.is-active {
    border-color: var(--primary);
    background: color-mix(in oklab, var(--primary) 10%, var(--card));
  }

  .assistant-card-name {
    color: var(--foreground);
    font-size: 0.84rem;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .assistant-detail-panel {
    padding: 1rem;
    overflow: auto;
  }

  .detail-header {
    padding: 0 0 1rem;
    margin-bottom: 1rem;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  .detail-section {
    margin-bottom: 1rem;
  }

  .param-grid-two {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1rem;
  }

  .field-label {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    color: var(--foreground);
    font-size: 0.875rem;
    font-weight: 500;
  }

  .field-label.compact {
    gap: 0.4rem;
  }

  .form-input {
    width: 100%;
  }

  .param-section {
    margin: 0;
    padding: 1rem;
    border: 1px solid var(--border);
    border-radius: 1rem;
    background: color-mix(in oklab, var(--muted) 45%, var(--card));
  }

  .detail-section.param-section {
    margin-bottom: 1rem;
  }

  .section-label {
    padding: 0 0.35rem;
    color: var(--muted-foreground);
    font-size: 0.8rem;
    font-weight: 600;
  }

  .collapsible-section {
    padding-top: 0.8rem;
  }

  .collapsible-trigger {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    border: none;
    background: transparent;
    padding: 0;
    cursor: pointer;
  }

  .collapsible-label {
    padding-left: 0;
  }

  .collapse-icon {
    color: var(--muted-foreground);
    font-size: 0.9rem;
    line-height: 1;
    transition: transform 0.15s ease;
  }

  .collapse-icon.is-open {
    transform: rotate(180deg);
  }

  .collapsible-body {
    margin-top: 0.9rem;
  }

  .param-row {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    margin-bottom: 1rem;
  }

  .param-row:last-child {
    margin-bottom: 0;
  }

  .param-label-row {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .param-value {
    color: var(--muted-foreground);
    font-variant-numeric: tabular-nums;
  }

  .btn-group {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .btn-option {
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--background);
    color: var(--foreground);
    padding: 0.35rem 0.75rem;
    font-size: 0.8rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-option.active {
    border-color: var(--primary);
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .status-banner {
    border-radius: 0.75rem;
    padding: 0.75rem 0.9rem;
    margin-bottom: 1rem;
    border: 1px solid var(--border);
  }

  .status-banner.error {
    background: color-mix(in oklab, #ef4444 10%, var(--card));
    color: #dc2626;
  }

  .status-banner.success {
    background: color-mix(in oklab, #22c55e 10%, var(--card));
    color: #16a34a;
  }

  .empty-state,
  .panel-status {
    padding: 1rem;
  }

  .context-overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
  }

  .context-menu {
    position: fixed;
    min-width: 9rem;
    padding: 0.35rem;
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    background: var(--popover, var(--card));
    box-shadow: 0 18px 45px rgba(15, 23, 42, 0.12);
  }

  .context-item {
    display: block;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--foreground);
    text-align: left;
    border-radius: 0.5rem;
    padding: 0.55rem 0.7rem;
    font-size: 0.82rem;
    cursor: pointer;
  }

  .context-item:hover {
    background: var(--muted);
  }

  .context-item.danger {
    color: #b91c1c;
  }

  @media (max-width: 1024px) {
    .assistant-settings-root {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 640px) {
    .param-grid-two,
    .header-actions {
      grid-template-columns: 1fr;
      flex-direction: column;
    }
  }
</style>
