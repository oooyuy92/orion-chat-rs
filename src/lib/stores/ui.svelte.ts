type Theme = 'light' | 'dark';

let theme = $state<Theme>('dark');
let sidebarOpen = $state(true);

export function getTheme(): Theme {
  return theme;
}

export function toggleTheme(): void {
  theme = theme === 'dark' ? 'light' : 'dark';
  document.documentElement.setAttribute('data-theme', theme);
}

export function setTheme(t: Theme): void {
  theme = t;
  document.documentElement.setAttribute('data-theme', theme);
}

export function getSidebarOpen(): boolean {
  return sidebarOpen;
}

export function toggleSidebar(): void {
  sidebarOpen = !sidebarOpen;
}
