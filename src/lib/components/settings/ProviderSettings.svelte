<script lang="ts">
  import { onMount } from 'svelte';
  import type { ProviderConfig, ProviderType, ModelInfo } from '$lib/types';
  import { api } from '$lib/utils/invoke';

  let providers = $state<ProviderConfig[]>([]);
  let loading = $state(true);
  let error = $state('');

  // Add provider form state
  let formName = $state('');
  let formType = $state<ProviderType>('openaiCompat');
  let formApiBase = $state('');
  let formApiKey = $state('');
  let formSubmitting = $state(false);

  // Track which providers are fetching models
  let fetchingModels = $state<Record<string, boolean>>({});

  const providerTypeOptions: { value: ProviderType; label: string }[] = [
    { value: 'openaiCompat', label: 'OpenAI Compatible' },
    { value: 'anthropic', label: 'Anthropic' },
    { value: 'gemini', label: 'Gemini' },
    { value: 'ollama', label: 'Ollama' },
  ];

  function badgeColor(type: ProviderType): string {
    switch (type) {
      case 'openaiCompat': return '#10a37f';
      case 'anthropic': return '#d97706';
      case 'gemini': return '#4285f4';
      case 'ollama': return '#888';
    }
  }

  async function loadProviders() {
    try {
      providers = await api.listProviders();
    } catch (e) {
      console.error('Failed to load providers:', e);
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleAddProvider() {
    if (!formName.trim() || !formApiBase.trim()) return;
    formSubmitting = true;
    error = '';
    try {
      const provider = await api.addProvider(
        formName.trim(),
        formType,
        formApiBase.trim(),
        formApiKey.trim() || undefined,
      );
      providers = [...providers, provider];
      formName = '';
      formApiBase = '';
      formApiKey = '';
      formType = 'openaiCompat';
    } catch (e) {
      console.error('Failed to add provider:', e);
      error = String(e);
    } finally {
      formSubmitting = false;
    }
  }

  async function handleFetchModels(provider: ProviderConfig) {
    fetchingModels = { ...fetchingModels, [provider.id]: true };
    try {
      const models = await api.fetchModels(provider.id);
      providers = providers.map((p) =>
        p.id === provider.id ? { ...p, models } : p,
      );
    } catch (e) {
      console.error(`Failed to fetch models for ${provider.name}:`, e);
      error = `Failed to fetch models: ${e}`;
    } finally {
      fetchingModels = { ...fetchingModels, [provider.id]: false };
    }
  }

  onMount(() => {
    loadProviders();
  });
</script>

<div class="provider-settings">
  <h2 class="section-title">Providers</h2>

  {#if error}
    <div class="error-banner">{error}</div>
  {/if}

  <!-- Provider List -->
  <div class="provider-list">
    {#if loading}
      <p class="status-text">Loading providers...</p>
    {:else if providers.length === 0}
      <p class="status-text">No providers configured yet.</p>
    {:else}
      {#each providers as provider (provider.id)}
        <div class="provider-card">
          <div class="provider-header">
            <div class="provider-info">
              <span class="provider-name">{provider.name}</span>
              <span class="type-badge" style="background-color: {badgeColor(provider.providerType)};">
                {providerTypeOptions.find((o) => o.value === provider.providerType)?.label ?? provider.providerType}
              </span>
            </div>
            <button
              class="fetch-btn"
              disabled={fetchingModels[provider.id]}
              onclick={() => handleFetchModels(provider)}
            >
              {fetchingModels[provider.id] ? 'Fetching...' : 'Fetch Models'}
            </button>
          </div>
          <div class="provider-meta">
            <span class="meta-item">{provider.apiBase}</span>
            <span class="meta-item">{provider.models.length} model{provider.models.length !== 1 ? 's' : ''}</span>
          </div>
          {#if provider.models.length > 0}
            <ul class="model-list">
              {#each provider.models as model (model.id)}
                <li class="model-item">{model.name || model.id}</li>
              {/each}
            </ul>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  <!-- Add Provider Form -->
  <div class="add-form">
    <h3 class="form-title">Add Provider</h3>
    <form onsubmit={(e) => { e.preventDefault(); handleAddProvider(); }}>
      <label class="field-label">
        Name
        <input class="input-field" type="text" bind:value={formName} placeholder="My Provider" required />
      </label>
      <label class="field-label">
        Type
        <select class="input-field" bind:value={formType}>
          {#each providerTypeOptions as opt (opt.value)}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      </label>
      <label class="field-label">
        API Base URL
        <input class="input-field" type="text" bind:value={formApiBase} placeholder="https://api.example.com/v1" required />
      </label>
      <label class="field-label">
        API Key
        <input class="input-field" type="password" bind:value={formApiKey} placeholder="Optional" />
      </label>
      <button class="submit-btn" type="submit" disabled={formSubmitting || !formName.trim() || !formApiBase.trim()}>
        {formSubmitting ? 'Adding...' : 'Add Provider'}
      </button>
    </form>
  </div>
</div>

<style>
  .provider-settings {
    padding: 1.5rem;
    max-width: 640px;
    color: var(--text-primary);
  }

  .section-title {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0 0 1rem;
  }

  .error-banner {
    padding: 0.5rem 0.75rem;
    margin-bottom: 1rem;
    border-radius: 0.5rem;
    font-size: 0.8125rem;
    background-color: rgba(239, 68, 68, 0.15);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  .status-text {
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .provider-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-bottom: 1.5rem;
  }

  .provider-card {
    padding: 0.75rem 1rem;
    border-radius: 0.5rem;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border);
  }

  .provider-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .provider-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }

  .provider-name {
    font-weight: 500;
    font-size: 0.9375rem;
  }

  .type-badge {
    font-size: 0.6875rem;
    padding: 0.125rem 0.5rem;
    border-radius: 9999px;
    color: #fff;
    white-space: nowrap;
  }

  .fetch-btn {
    font-size: 0.75rem;
    padding: 0.25rem 0.625rem;
    border-radius: 0.375rem;
    border: 1px solid var(--border);
    background-color: transparent;
    color: var(--accent);
    cursor: pointer;
    white-space: nowrap;
    transition: background-color 0.15s;
  }

  .fetch-btn:hover:not(:disabled) {
    background-color: var(--bg-primary);
  }

  .fetch-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .provider-meta {
    display: flex;
    gap: 1rem;
    margin-top: 0.375rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .model-list {
    margin: 0.5rem 0 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .model-item {
    font-size: 0.6875rem;
    padding: 0.125rem 0.5rem;
    border-radius: 0.25rem;
    background-color: var(--bg-primary);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .add-form {
    padding: 1rem;
    border-radius: 0.5rem;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border);
  }

  .form-title {
    font-size: 1rem;
    font-weight: 600;
    margin: 0 0 0.75rem;
  }

  .field-label {
    display: block;
    font-size: 0.8125rem;
    font-weight: 500;
    margin-bottom: 0.75rem;
    color: var(--text-secondary);
  }

  .input-field {
    display: block;
    width: 100%;
    margin-top: 0.25rem;
    padding: 0.5rem 0.625rem;
    font-size: 0.875rem;
    border-radius: 0.375rem;
    border: 1px solid var(--border);
    background-color: var(--bg-primary);
    color: var(--text-primary);
    box-sizing: border-box;
  }

  .input-field:focus {
    outline: none;
    border-color: var(--accent);
  }

  .submit-btn {
    margin-top: 0.25rem;
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    font-weight: 500;
    border-radius: 0.5rem;
    border: none;
    background-color: var(--accent);
    color: #fff;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .submit-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .submit-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
