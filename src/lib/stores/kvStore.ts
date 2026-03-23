/**
 * 跨平台 key-value 存储抽象。
 * - Tauri 桌面端：使用 @tauri-apps/plugin-store（基于文件）
 * - Web / PWA 端：使用 localStorage
 */
import { load } from '@tauri-apps/plugin-store';
import { isTauri } from '$lib/api/platform';

export interface KVStore {
  get<T>(key: string): Promise<T | undefined>;
  set(key: string, value: unknown): Promise<void>;
  delete(key: string): Promise<void>;
  save(): Promise<void>;
}

// Web localStorage implementation
class WebStore implements KVStore {
  private prefix: string;

  constructor(name: string) {
    // Use store name as localStorage key prefix to avoid collisions
    this.prefix = `orion:${name}:`;
  }

  async get<T>(key: string): Promise<T | undefined> {
    const raw = localStorage.getItem(this.prefix + key);
    if (raw === null) return undefined;
    try {
      return JSON.parse(raw) as T;
    } catch {
      return undefined;
    }
  }

  async set(key: string, value: unknown): Promise<void> {
    localStorage.setItem(this.prefix + key, JSON.stringify(value));
  }

  async delete(key: string): Promise<void> {
    localStorage.removeItem(this.prefix + key);
  }

  async save(): Promise<void> {
    // localStorage is synchronous; no-op
  }
}

/**
 * Load a named KV store.
 * In Tauri, delegates to plugin-store; in web, uses localStorage.
 */
export async function loadStore(name: string): Promise<KVStore> {
  if (isTauri()) {
    return load(name);
  }
  return new WebStore(name);
}
