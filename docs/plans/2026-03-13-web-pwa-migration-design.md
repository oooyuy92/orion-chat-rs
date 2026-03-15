# UI/UX 设计方案：Orion Chat Web/PWA 改造

**设计时间**：2026-03-13
**目标平台**：Web (桌面 + 移动) + Tauri 桌面端（统一代码库）

---

## 1. 设计目标

### 1.1 用户目标
- **桌面端用户**：保持现有 Tauri 体验，无感知变化
- **Web 端用户**：通过 Docker 私有化部署，浏览器访问
- **移动端用户**：PWA 安装后获得类原生体验

### 1.2 业务目标
- 统一代码库，业务逻辑只写一次
- 降低部署门槛（Docker 一键部署）
- 扩大用户覆盖（支持移动设备）
- 保持桌面端性能和功能优势

---

## 2. 架构设计

### 2.1 环境检测与 API 统一层

#### 核心思路
创建统一的 API 抽象层，自动检测运行环境（Tauri/Web），调用对应的实现。

#### 目录结构
```
src/lib/
├── api/
│   ├── index.ts              # 统一导出，运行时选择实现
│   ├── platform.ts           # 环境检测
│   ├── types.ts              # 共享接口定义（从 invoke.ts 迁移）
│   ├── tauri/
│   │   └── impl.ts           # Tauri 实现（封装现有 invoke.ts）
│   └── web/
│       ├── impl.ts           # Web 实现（HTTP + SSE）
│       └── sse.ts            # SSE 流式处理工具
├── utils/
│   └── invoke.ts             # 保留兼容，内部改为 re-export api/index.ts
```

#### 环境检测
```typescript
// src/lib/api/platform.ts
export type Platform = 'tauri' | 'web';

export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}
```

#### 统一 API 接口（index.ts）
```typescript
// src/lib/api/index.ts
import { isTauri } from './platform';
import type { ApiInterface } from './types';

// 运行时动态选择实现
// 注意：Vite 的 tree-shaking 会在构建时保留两者，但运行时只执行一个分支
let _api: ApiInterface | null = null;

export async function getApi(): Promise<ApiInterface> {
  if (_api) return _api;
  if (isTauri()) {
    const { tauriApi } = await import('./tauri/impl');
    _api = tauriApi;
  } else {
    const { webApi } = await import('./web/impl');
    _api = webApi;
  }
  return _api;
}

// 同步版本（在 onMount 后使用）
export function createApi(): ApiInterface {
  if (isTauri()) {
    // 同步 import，Tauri 环境下已经加载
    const { tauriApi } = require('./tauri/impl');
    return tauriApi;
  } else {
    const { webApi } = require('./web/impl');
    return webApi;
  }
}
```

**实际推荐方案**：不用动态 import，直接在 index.ts 中 import 两者，用 isTauri() 做运行时分支。Vite 构建时两个实现都会打包，但 Tauri 的 `@tauri-apps/api` 在 Web 环境下不会被调用，不影响功能。

```typescript
// src/lib/api/index.ts（推荐方案）
import { isTauri } from './platform';
import { tauriApi } from './tauri/impl';
import { webApi } from './web/impl';
import type { ApiInterface } from './types';

export const api: ApiInterface = isTauri() ? tauriApi : webApi;
export type { ApiInterface, ChatEvent, ChatEventHandler, AppPaths } from './types';
```

