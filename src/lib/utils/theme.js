export const DEFAULT_THEME = 'default';
const PREVIEW_BACKGROUND = '#f7f8fa';
const PREVIEW_ACCENT = '#f1f3f5';
const PREVIEW_BORDER = '#d8dde3';

export const THEME_OPTIONS = [
  {
    id: 'default',
    label: 'Default',
    labelZh: '默认',
    preview: {
      background: PREVIEW_BACKGROUND,
      primary: '#334155',
      accent: PREVIEW_ACCENT,
      border: PREVIEW_BORDER,
    },
  },
  {
    id: 'blue',
    label: 'Blue',
    labelZh: '蓝',
    preview: {
      background: PREVIEW_BACKGROUND,
      primary: '#3b82f6',
      accent: PREVIEW_ACCENT,
      border: PREVIEW_BORDER,
    },
  },
  {
    id: 'green',
    label: 'Green',
    labelZh: '绿',
    preview: {
      background: PREVIEW_BACKGROUND,
      primary: '#22c55e',
      accent: PREVIEW_ACCENT,
      border: PREVIEW_BORDER,
    },
  },
  {
    id: 'orange',
    label: 'Orange',
    labelZh: '橙',
    preview: {
      background: PREVIEW_BACKGROUND,
      primary: '#f97316',
      accent: PREVIEW_ACCENT,
      border: PREVIEW_BORDER,
    },
  },
  {
    id: 'red',
    label: 'Red',
    labelZh: '红',
    preview: {
      background: PREVIEW_BACKGROUND,
      primary: '#ef4444',
      accent: PREVIEW_ACCENT,
      border: PREVIEW_BORDER,
    },
  },
  {
    id: 'rose',
    label: 'Rose',
    labelZh: '玫瑰',
    preview: {
      background: PREVIEW_BACKGROUND,
      primary: '#f43f5e',
      accent: PREVIEW_ACCENT,
      border: PREVIEW_BORDER,
    },
  },
  {
    id: 'violet',
    label: 'Violet',
    labelZh: '紫罗兰',
    preview: {
      background: PREVIEW_BACKGROUND,
      primary: '#8b5cf6',
      accent: PREVIEW_ACCENT,
      border: PREVIEW_BORDER,
    },
  },
  {
    id: 'yellow',
    label: 'Yellow',
    labelZh: '黄',
    preview: {
      background: PREVIEW_BACKGROUND,
      primary: '#eab308',
      accent: PREVIEW_ACCENT,
      border: PREVIEW_BORDER,
    },
  },
];

const THEME_IDS = new Set(THEME_OPTIONS.map((theme) => theme.id));

/**
 * @param {string | null | undefined} theme
 */
export function normalizeTheme(theme) {
  return typeof theme === 'string' && THEME_IDS.has(theme) ? theme : DEFAULT_THEME;
}

/**
 * @param {string | null | undefined} theme
 * @param {{ dataset?: { theme?: string }, setAttribute?: (name: string, value: string) => void } | undefined} [root]
 */
export function applyTheme(
  theme,
  root = typeof document === 'undefined' ? undefined : document.documentElement,
) {
  const resolvedTheme = normalizeTheme(theme);
  if (!root) {
    return resolvedTheme;
  }

  if (root.dataset) {
    root.dataset.theme = resolvedTheme;
  } else if (typeof root.setAttribute === 'function') {
    root.setAttribute('data-theme', resolvedTheme);
  }

  return resolvedTheme;
}
