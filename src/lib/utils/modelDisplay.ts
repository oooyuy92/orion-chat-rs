import type { ModelInfo } from '$lib/types';

type ModelLike = Pick<ModelInfo, 'id' | 'name' | 'requestName' | 'displayName' | 'source'>;

function normalize(value: string | null | undefined): string {
  return value?.trim() ?? '';
}

export function resolveModelLabel(model: ModelLike): string {
  return normalize(model.displayName) || normalize(model.requestName) || normalize(model.name) || model.id;
}

export function resolveModelSecondaryLabel(model: ModelLike): string {
  const requestName = normalize(model.requestName) || normalize(model.name);
  if (!requestName || requestName === resolveModelLabel(model)) {
    return '';
  }
  return requestName;
}

export function isManualModel(model: ModelLike): boolean {
  return model.source === 'manual';
}
