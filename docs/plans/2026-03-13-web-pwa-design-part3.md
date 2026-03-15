## 4. 移动端 UI 适配方案

### 4.1 布局结构

#### 桌面端（保持不变）
```
+------------------+----------------------------------+
|                  |  ChatArea Header                 |
|   AppSidebar     |----------------------------------|
|   (固定宽度)     |  MessageList                     |
|                  |                                  |
|                  |----------------------------------|
|                  |  ChatInput                       |
+------------------+----------------------------------+
```

#### 移动端（抽屉式侧边栏）
```
+------------------------------------------+
|  [≡] Orion Chat              [+] [...]   |
+------------------------------------------+
|                                          |
|  MessageList                             |
|                                          |
+------------------------------------------+
|  ChatInput                               |
+------------------------------------------+

侧边栏展开时（覆盖层）：
+------------------------------------------+
|  [×] 对话列表                            |
|  --------------------------------        |
|  > 对话 1                                |
|  > 对话 2                                |
|  ...                                     |
+------------------------------------------+
```

### 4.2 响应式断点策略

| 断点 | 宽度 | 布局 | 侧边栏 |
|------|------|------|--------|
| mobile | < 768px | 单列 | 抽屉式（Drawer） |
| tablet | 768px - 1023px | 单列 | 抽屉式（Drawer） |
| desktop | >= 1024px | 双列 | 固定侧边栏（现有） |

### 4.3 核心组件改造

#### AppSidebar 改造
```svelte
<!-- src/lib/components/sidebar/AppSidebar.svelte 改造思路 -->
<script lang="ts">
  import { isMobile } from '$lib/stores/viewport.svelte';

  // 移动端：受控的 open 状态
  let mobileOpen = $state(false);
</script>

{#if isMobile()}
  <!-- 移动端：Drawer 模式 -->
  <Drawer bind:open={mobileOpen}>
    <DrawerContent class="w-[280px] h-full">
      <SidebarContent onClose={() => mobileOpen = false} />
    </DrawerContent>
  </Drawer>
{:else}
  <!-- 桌面端：现有 SidebarProvider 模式 -->
  <SidebarProvider>
    <SidebarContent />
  </SidebarProvider>
{/if}
```

#### 视口检测 Store
```typescript
// src/lib/stores/viewport.svelte.ts
let _isMobile = $state(false);

if (typeof window !== 'undefined') {
  const mq = window.matchMedia('(max-width: 767px)');
  _isMobile = mq.matches;

  mq.addEventListener('change', (e) => {
    _isMobile = e.matches;
  });
}

export function isMobile() {
  return _isMobile;
}
```

#### 移动端 ChatArea Header
```svelte
<!-- 移动端顶部导航栏 -->
<header class="flex items-center gap-2 px-4 py-3 border-b
               md:hidden
               safe-area-top">
  <!-- 汉堡菜单 -->
  <button
    onclick={() => sidebarOpen = true}
    class="p-2 -ml-2 rounded-lg hover:bg-muted touch-target"
    aria-label="打开对话列表"
  >
    <Menu class="w-5 h-5" />
  </button>

  <!-- 当前对话标题 -->
  <h1 class="flex-1 text-sm font-medium truncate">
    {currentConversation?.title ?? 'Orion Chat'}
  </h1>

  <!-- 新建对话 -->
  <button
    onclick={handleNewConversation}
    class="p-2 rounded-lg hover:bg-muted touch-target"
    aria-label="新建对话"
  >
    <Plus class="w-5 h-5" />
  </button>
</header>
```

### 4.4 安全区域适配

#### CSS 变量定义
```css
/* src/app.css */
:root {
  --safe-area-top: env(safe-area-inset-top, 0px);
  --safe-area-bottom: env(safe-area-inset-bottom, 0px);
  --safe-area-left: env(safe-area-inset-left, 0px);
  --safe-area-right: env(safe-area-inset-right, 0px);
}
```

#### Tailwind 工具类（Tailwind 4 方式）
```css
/* src/app.css */
@utility safe-area-top {
  padding-top: env(safe-area-inset-top, 0px);
}

@utility safe-area-bottom {
  padding-bottom: env(safe-area-inset-bottom, 0px);
}

@utility safe-area-x {
  padding-left: env(safe-area-inset-left, 0px);
  padding-right: env(safe-area-inset-right, 0px);
}
```

#### 关键位置应用
```svelte
<!-- 根布局 -->
<div class="flex h-screen safe-area-x" style="...">

<!-- 移动端顶部 Header -->
<header class="safe-area-top ...">

<!-- 底部输入框 -->
<div class="safe-area-bottom ...">
```

### 4.5 软键盘处理

#### 问题
iOS Safari 弹出软键盘时，`100vh` 不会缩小，导致输入框被遮挡。

#### 解决方案
```typescript
// src/lib/utils/viewport.ts
export function setupViewportHeightFix() {
  if (typeof window === 'undefined') return;

  function updateVh() {
    // 使用 visualViewport 获取实际可见高度
    const vh = window.visualViewport?.height ?? window.innerHeight;
    document.documentElement.style.setProperty('--vh', `${vh * 0.01}px`);
  }

  updateVh();
  window.visualViewport?.addEventListener('resize', updateVh);
  window.addEventListener('resize', updateVh);
}
```

```css
/* 使用 --vh 替代 vh 单位 */
.h-screen-safe {
  height: calc(var(--vh, 1vh) * 100);
}
```

```svelte
<!-- src/routes/+layout.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { setupViewportHeightFix } from '$lib/utils/viewport';

  onMount(() => {
    setupViewportHeightFix();
  });
</script>

<div class="flex h-screen-safe" style="...">
```

#### 输入框焦点处理
```svelte
<!-- ChatInput 组件 -->
<script lang="ts">
  import { isMobile } from '$lib/stores/viewport.svelte';

  let textarea: HTMLTextAreaElement;

  function handleFocus() {
    if (isMobile()) {
      // 延迟滚动，等待键盘弹出
      setTimeout(() => {
        textarea.scrollIntoView({ behavior: 'smooth', block: 'end' });
      }, 300);
    }
  }
</script>

<textarea
  bind:this={textarea}
  onfocus={handleFocus}
  class="..."
/>
```

### 4.6 触摸交互优化

#### 替换 hover-only 操作
```svelte
<!-- 消息操作按钮：桌面端 hover 显示，移动端长按显示 -->
<script lang="ts">
  import { isMobile } from '$lib/stores/viewport.svelte';

  let showActions = $state(false);
  let longPressTimer: ReturnType<typeof setTimeout>;

  function handleTouchStart() {
    longPressTimer = setTimeout(() => {
      showActions = true;
    }, 500);
  }

  function handleTouchEnd() {
    clearTimeout(longPressTimer);
  }
</script>

<div
  class="group relative"
  ontouchstart={handleTouchStart}
  ontouchend={handleTouchEnd}
>
  <!-- 消息内容 -->
  <MessageContent />

  <!-- 操作按钮：桌面端 hover 显示，移动端长按后显示 -->
  <div class={[
    'absolute top-0 right-0 flex gap-1',
    isMobile()
      ? showActions ? 'flex' : 'hidden'
      : 'hidden group-hover:flex'
  ].join(' ')}>
    <CopyButton />
    <DeleteButton />
  </div>
</div>
```

#### 触摸目标尺寸
```css
/* 确保所有可点击元素至少 44x44px */
@utility touch-target {
  min-width: 44px;
  min-height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
}
```

---

## 5. 组件改造清单

### 5.1 需要改造的组件

| 组件 | 改造内容 | 优先级 |
|------|---------|--------|
| `+layout.svelte` | 注册 SW、添加 InstallPrompt、viewport fix | 高 |
| `+page.svelte` | 移动端侧边栏状态管理 | 高 |
| `AppSidebar.svelte` | 移动端 Drawer 模式 | 高 |
| `ChatArea.svelte` | 移动端 Header、安全区域 | 高 |
| `ChatInput.svelte` | 软键盘处理、安全区域 bottom | 高 |
| `MessageItem.svelte` | 长按操作替换 hover | 中 |
| `ProviderSettings.svelte` | 隐藏 Tauri 专属设置项 | 中 |
| `DataSettings.svelte` | Web 模式下替换文件选择器 | 中 |

### 5.2 新增组件

| 组件 | 用途 |
|------|------|
| `InstallPrompt.svelte` | PWA 安装提示 |
| `MobileHeader.svelte` | 移动端顶部导航 |
| `MobileDrawer.svelte` | 移动端侧边栏抽屉 |

### 5.3 新增工具/Store

| 文件 | 用途 |
|------|------|
| `src/lib/api/platform.ts` | 环境检测 |
| `src/lib/api/index.ts` | 统一 API 入口 |
| `src/lib/api/tauri/impl.ts` | Tauri 实现 |
| `src/lib/api/web/impl.ts` | Web HTTP 实现 |
| `src/lib/api/web/sse.ts` | SSE 流式处理 |
| `src/lib/stores/viewport.svelte.ts` | 视口/设备检测 |
| `src/lib/utils/viewport.ts` | vh 修复工具 |
| `src/service-worker.ts` | PWA Service Worker |
| `static/manifest.json` | PWA Manifest |

---

## 6. 实施顺序建议

### Phase 1：API 抽象层（不影响现有功能）
1. 创建 `src/lib/api/platform.ts`
2. 将现有 `invoke.ts` 内容迁移到 `src/lib/api/tauri/impl.ts`
3. 创建 `src/lib/api/index.ts`，Tauri 环境下行为与现在完全一致
4. 将 `invoke.ts` 改为 re-export `api/index.ts`

### Phase 2：PWA 基础配置
1. 添加 `static/manifest.json`
2. 修改 `src/app.html`（添加 meta 标签）
3. 创建 `src/service-worker.ts`
4. 在 `+layout.svelte` 中注册 SW（仅 Web 环境）

### Phase 3：移动端 UI 适配
1. 添加 `viewport.svelte.ts` store
2. 修改根布局添加 viewport height fix
3. 添加安全区域 CSS 工具类
4. 改造 `AppSidebar` 支持 Drawer 模式
5. 添加移动端 `ChatArea` Header
6. 修复 `ChatInput` 软键盘问题

### Phase 4：桌面专属功能隐藏
1. 在 `ProviderSettings` 中隐藏 autostart 设置
2. 在 `DataSettings` 中替换文件选择器为 Web 版本
3. 隐藏其他 Tauri 专属 UI

---

## 7. 关键设计决策

### 7.1 为什么不用动态 import 做代码分割？
Tauri 的 `@tauri-apps/api` 在 Web 环境下不会被调用，即使打包进去也不会报错（它只是一些 JS 函数，不会自动执行）。动态 import 会增加复杂度，且 Tauri 包体积很小，不值得。

### 7.2 为什么 Service Worker 只在 Web 环境注册？
Tauri 应用不需要 SW，且 Tauri 有自己的更新机制。在 Tauri 中注册 SW 可能导致请求拦截冲突。

### 7.3 移动端侧边栏为什么用 Drawer 而不是 Sheet？
shadcn-svelte 的 `Sheet` 组件本质上就是 Drawer，可以直接使用。关键是要在 `<768px` 时切换到覆盖模式，而不是推挤内容。

### 7.4 viewport height fix 为什么必要？
iOS Safari 的 `100vh` 包含了地址栏高度，导致内容被遮挡。使用 `visualViewport.height` 可以获取真实可见高度。Android Chrome 在 Chrome 108+ 已修复此问题，但 iOS 仍需处理。
