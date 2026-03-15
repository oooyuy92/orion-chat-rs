import { writable } from 'svelte/store';
import type { Conversation } from '$lib/types';

export const titleUpdates = writable<{ id: string; title: string } | null>(null);
export const assistantUpdates = writable<{ id: string; assistantId: string | null } | null>(null);
export const conversationCreated = writable<Conversation | null>(null);
export const streamingConversations = writable<Set<string>>(new Set());
