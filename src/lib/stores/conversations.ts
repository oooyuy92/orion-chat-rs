import { writable } from 'svelte/store';

export const titleUpdates = writable<{ id: string; title: string } | null>(null);
export const assistantUpdates = writable<{ id: string; assistantId: string | null } | null>(null);
