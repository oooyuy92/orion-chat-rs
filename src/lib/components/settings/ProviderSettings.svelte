<script lang="ts">
  import { onMount } from 'svelte';
  import type { ProviderConfig, ProviderType } from '$lib/types';
  import { api } from '$lib/utils/invoke';
  import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '$lib/components/ui/card';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Separator } from '$lib/components/ui/separator';

  type SectionGroup = {
    title: string;
    items: string[];
  };

  const sectionGroups: SectionGroup[] = [
    { title: '模型服务', items: ['默认模型', '常规设置', '显示设置', '数据设置'] },
    { title: 'MCP 服务器', items: ['网络搜索', '全局记忆', 'API 服务器', '文档处理', '快捷短语', '快捷键'] },
    { title: '其他', items: ['快捷助手', '划词助手', '关于我们'] },
  ];

  const providerTypeOptions: { value: ProviderType; label: string; defaultBase: string }[] = [
    { value: 'openaiCompat', label: 'OpenAI Compatible', defaultBase: 'https://api.openai.com/v1' },
    { value: 'anthropic', label: 'Anthropic', defaultBase: 'https://api.anthropic.com' },
    { value: 'gemini', label: 'Gemini', defaultBase: 'https://generativelanguage.googleapis.com' },
    { value: 'ollama', label: 'Ollama', defaultBase: 'http://127.0.0.1:11434' },
  ];

  let providers = $state<ProviderConfig[]>([]);
  let loading = $state(true);
  let error = $state('');
  let success = $state('');

  let selectedProviderId = $state('');
  let search = $state('');

  let draftName = $state('');
  let draftType = $state<ProviderType>('openaiCompat');
  let draftApiBase = $state('');
  let draftApiKey = $state('');
  let draftEnabled = $state(true);

  let showApiKey = $state(false);
  let saving = $state(false);
  let deleting = $state(false);
  let updatingModels = $state<Record<string, boolean>>({});
  let bulkUpdatingModels = $state(false);

  let syncingModels = $state<Record<string, boolean>>({});

  let showAddForm = $state(false);
  let creating = $state(false);
  let newName = $state('');
  let newType = $state<ProviderType>('openaiCompat');
  let newApiBase = $state('https://api.openai.com/v1');
  let newApiKey = $state('');
  let newEnabled = $state(true);

  let selectedProvider = $derived.by(() =>
    providers.find((provider) => provider.id === selectedProviderId) ?? null,
  );

  let filteredProviders = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) {
      return providers;
    }
    return providers.filter((provider) => provider.name.toLowerCase().includes(q));
  });

  let isDirty = $derived.by(() => {
    if (!selectedProvider) {
      return false;
    }

    return (
      draftName.trim() !== selectedProvider.name ||
      draftType !== selectedProvider.providerType ||
      draftApiBase.trim() !== selectedProvider.apiBase ||
      draftApiKey !== (selectedProvider.apiKey ?? '') ||
      draftEnabled !== selectedProvider.enabled
    );
  });

  let endpointPreview = $derived.by(() => {
    const base = draftApiBase.trim().replace(/\/+$/, '');
    if (!base) {
      return '';
    }

    switch (draftType) {
      case 'anthropic':
        return `${base}/v1/messages`;
      case 'gemini':
        return `${base}/v1beta/models`;
      case 'ollama':
        return `${base}/api/tags`;
      default:
        return `${base}/chat/completions`;
    }
  });

  function providerTypeDefaultBase(type: ProviderType): string {
    return (
      providerTypeOptions.find((option) => option.value === type)?.defaultBase ??
      'https://api.openai.com/v1'
    );
  }

  function applyDraft(provider: ProviderConfig) {
    draftName = provider.name;
    draftType = provider.providerType;
    draftApiBase = provider.apiBase;
    draftApiKey = provider.apiKey ?? '';
    draftEnabled = provider.enabled;
    showApiKey = false;
  }

  function selectProvider(id: string) {
    selectedProviderId = id;
    const provider = providers.find((item) => item.id === id);
    if (provider) {
      applyDraft(provider);
    }
  }

  function normalizeProviders(items: ProviderConfig[]): ProviderConfig[] {
    return [...items].sort((a, b) => a.name.localeCompare(b.name, 'zh-CN'));
  }

  async function loadProviders() {
    loading = true;
    error = '';
    try {
      providers = normalizeProviders(await api.listProviders());

      if (providers.length > 0) {
        const initial = providers.some((provider) => provider.id === selectedProviderId)
          ? selectedProviderId
          : providers[0].id;
        selectProvider(initial);
      } else {
        selectedProviderId = '';
      }
    } catch (e) {
      console.error('Failed to load providers:', e);
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleSyncModels(providerId = selectedProviderId) {
    if (!providerId) {
      return;
    }

    syncingModels = { ...syncingModels, [providerId]: true };
    success = '';
    error = '';

    try {
      const models = await api.fetchModels(providerId);
      providers = providers.map((provider) =>
        provider.id === providerId ? { ...provider, models } : provider,
      );
      success = `模型同步成功，共 ${models.length} 个模型。`;
    } catch (e) {
      console.error('Failed to fetch models:', e);
      error = `模型检测失败：${e}`;
    } finally {
      syncingModels = { ...syncingModels, [providerId]: false };
    }
  }

  async function handleToggleModelVisibility(modelId: string, enabled: boolean) {
    if (!selectedProvider || !draftEnabled) {
      return;
    }

    updatingModels = { ...updatingModels, [modelId]: true };
    error = '';
    success = '';

    try {
      await api.updateModelVisibility(modelId, enabled);
      providers = providers.map((provider) =>
        provider.id === selectedProvider.id
          ? {
              ...provider,
              models: provider.models.map((model) =>
                model.id === modelId ? { ...model, enabled } : model,
              ),
            }
          : provider,
      );
    } catch (e) {
      console.error('Failed to update model visibility:', e);
      error = `更新模型显示状态失败：${e}`;
    } finally {
      updatingModels = { ...updatingModels, [modelId]: false };
    }
  }

  async function handleBatchModelVisibility(enabled: boolean) {
    if (!selectedProvider || !draftEnabled || selectedProvider.models.length === 0) {
      return;
    }

    bulkUpdatingModels = true;
    error = '';
    success = '';

    try {
      await api.updateProviderModelsVisibility(selectedProvider.id, enabled);
      providers = providers.map((provider) =>
        provider.id === selectedProvider.id
          ? {
              ...provider,
              models: provider.models.map((model) => ({ ...model, enabled })),
            }
          : provider,
      );
      success = enabled ? '已勾选该服务下全部模型。' : '已取消该服务下全部模型。';
    } catch (e) {
      console.error('Failed to batch update model visibility:', e);
      error = `批量更新模型显示状态失败：${e}`;
    } finally {
      bulkUpdatingModels = false;
    }
  }

  async function handleSaveSelected() {
    if (!selectedProvider) {
      return;
    }

    const nextName = draftName.trim();
    const nextBase = draftApiBase.trim();

    if (!nextName || !nextBase) {
      error = '服务名称和 API 地址不能为空。';
      return;
    }

    saving = true;
    error = '';
    success = '';

    try {
      const updated = await api.updateProvider(
        selectedProvider.id,
        nextName,
        draftType,
        nextBase,
        draftApiKey.trim() || null,
        draftEnabled,
      );

      providers = normalizeProviders(
        providers.map((provider) => (provider.id === updated.id ? updated : provider)),
      );
      selectProvider(updated.id);
      success = '服务配置已保存。';
    } catch (e) {
      console.error('Failed to update provider:', e);
      error = `保存失败：${e}`;
    } finally {
      saving = false;
    }
  }

  async function handleDeleteSelected() {
    if (!selectedProvider) {
      return;
    }

    const confirmed = window.confirm(`确认删除服务 “${selectedProvider.name}” 吗？`);
    if (!confirmed) {
      return;
    }

    deleting = true;
    error = '';
    success = '';

    try {
      const deletingId = selectedProvider.id;
      await api.deleteProvider(deletingId);
      providers = providers.filter((provider) => provider.id !== deletingId);

      if (providers.length > 0) {
        selectProvider(providers[0].id);
      } else {
        selectedProviderId = '';
        draftName = '';
        draftApiBase = '';
        draftApiKey = '';
      }

      success = '服务已删除。';
    } catch (e) {
      console.error('Failed to delete provider:', e);
      error = `删除失败：${e}`;
    } finally {
      deleting = false;
    }
  }

  async function handleCreateProvider() {
    const name = newName.trim();
    const base = newApiBase.trim();

    if (!name || !base) {
      error = '请先填写服务名称和 API 地址。';
      return;
    }

    creating = true;
    error = '';
    success = '';

    try {
      const provider = await api.addProvider(
        name,
        newType,
        base,
        newApiKey.trim() || undefined,
        newEnabled,
      );

      providers = normalizeProviders([...providers, provider]);
      selectProvider(provider.id);

      showAddForm = false;
      newName = '';
      newType = 'openaiCompat';
      newApiBase = providerTypeDefaultBase('openaiCompat');
      newApiKey = '';
      newEnabled = true;
      success = '服务已添加。';
    } catch (e) {
      console.error('Failed to create provider:', e);
      error = `添加失败：${e}`;
    } finally {
      creating = false;
    }
  }

  function handleDraftTypeChange(next: ProviderType) {
    draftType = next;
    if (!draftApiBase.trim()) {
      draftApiBase = providerTypeDefaultBase(next);
    }
  }

  function handleNewTypeChange(next: ProviderType) {
    newType = next;
    if (!newApiBase.trim()) {
      newApiBase = providerTypeDefaultBase(next);
    }
  }

  onMount(() => {
    loadProviders();
  });
</script>

<div class="settings-root">
  <aside class="settings-nav">
    {#each sectionGroups as group}
      <section class="nav-group">
        <h3>{group.title}</h3>
        {#each group.items as item}
          <button class="nav-item" class:is-active={item === 'API 服务器'}>{item}</button>
        {/each}
      </section>
    {/each}
  </aside>

  <section class="provider-list-panel">
    <div class="panel-head">
      <h2>模型服务</h2>
      <Input
        type={'search'}
        bind:value={search}
        placeholder="搜索模型平台..."
      />
    </div>

    <div class="provider-list-scroll">
      {#if loading}
        <p class="panel-status">正在加载服务...</p>
      {:else if filteredProviders.length === 0}
        <p class="panel-status">未找到匹配服务</p>
      {:else}
        {#each filteredProviders as provider (provider.id)}
          <Card
            class="provider-card {provider.id === selectedProviderId ? 'is-active' : ''}"
            onclick={() => selectProvider(provider.id)}
          >
            <CardContent class="provider-card-content">
              <span class="provider-name">{provider.name}</span>
              <span class="provider-state" class:enabled={provider.enabled}>
                {provider.enabled ? 'ON' : 'OFF'}
              </span>
            </CardContent>
          </Card>
        {/each}
      {/if}
    </div>

    <div class="add-provider">
      <Button
        variant={'outline'}
        class="w-full"
        onclick={() => (showAddForm = !showAddForm)}
      >
        {showAddForm ? '收起' : '+ 添加'}
      </Button>

      {#if showAddForm}
        <div class="add-form">
          <label>
            服务名称
            <Input bind:value={newName} placeholder="例如：OpenAI 官方" />
          </label>

          <label>
            提供商类型
            <select
              class="form-input"
              value={newType}
              onchange={(event) => handleNewTypeChange((event.target as HTMLSelectElement).value as ProviderType)}
            >
              {#each providerTypeOptions as option (option.value)}
                <option value={option.value}>{option.label}</option>
              {/each}
            </select>
          </label>

          <label>
            API 地址
            <Input bind:value={newApiBase} placeholder="https://api.example.com/v1" />
          </label>

          <label>
            API 密钥
            <Input bind:value={newApiKey} type={'password'} placeholder="可选（Ollama 可留空）" />
          </label>

          <label class="switch-row">
            <input type="checkbox" checked={newEnabled} onchange={(e) => newEnabled = (e.target as HTMLInputElement).checked} />
            <span>创建后立即启用</span>
          </label>

          <Button onclick={handleCreateProvider} disabled={creating} class="w-full">
            {creating ? '添加中...' : '确认添加'}
          </Button>
        </div>
      {/if}
    </div>
  </section>

  <section class=”provider-detail-panel”>
    {#if error}
      <div class:alert={true} class:error={true}>{error}</div>
    {/if}
    {#if success}
      <div class:alert={true} class:success={true}>{success}</div>
    {/if}

    {#if selectedProvider}
      <div class=”detail-head”>
        <h2>{selectedProvider.name}</h2>
        <label class=”switch-row”>
          <input type=”checkbox” checked={draftEnabled} onchange={(e) => draftEnabled = (e.target as HTMLInputElement).checked} />
          <span>{draftEnabled ? '已启用' : '已停用'}</span>
        </label>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>基本配置</CardTitle>
          <CardDescription>配置服务的基本信息和认证</CardDescription>
        </CardHeader>
        <CardContent>
          <div class=”detail-grid”>
            <label>
              提供商类型
              <select
                class=”form-input”
                value={draftType}
                onchange={(event) => handleDraftTypeChange((event.target as HTMLSelectElement).value as ProviderType)}
              >
                {#each providerTypeOptions as option (option.value)}
                  <option value={option.value}>{option.label}</option>
                {/each}
              </select>
            </label>

            <label>
              服务名称
              <Input bind:value={draftName} placeholder=”服务名称” />
            </label>

            <label class=”full-width”>
              API 密钥
              <div class=”key-input-wrap”>
                <Input
                  bind:value={draftApiKey}
                  type={showApiKey ? 'text' : 'password'}
                  placeholder={'输入 API 密钥'}
                />
                <Button
                  variant={'outline'}
                  size={'sm'}
                  onclick={() => (showApiKey = !showApiKey)}
                >
                  {showApiKey ? '隐藏' : '显示'}
                </Button>
                <Button
                  variant={'outline'}
                  size={'sm'}
                  disabled={syncingModels[selectedProvider.id] || !draftEnabled}
                  onclick={() => handleSyncModels(selectedProvider.id)}
                >
                  {syncingModels[selectedProvider.id] ? '检测中...' : '检测'}
                </Button>
              </div>
              <small>多个密钥可用英文逗号分隔。</small>
            </label>

            <label class=”full-width”>
              API 地址
              <Input bind:value={draftApiBase} placeholder=”https://api.example.com/v1” />
              {#if endpointPreview}
                <small>预览接口: {endpointPreview}</small>
              {/if}
            </label>
          </div>
        </CardContent>
      </Card>

      <Card class=”mt-4”>
        <CardHeader>
          <div class=”models-head”>
            <div>
              <CardTitle>模型可见性</CardTitle>
              <CardDescription>管理该服务下的模型显示状态</CardDescription>
            </div>
            <div class=”models-actions”>
              <Button
                variant={'outline'}
                size={'sm'}
                onclick={() => handleBatchModelVisibility(true)}
                disabled={!draftEnabled || bulkUpdatingModels || selectedProvider.models.length === 0}
              >
                全选
              </Button>
              <Button
                variant={'outline'}
                size={'sm'}
                onclick={() => handleBatchModelVisibility(false)}
                disabled={!draftEnabled || bulkUpdatingModels || selectedProvider.models.length === 0}
              >
                全不选
              </Button>
              <Button
                variant={'outline'}
                size={'sm'}
                onclick={() => handleSyncModels(selectedProvider.id)}
                disabled={syncingModels[selectedProvider.id] || !draftEnabled}
              >
                {syncingModels[selectedProvider.id] ? '同步中...' : '同步模型'}
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {#if selectedProvider.models.length > 0}
            <ul class=”model-list”>
              {#each selectedProvider.models as model (model.id)}
                <li class=”model-item”>
                  <label class=”model-toggle”>
                    <input
                      type=”checkbox”
                      checked={model.enabled}
                      disabled={!draftEnabled || updatingModels[model.id] || bulkUpdatingModels}
                      onchange={(event) =>
                        handleToggleModelVisibility(
                          model.id,
                          (event.target as HTMLInputElement).checked,
                        )}
                    />
                    <span class:model-muted={!model.enabled}>{model.name || model.id}</span>
                  </label>
                </li>
              {/each}
            </ul>
          {:else}
            <p class=”panel-status”>暂无模型，点击”同步模型”拉取。</p>
          {/if}

          {#if !draftEnabled}
            <p class:panel-status={true} class:mt-1={true}>当前服务已停用，模型勾选状态会保留但不会在聊天中显示。</p>
          {/if}
        </CardContent>
      </Card>

      <div class=”detail-actions”>
        <Button disabled={!isDirty || saving} onclick={handleSaveSelected}>
          {saving ? '保存中...' : '保存'}
        </Button>
        <Button variant={'destructive'} disabled={deleting} onclick={handleDeleteSelected}>
          {deleting ? '删除中...' : '删除'}
        </Button>
      </div>
    {:else}
      <Card>
        <CardContent class=”empty-state”>
          <h3>请选择一个服务</h3>
          <p>你可以在中间列表中选择，或先点击 “+ 添加” 创建新服务。</p>
        </CardContent>
      </Card>
    {/if}
  </section>
</div>

<style>
  .settings-root {
    height: 100%;
    min-height: 0;
    display: grid;
    grid-template-columns: 12.5rem 16rem minmax(0, 1fr);
    border-top: 1px solid var(--border);
    background: var(--background);
  }

  .settings-nav,
  .provider-list-panel,
  .provider-detail-panel {
    min-height: 0;
  }

  .settings-nav {
    border-right: 1px solid var(--border);
    background: var(--sidebar-bg);
    padding: 0.9rem 0.65rem;
    overflow-y: auto;
  }

  .nav-group {
    padding-bottom: 0.75rem;
    margin-bottom: 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .nav-group:last-child {
    margin-bottom: 0;
    border-bottom: none;
  }

  .nav-group h3 {
    margin: 0 0 0.45rem;
    font-size: 0.82rem;
    color: var(--muted-foreground);
    font-weight: 600;
  }

  .nav-item {
    width: 100%;
    text-align: left;
    background: transparent;
    color: var(--foreground);
    border: none;
    border-radius: 0.55rem;
    padding: 0.4rem 0.55rem;
    font-size: 0.82rem;
    cursor: pointer;
  }

  .nav-item:hover {
    background: var(--sidebar-hover);
  }

  .nav-item.is-active {
    background: var(--sidebar-active);
    font-weight: 600;
  }

  .provider-list-panel {
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    background: var(--card);
  }

  .panel-head {
    padding: 0.9rem 0.75rem 0.7rem;
    border-bottom: 1px solid var(--border);
  }

  .panel-head h2 {
    margin: 0 0 0.55rem;
    font-size: 0.9rem;
    font-weight: 650;
  }

  .form-input {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 0.6rem;
    background: var(--background);
    color: var(--foreground);
    padding: 0.5rem 0.6rem;
    font-size: 0.84rem;
    box-sizing: border-box;
    outline: none;
  }

  .form-input:focus {
    border-color: #c4c4cc;
  }

  .provider-list-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  :global(.provider-card) {
    cursor: pointer;
    transition: all 0.15s ease;
    border: 1px solid transparent;
  }

  :global(.provider-card:hover) {
    background: var(--muted);
  }

  :global(.provider-card.is-active) {
    background: var(--muted);
    border-color: var(--border);
  }

  :global(.provider-card-content) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
    padding: 0.48rem 0.52rem !important;
  }

  .provider-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.84rem;
    text-align: left;
  }

  .provider-state {
    font-size: 0.68rem;
    border-radius: 999px;
    padding: 0.08rem 0.34rem;
    color: #9f1d1d;
    background: #fee2e2;
    border: 1px solid #fecaca;
  }

  .provider-state.enabled {
    color: #166534;
    background: #dcfce7;
    border: 1px solid #bbf7d0;
  }

  .add-provider {
    border-top: 1px solid var(--border);
    padding: 0.6rem;
  }

  :global(.w-full) {
    width: 100%;
  }

  .add-form {
    margin-top: 0.55rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .add-form label,
  .detail-grid label {
    display: flex;
    flex-direction: column;
    gap: 0.28rem;
    font-size: 0.78rem;
    color: var(--muted-foreground);
  }

  .provider-detail-panel {
    padding: 0.95rem;
    overflow-y: auto;
  }

  .detail-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
    margin-bottom: 0.7rem;
    padding-bottom: 0.7rem;
    border-bottom: 1px solid var(--border);
  }

  .detail-head h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
  }

  .switch-row {
    display: inline-flex;
    align-items: center;
    gap: 0.36rem;
    font-size: 0.78rem;
    color: var(--muted-foreground);
  }

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 0.6rem;
  }

  .detail-grid .full-width {
    grid-column: 1 / -1;
  }

  .key-input-wrap {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 0.35rem;
    align-items: center;
  }

  small {
    color: var(--muted-foreground);
    font-size: 0.72rem;
  }

  :global(.mt-4) {
    margin-top: 1rem;
  }

  .models-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .models-actions {
    display: inline-flex;
    gap: 0.35rem;
    align-items: center;
    flex-shrink: 0;
  }

  .model-list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
    gap: 0.36rem;
  }

  .model-item {
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 0.34rem 0.5rem;
    font-size: 0.78rem;
    color: var(--foreground);
    background: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .model-toggle {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    width: 100%;
    min-width: 0;
    cursor: pointer;
  }

  .model-toggle input {
    margin: 0;
    cursor: pointer;
  }

  .model-toggle span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .model-muted {
    color: var(--muted-foreground);
    text-decoration: line-through;
  }

  .mt-1 {
    margin-top: 0.35rem;
  }

  .detail-actions {
    margin-top: 0.9rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .panel-status {
    margin: 0;
    font-size: 0.8rem;
    color: var(--muted-foreground);
  }

  .alert {
    border: 1px solid var(--border);
    border-radius: 0.6rem;
    padding: 0.5rem 0.62rem;
    margin-bottom: 0.7rem;
    font-size: 0.8rem;
  }

  .alert.error {
    border-color: #fecaca;
    background: #fef2f2;
    color: #991b1b;
  }

  .alert.success {
    border-color: #bbf7d0;
    background: #f0fdf4;
    color: #166534;
  }

  :global(.empty-state) {
    padding: 2rem 1rem !important;
    text-align: center;
  }

  .empty-state h3 {
    margin: 0;
    font-size: 0.9rem;
  }

  .empty-state p {
    margin: 0.45rem 0 0;
    font-size: 0.8rem;
    color: var(--muted-foreground);
  }

  @media (max-width: 1160px) {
    .settings-root {
      grid-template-columns: 11.5rem 12rem minmax(0, 1fr);
    }
  }

  @media (max-width: 980px) {
    .settings-root {
      grid-template-columns: 1fr;
    }

    .settings-nav,
    .provider-list-panel {
      border-right: none;
      border-bottom: 1px solid var(--border);
    }

    .settings-nav {
      max-height: 15rem;
    }

    .provider-list-panel {
      max-height: 20rem;
    }

    .detail-grid {
      grid-template-columns: 1fr;
    }

    .key-input-wrap {
      grid-template-columns: 1fr;
    }
  }
</style>
