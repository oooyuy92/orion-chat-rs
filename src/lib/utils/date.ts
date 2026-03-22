import type { Conversation } from '$lib/types';
import type { Language } from '$lib/stores/i18n.svelte';

/**
 * 解析 SQLite 时间格式 "YYYY-MM-DD HH:MM:SS" 为 JavaScript Date 对象
 * SQLite 存储的时间格式需要添加 'T' 才能被 JavaScript 正确解析
 */
function parseSQLiteDate(dateStr: string): Date {
  // SQLite 格式: "2026-03-22 07:42:37"
  // 需要转换为 ISO 8601 格式: "2026-03-22T07:42:37"
  return new Date(dateStr.replace(' ', 'T'));
}

export function groupConversationsByTime(conversations: Conversation[]) {
  const now = new Date();
  const today = startOfDay(now);
  const yesterday = startOfDay(subDays(now, 1));
  const last7Days = startOfDay(subDays(now, 7));
  const last30Days = startOfDay(subDays(now, 30));

  return {
    today: conversations.filter((c) => parseSQLiteDate(c.updatedAt) >= today),
    yesterday: conversations.filter((c) => {
      const date = parseSQLiteDate(c.updatedAt);
      return date >= yesterday && date < today;
    }),
    last7Days: conversations.filter((c) => {
      const date = parseSQLiteDate(c.updatedAt);
      return date >= last7Days && date < yesterday;
    }),
    last30Days: conversations.filter((c) => {
      const date = parseSQLiteDate(c.updatedAt);
      return date >= last30Days && date < last7Days;
    }),
    older: conversations.filter((c) => parseSQLiteDate(c.updatedAt) < last30Days),
  };
}

export function formatRelativeTime(date: string, language: Language = 'en'): string {
  const now = new Date();
  const target = new Date(date.replace(' ', 'T')); // 修复 SQLite 时间格式
  const diffMs = now.getTime() - target.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (language === 'zh') {
    if (diffMins < 1) return '刚刚';
    if (diffMins < 60) return `${diffMins} 分钟前`;
    if (diffHours < 24) return `${diffHours} 小时前`;
    if (diffDays < 7) return `${diffDays} 天前`;
    return target.toLocaleDateString('zh-CN');
  }

  if (diffMins < 1) return 'Just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return target.toLocaleDateString('en-US');
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
