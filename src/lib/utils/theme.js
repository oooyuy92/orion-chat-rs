export const DEFAULT_THEME = 'default';

export const THEME_OPTIONS = [
  {
    id: 'default',
    label: 'Default',
    labelZh: '默认',
    preview: {
      background: 'oklch(0.986 0.003 248)',
      primary: 'oklch(0.26 0.014 248)',
      accent: 'oklch(0.96 0.008 248)',
      border: 'oklch(0.91 0.006 248)',
    },
  },
  {
    id: 'blue',
    label: 'Blue',
    labelZh: '蓝',
    preview: {
      background: 'oklch(0.985 0.008 255)',
      primary: 'oklch(0.6 0.19 258)',
      accent: 'oklch(0.94 0.03 255)',
      border: 'oklch(0.9 0.018 255)',
    },
  },
  {
    id: 'green',
    label: 'Green',
    labelZh: '绿',
    preview: {
      background: 'oklch(0.985 0.01 150)',
      primary: 'oklch(0.64 0.17 150)',
      accent: 'oklch(0.94 0.03 150)',
      border: 'oklch(0.9 0.018 150)',
    },
  },
  {
    id: 'orange',
    label: 'Orange',
    labelZh: '橙',
    preview: {
      background: 'oklch(0.987 0.01 65)',
      primary: 'oklch(0.7 0.18 60)',
      accent: 'oklch(0.95 0.03 70)',
      border: 'oklch(0.91 0.02 70)',
    },
  },
  {
    id: 'red',
    label: 'Red',
    labelZh: '红',
    preview: {
      background: 'oklch(0.986 0.009 20)',
      primary: 'oklch(0.62 0.2 25)',
      accent: 'oklch(0.95 0.028 20)',
      border: 'oklch(0.91 0.018 20)',
    },
  },
  {
    id: 'rose',
    label: 'Rose',
    labelZh: '玫瑰',
    preview: {
      background: 'oklch(0.987 0.01 8)',
      primary: 'oklch(0.66 0.18 12)',
      accent: 'oklch(0.955 0.03 8)',
      border: 'oklch(0.915 0.018 8)',
    },
  },
  {
    id: 'violet',
    label: 'Violet',
    labelZh: '紫罗兰',
    preview: {
      background: 'oklch(0.986 0.01 305)',
      primary: 'oklch(0.62 0.2 305)',
      accent: 'oklch(0.95 0.03 305)',
      border: 'oklch(0.91 0.018 305)',
    },
  },
  {
    id: 'yellow',
    label: 'Yellow',
    labelZh: '黄',
    preview: {
      background: 'oklch(0.988 0.012 100)',
      primary: 'oklch(0.76 0.17 98)',
      accent: 'oklch(0.958 0.03 98)',
      border: 'oklch(0.918 0.02 98)',
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
