## 2.2 Web API 实现方案

### HTTP API 设计

#### 基础配置
```typescript
// src/lib/api/web/config.ts
export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '/api';

// 自动从 cookie/localStorage 读取 token
export function getAuthToken(): string | null {
  return localStorage.getItem('auth_token');
}
```

#### 核心实现
```typescript
// src/lib/api/web/impl.ts
import type { ApiInterface, ChatEventHandler, ChatEvent } from '../types';
import { API_BASE_URL, getAuthToken } from './config';
import { createSSEStream } from './sse';

async function fetchApi<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
  const token = getAuthToken();
  const headers = {
    'Content-Type': 'application/json',
    ...(token && { Authorization: `Bearer ${token}` }),
    ...options.headers,
  };

  const response = await fetch(`${API_BASE_URL}${endpoint}`, {
    ...options,
    headers,
  });

  if (!response.ok) {
    throw new Error(`API Error: ${response.status} ${response.statusText}`);
  }

  return response.json();
}

export const webApi: ApiInterface = {
  // Conversations
  createConversation(title, assistantId, modelId) {
    return fetchApi('/conversations', {
      method: 'POST',
      body: JSON.stringify({ title, assistantId, modelId }),
    });
  },

  listConversations() {
    return fetchApi('/conversations');
  },

  updateConversationTitle(id, title) {
    return fetchApi(`/conversations/${id}/title`, {
      method: 'PUT',
      body: JSON.stringify({ title }),
    });
  },

  deleteConversation(id) {
    return fetchApi(`/conversations/${id}`, { method: 'DELETE' });
  },

  // Messages - 流式处理
  async sendMessage(conversationId, content, modelId, onEvent, commonParams, providerParams) {
    const token = getAuthToken();
    const response = await fetch(`${API_BASE_URL}/messages/send`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(token && { Authorization: `Bearer ${token}` }),
      },
      body: JSON.stringify({
        conversationId,
        content,
        modelId,
        commonParams,
        providerParams,
      }),
    });

    if (!response.ok) {
      throw new Error(`Send failed: ${response.status}`);
    }

    // SSE 流式处理
    await createSSEStream(response, onEvent);

    // 返回最终消息（从最后一个 finished 事件中获取）
    // 实际实现中需要在 onEvent 中收集
    return {} as any; // 简化示例
  },

  // 其他方法类似...
};
```

### SSE 流式处理
```typescript
// src/lib/api/web/sse.ts
import type { ChatEvent, ChatEventHandler } from '../types';

export async function createSSEStream(
  response: Response,
  onEvent: ChatEventHandler
): Promise<void> {
  const reader = response.body?.getReader();
  if (!reader) throw new Error('No response body');

  const decoder = new TextDecoder();
  let buffer = '';

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const data = line.slice(6);
          if (data === '[DONE]') return;

          try {
            const event: ChatEvent = JSON.parse(data);
            onEvent(event);
          } catch (e) {
            console.error('Failed to parse SSE event:', e);
          }
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
}
```

---

## 2.3 桌面专属功能处理

### 功能分类

| 功能 | Tauri 实现 | Web 实现 | 处理策略 |
|------|-----------|---------|---------|
| 文件选择器 | `pickDirectory()` | `<input type="file">` | 条件渲染 |
| 打开路径 | `openPath()` | 下载链接 | 条件渲染 |
| 自动启动 | `autostart` plugin | 不支持 | 隐藏设置项 |
| 本地备份 | `localBackup()` | 导出下载 | 不同 UI |
| 系统托盘 | Tauri 原生 | 不支持 | Tauri 独有 |
| 窗口控制 | Tauri API | 不支持 | Tauri 独有 |

### 条件渲染示例
```svelte
<!-- src/lib/components/settings/DataSettings.svelte -->
<script lang="ts">
  import { isTauri } from '$lib/api/platform';
  import { api } from '$lib/api';

  async function handleBackup() {
    if (isTauri()) {
      const destPath = await api.pickDirectory();
      if (destPath) {
        await api.localBackup(destPath);
      }
    } else {
      // Web: 导出为 JSON 下载
      const data = await api.exportAllData();
      downloadJson(data, 'orion-backup.json');
    }
  }
</script>

<Button onclick={handleBackup}>
  {isTauri() ? '本地备份' : '导出数据'}
</Button>

{#if isTauri()}
  <div class="mt-4">
    <h3>自动启动</h3>
    <Switch bind:checked={autostartEnabled} />
  </div>
{/if}
```

---

## 3. PWA 配置方案

### 3.1 Manifest 配置

```json
// static/manifest.json
{
  "name": "Orion Chat",
  "short_name": "Orion",
  "description": "AI Chat Assistant with Multi-Model Support",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#ffffff",
  "theme_color": "#3B82F6",
  "orientation": "any",
  "icons": [
    {
      "src": "/icons/icon-72x72.png",
      "sizes": "72x72",
      "type": "image/png",
      "purpose": "any"
    },
    {
      "src": "/icons/icon-96x96.png",
      "sizes": "96x96",
      "type": "image/png",
      "purpose": "any"
    },
    {
      "src": "/icons/icon-128x128.png",
      "sizes": "128x128",
      "type": "image/png",
      "purpose": "any"
    },
    {
      "src": "/icons/icon-144x144.png",
      "sizes": "144x144",
      "type": "image/png",
      "purpose": "any"
    },
    {
      "src": "/icons/icon-152x152.png",
      "sizes": "152x152",
      "type": "image/png",
      "purpose": "any"
    },
    {
      "src": "/icons/icon-192x192.png",
      "sizes": "192x192",
      "type": "image/png",
      "purpose": "any"
    },
    {
      "src": "/icons/icon-384x384.png",
      "sizes": "384x384",
      "type": "image/png",
      "purpose": "any"
    },
    {
      "src": "/icons/icon-512x512.png",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "any"
    },
    {
      "src": "/icons/maskable-icon-512x512.png",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "maskable"
    }
  ],
  "screenshots": [
    {
      "src": "/screenshots/desktop.png",
      "sizes": "1280x720",
      "type": "image/png",
      "form_factor": "wide"
    },
    {
      "src": "/screenshots/mobile.png",
      "sizes": "750x1334",
      "type": "image/png",
      "form_factor": "narrow"
    }
  ],
  "categories": ["productivity", "utilities"],
  "shortcuts": [
    {
      "name": "New Chat",
      "short_name": "New",
      "description": "Start a new conversation",
      "url": "/?action=new",
      "icons": [{ "src": "/icons/new-chat.png", "sizes": "96x96" }]
    }
  ]
}
```

### 3.2 HTML Head 配置

```html
<!-- src/app.html -->
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />

  <!-- PWA Meta -->
  <link rel="manifest" href="/manifest.json" />
  <meta name="theme-color" content="#3B82F6" />
  <meta name="apple-mobile-web-app-capable" content="yes" />
  <meta name="apple-mobile-web-app-status-bar-style" content="default" />
  <meta name="apple-mobile-web-app-title" content="Orion Chat" />

  <!-- Apple Touch Icons -->
  <link rel="apple-touch-icon" href="/icons/icon-152x152.png" />
  <link rel="apple-touch-icon" sizes="180x180" href="/icons/icon-180x180.png" />

  <!-- Favicon -->
  <link rel="icon" type="image/png" sizes="32x32" href="/icons/favicon-32x32.png" />
  <link rel="icon" type="image/png" sizes="16x16" href="/icons/favicon-16x16.png" />

  %sveltekit.head%
</head>
<body>
  <div id="app">%sveltekit.body%</div>
</body>
</html>
```

### 3.3 Service Worker

```typescript
// src/service-worker.ts
/// <reference types="@sveltejs/kit" />
/// <reference no-default-lib="true"/>
/// <reference lib="esnext" />
/// <reference lib="webworker" />

import { build, files, version } from '$service-worker';

const sw = self as unknown as ServiceWorkerGlobalScope;

const CACHE_NAME = `orion-chat-${version}`;
const STATIC_ASSETS = [...build, ...files];

// Install: 缓存静态资源
sw.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll(STATIC_ASSETS))
  );
  sw.skipWaiting();
});

// Activate: 清理旧缓存
sw.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(
        keys
          .filter((key) => key !== CACHE_NAME)
          .map((key) => caches.delete(key))
      )
    )
  );
  sw.clients.claim();
});

// Fetch: 网络优先，失败时使用缓存
sw.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // API 请求：仅网络
  if (url.pathname.startsWith('/api/')) {
    event.respondWith(fetch(request));
    return;
  }

  // 静态资源：缓存优先
  if (STATIC_ASSETS.includes(url.pathname)) {
    event.respondWith(
      caches.match(request).then((cached) => cached || fetch(request))
    );
    return;
  }

  // 其他：网络优先，失败时使用缓存
  event.respondWith(
    fetch(request)
      .then((response) => {
        if (response.ok) {
          const clone = response.clone();
          caches.open(CACHE_NAME).then((cache) => cache.put(request, clone));
        }
        return response;
      })
      .catch(() => caches.match(request).then((cached) => cached || new Response('Offline')))
  );
});
```

### 3.4 安装提示组件

```svelte
<!-- src/lib/components/pwa/InstallPrompt.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { Button } from '$lib/components/ui/button';
  import { X } from 'lucide-svelte';

  let deferredPrompt: any = null;
  let showPrompt = $state(false);

  onMount(() => {
    window.addEventListener('beforeinstallprompt', (e) => {
      e.preventDefault();
      deferredPrompt = e;

      // 检查是否已经提示过
      const dismissed = localStorage.getItem('pwa-install-dismissed');
      if (!dismissed) {
        showPrompt = true;
      }
    });
  });

  async function handleInstall() {
    if (!deferredPrompt) return;

    deferredPrompt.prompt();
    const { outcome } = await deferredPrompt.userChoice;

    if (outcome === 'accepted') {
      console.log('PWA installed');
    }

    deferredPrompt = null;
    showPrompt = false;
  }

  function handleDismiss() {
    showPrompt = false;
    localStorage.setItem('pwa-install-dismissed', 'true');
  }
</script>

{#if showPrompt}
  <div class="fixed bottom-4 left-4 right-4 md:left-auto md:right-4 md:w-96
              bg-card border rounded-lg shadow-lg p-4 z-50">
    <button
      onclick={handleDismiss}
      class="absolute top-2 right-2 p-1 hover:bg-muted rounded"
    >
      <X class="w-4 h-4" />
    </button>

    <h3 class="font-semibold mb-2">安装 Orion Chat</h3>
    <p class="text-sm text-muted-foreground mb-4">
      安装到主屏幕，获得更好的使用体验
    </p>

    <div class="flex gap-2">
      <Button onclick={handleInstall} class="flex-1">
        安装
      </Button>
      <Button variant="outline" onclick={handleDismiss}>
        稍后
      </Button>
    </div>
  </div>
{/if}
```

### 3.5 SvelteKit 配置

```typescript
// svelte.config.js
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      fallback: 'index.html',
      precompress: true, // 启用 gzip/brotli 压缩
    }),
    serviceWorker: {
      register: false, // 手动注册
    },
  },
};

export default config;
```

```typescript
// src/routes/+layout.ts
export const prerender = true;
export const ssr = false; // SPA 模式
export const trailingSlash = 'always';
```

```svelte
<!-- src/routes/+layout.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { isWeb } from '$lib/api/platform';
  import InstallPrompt from '$lib/components/pwa/InstallPrompt.svelte';

  onMount(() => {
    // 注册 Service Worker（仅 Web 环境）
    if (isWeb() && 'serviceWorker' in navigator) {
      navigator.serviceWorker.register('/service-worker.js');
    }
  });
</script>

<!-- 现有布局 -->
<slot />

<!-- PWA 安装提示（仅 Web 环境） -->
{#if isWeb()}
  <InstallPrompt />
{/if}
```
