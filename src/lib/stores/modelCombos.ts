import { loadStore } from '$lib/stores/kvStore';
import type { ModelCombo } from '$lib/types';

const STORE_NAME = 'model-combos.json';
const KEY = 'combos';

export async function loadCombos(): Promise<ModelCombo[]> {
  try {
    const store = await loadStore(STORE_NAME);
    return (await store.get<ModelCombo[]>(KEY)) ?? [];
  } catch {
    return [];
  }
}

export async function saveCombos(combos: ModelCombo[]): Promise<void> {
  const store = await loadStore(STORE_NAME);
  await store.set(KEY, combos);
  await store.save();
}

export async function addCombo(combo: ModelCombo): Promise<void> {
  const combos = await loadCombos();
  combos.push(combo);
  await saveCombos(combos);
}

export async function updateCombo(combo: ModelCombo): Promise<void> {
  const combos = await loadCombos();
  const idx = combos.findIndex((c) => c.id === combo.id);
  if (idx !== -1) {
    combos[idx] = combo;
    await saveCombos(combos);
  }
}

export async function deleteCombo(id: string): Promise<void> {
  const combos = await loadCombos();
  await saveCombos(combos.filter((c) => c.id !== id));
}
