import { writable } from 'svelte/store';

export const titleUpdates = writable<{ id: string; title: string } | null>(null);
