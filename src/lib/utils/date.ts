import type { Conversation } from '$lib/types';

export function groupConversationsByTime(conversations: Conversation[]) {
  const now = new Date();
  const today = startOfDay(now);
  const yesterday = startOfDay(subDays(now, 1));
  const last7Days = startOfDay(subDays(now, 7));
  const last30Days = startOfDay(subDays(now, 30));

  return {
    today: conversations.filter((c) => new Date(c.updatedAt) >= today),
    yesterday: conversations.filter((c) => {
      const date = new Date(c.updatedAt);
      return date >= yesterday && date < today;
    }),
    last7Days: conversations.filter((c) => {
      const date = new Date(c.updatedAt);
      return date >= last7Days && date < yesterday;
    }),
    last30Days: conversations.filter((c) => {
      const date = new Date(c.updatedAt);
      return date >= last30Days && date < last7Days;
    }),
    older: conversations.filter((c) => new Date(c.updatedAt) < last30Days),
  };
}

export function formatRelativeTime(date: string): string {
  const now = new Date();
  const target = new Date(date);
  const diffMs = now.getTime() - target.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return 'Just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return target.toLocaleDateString();
}

function startOfDay(date: Date): Date {
  const d = new Date(date);
  d.setHours(0, 0, 0, 0);
  return d;
}

function subDays(date: Date, days: number): Date {
  const d = new Date(date);
  d.setDate(d.getDate() - days);
  return d;
}
