import { load as loadStore } from '@tauri-apps/plugin-store';
import type { Role } from '$lib/types';

export type Language = 'zh' | 'en';
export type ConversationGroupKey = 'today' | 'yesterday' | 'last7Days' | 'last30Days' | 'older';

const SETTINGS_STORE = 'settings.json';

const messages = {
  zh: {
    settingsPageTitle: '设置',
    selectConversationPrompt: '选择或创建一个对话以开始聊天',
    noMessagesYet: '还没有消息',
    askAnything: '发送一条消息，开始这段对话。',
    inputPlaceholder: '你想了解什么？',
    stop: '停止',
    send: '发送',
    messageGenerationFailed: '消息生成失败。',
    cancel: '取消',
    resend: '重新发送',
    edit: '修改',
    copy: '复制',
    copied: '已复制',
    delete: '删除',
    showReasoning: '思考过程',
    hideReasoning: '收起思考',
    regenerate: '重新生成',
    generateNewVersion: '生成新版本',
    selectModel: '选择模型',
    modelParameters: '模型参数',
    parameters: '参数',
    reset: '重置',
    resetToDefaults: '恢复默认参数',
    common: '通用',
    temperature: '温度',
    topP: 'Top P',
    maxTokens: '最大 Tokens',
    thinking: '思考',
    budgetTokens: '预算 Tokens',
    effort: '思考强度',
    default: '默认',
    low: '低',
    medium: '中',
    high: '高',
    disabled: '关闭',
    adaptive: '自适应',
    enabled: '开启',
    thinkingLevel: '思考等级',
    thinkingBudget: '思考预算',
    frequencyPenalty: '频率惩罚',
    presencePenalty: '存在惩罚',
    reasoningEffort: '推理强度',
    seed: '随机种子',
    maxCompletionTokens: '最大补全 Tokens',
    think: '思考模式',
    on: '开启',
    off: '关闭',
    numCtx: '上下文长度',
    repeatPenalty: '重复惩罚',
    minP: 'Min P',
    keepAlive: '保活时长',
    searchMessages: '搜索消息...',
    searching: '搜索中...',
    noResultsFound: '没有找到结果',
    newAssistant: '+ 新助手',
    loading: '加载中...',
    noAssistantsYet: '还没有助手',
    newChat: '+ 新对话',
    newChatTitle: '新对话',
    loadingConversations: '正在加载对话...',
    noConversationsYet: '还没有对话',
    pinned: '已固定',
    rename: '重命名',
    addPrefix: '添加前缀',
    pin: '固定',
    unpin: '取消固定',
    prefixPlaceholder: '前缀…',
    settings: '设置',
    close: '关闭',
    toggleSidebar: '切换侧边栏',
    sidebar: '侧边栏',
    mobileSidebarDescription: '显示移动端侧边栏。',
    conversationGroups: {
      today: '今天',
      yesterday: '昨天',
      last7Days: '最近 7 天',
      last30Days: '最近 30 天',
      older: '更早',
    },
    roles: {
      user: '用户',
      assistant: '助手',
      system: '系统',
    },
    relativeTime: {
      justNow: '刚刚',
      minutesAgo: (count: number) => `${count} 分钟前`,
      hoursAgo: (count: number) => `${count} 小时前`,
      daysAgo: (count: number) => `${count} 天前`,
    },
    pasteLabel: (count: number) => `[${count} 字符]`,
  },
  en: {
    settingsPageTitle: 'Settings',
    selectConversationPrompt: 'Select or create a conversation to start chatting',
    noMessagesYet: 'No messages yet',
    askAnything: 'Ask anything to start the conversation.',
    inputPlaceholder: 'What would you like to know?',
    stop: 'Stop',
    send: 'Send',
    messageGenerationFailed: 'Message generation failed.',
    cancel: 'Cancel',
    resend: 'Resend',
    edit: 'Edit',
    copy: 'Copy',
    copied: 'Copied',
    delete: 'Delete',
    showReasoning: 'Reasoning',
    hideReasoning: 'Hide reasoning',
    regenerate: 'Regenerate',
    generateNewVersion: 'Generate new version',
    selectModel: 'Select model',
    modelParameters: 'Model parameters',
    parameters: 'Parameters',
    reset: 'Reset',
    resetToDefaults: 'Reset to defaults',
    common: 'Common',
    temperature: 'Temperature',
    topP: 'Top P',
    maxTokens: 'Max Tokens',
    thinking: 'Thinking',
    budgetTokens: 'Budget Tokens',
    effort: 'Effort',
    default: 'Default',
    low: 'Low',
    medium: 'Medium',
    high: 'High',
    disabled: 'Disabled',
    adaptive: 'Adaptive',
    enabled: 'Enabled',
    thinkingLevel: 'Thinking Level',
    thinkingBudget: 'Thinking Budget',
    frequencyPenalty: 'Frequency Penalty',
    presencePenalty: 'Presence Penalty',
    reasoningEffort: 'Reasoning Effort',
    seed: 'Seed',
    maxCompletionTokens: 'Max Completion Tokens',
    think: 'Think',
    on: 'On',
    off: 'Off',
    numCtx: 'Num Ctx',
    repeatPenalty: 'Repeat Penalty',
    minP: 'Min P',
    keepAlive: 'Keep Alive',
    searchMessages: 'Search messages...',
    searching: 'Searching...',
    noResultsFound: 'No results found',
    newAssistant: '+ New Assistant',
    loading: 'Loading...',
    noAssistantsYet: 'No assistants yet',
    newChat: '+ New Chat',
    newChatTitle: 'New Chat',
    loadingConversations: 'Loading conversations...',
    noConversationsYet: 'No conversations yet',
    pinned: 'Pinned',
    rename: 'Rename',
    addPrefix: 'Add prefix',
    pin: 'Pin',
    unpin: 'Unpin',
    prefixPlaceholder: 'Prefix…',
    settings: 'Settings',
    close: 'Close',
    toggleSidebar: 'Toggle Sidebar',
    sidebar: 'Sidebar',
    mobileSidebarDescription: 'Displays the mobile sidebar.',
    conversationGroups: {
      today: 'Today',
      yesterday: 'Yesterday',
      last7Days: 'Last 7 days',
      last30Days: 'Last 30 days',
      older: 'Older',
    },
    roles: {
      user: 'User',
      assistant: 'Assistant',
      system: 'System',
    },
    relativeTime: {
      justNow: 'Just now',
      minutesAgo: (count: number) => `${count}m ago`,
      hoursAgo: (count: number) => `${count}h ago`,
      daysAgo: (count: number) => `${count}d ago`,
    },
    pasteLabel: (count: number) => `[${count} chars]`,
  },
} as const;

function isLanguage(value: unknown): value is Language {
  return value === 'zh' || value === 'en';
}

async function persistLanguage(nextLanguage: Language) {
  const store = await loadStore(SETTINGS_STORE);
  await store.set('language', nextLanguage);
  await store.save();
}

export const i18n = (() => {
  let language = $state<Language>('zh');
  let initialized = false;

  return {
    get language() {
      return language;
    },

    get t() {
      return messages[language];
    },

    async init() {
      if (initialized) return;
      initialized = true;
      try {
        const store = await loadStore(SETTINGS_STORE);
        const savedLanguage = await store.get<Language>('language');
        if (isLanguage(savedLanguage)) {
          language = savedLanguage;
        }
      } catch (error) {
        console.error('Failed to load language:', error);
      }
    },

    async setLanguage(nextLanguage: Language) {
      if (!isLanguage(nextLanguage)) return;
      language = nextLanguage;
      initialized = true;
      try {
        await persistLanguage(nextLanguage);
      } catch (error) {
        console.error('Failed to save language:', error);
      }
    },

    pasteLabel(count: number) {
      return messages[language].pasteLabel(count);
    },

    conversationGroupLabel(group: ConversationGroupKey) {
      return messages[language].conversationGroups[group];
    },

    roleLabel(role: Role) {
      return messages[language].roles[role];
    },

    formatRelativeTime(date: string) {
      const now = new Date();
      const target = new Date(date);
      const diffMs = now.getTime() - target.getTime();
      const diffMins = Math.floor(diffMs / 60000);
      const diffHours = Math.floor(diffMs / 3600000);
      const diffDays = Math.floor(diffMs / 86400000);
      const relative = messages[language].relativeTime;

      if (diffMins < 1) return relative.justNow;
      if (diffMins < 60) return relative.minutesAgo(diffMins);
      if (diffHours < 24) return relative.hoursAgo(diffHours);
      if (diffDays < 7) return relative.daysAgo(diffDays);
      return target.toLocaleDateString(language === 'zh' ? 'zh-CN' : 'en-US');
    },
  };
})();
