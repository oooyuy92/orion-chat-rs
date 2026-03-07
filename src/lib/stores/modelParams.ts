import { load as loadStore } from '@tauri-apps/plugin-store';
import type { CommonParams, ProviderParams, ModelParams, ProviderType } from '$lib/types';

const STORE_NAME = 'model-params.json';

function defaultProviderParams(providerType: ProviderType): ProviderParams {
  switch (providerType) {
    case 'anthropic':
      return { provider_type: 'anthropic' };
    case 'gemini':
      return { provider_type: 'gemini' };
    case 'ollama':
      return { provider_type: 'ollama' };
    default:
      return { provider_type: 'openaiCompat' };
  }
}

function providerTypeToTag(pt: ProviderType): ProviderParams['provider_type'] {
  switch (pt) {
    case 'anthropic': return 'anthropic';
    case 'gemini': return 'gemini';
    case 'ollama': return 'ollama';
    default: return 'openaiCompat';
  }
}

export async function getModelParams(modelId: string, providerType: ProviderType): Promise<ModelParams> {
  try {
    const store = await loadStore(STORE_NAME);
    const saved = await store.get<ModelParams>(modelId);
    if (saved) {
      // Ensure provider_type tag matches (in case provider changed)
      const expectedTag = providerTypeToTag(providerType);
      if (saved.providerParams?.provider_type !== expectedTag) {
        saved.providerParams = defaultProviderParams(providerType);
      }
      return saved;
    }
  } catch {
    // store doesn't exist yet or read error
  }
  return {
    common: {},
    providerParams: defaultProviderParams(providerType),
  };
}

export async function setModelParams(modelId: string, params: ModelParams): Promise<void> {
  const store = await loadStore(STORE_NAME);
  await store.set(modelId, params);
  await store.save();
}

export async function deleteModelParams(modelId: string): Promise<void> {
  const store = await loadStore(STORE_NAME);
  await store.delete(modelId);
  await store.save();
}
