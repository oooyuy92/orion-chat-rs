# Orion Chat RS - shadcn UI 重构设计

**日期**: 2026-03-04
**状态**: Approved
**目标**: 将UI完全改造为shadcn风格，保持亮色主题，保留所有现有功能

## 背景

当前UI使用自定义CSS + Tailwind，有以下问题：
- 双主题系统（亮色+暗色）维护成本高
- 消息气泡样式不够现代（两边都有背景色和边框）
- 输入框样式自定义，不够精致
- 侧边栏缺少model selector和时间分组
- 设置页面是全屏显示，不够轻量

用户需求：
- 完全复刻 shadcn.io/ai/chatbot 的视觉风格
- 始终保持亮色界面（移除暗色主题）
- 侧边栏要有model selector + 时间分组对话列表 + 用户菜单
- 设置改为弹窗（侧边栏底部点击弹出）
- 保留所有现有功能（CompareView/SearchPanel/AssistantList）

## 技术方案

### 方案选择

**选定方案**：引入 shadcn-svelte 组件库（方案2）

**理由**：
- 可以利用现成的高质量组件（Button/Card/Dialog/DropdownMenu等）
- 样式100%符合shadcn规范
- 有TypeScript类型支持
- 不兼容的组件可以手写（灵活性高）

**替代方案**：
- 方案1（渐进式迁移）：只改CSS，不引入组件库 — 工作量大，样式类名冗长
- 方案3（纯手写复刻）：完全手写所有组件 — 维护成本高

### 技术栈

**保持不变**：
- Svelte 5 + SvelteKit (adapter-static)
- Tauri v2
- Rust后端（所有business logic）
- SQLite数据库
- Tailwind CSS（升级到v4）

**新增依赖**：
```json
{
  "shadcn-svelte": "^0.13.0",
  "bits-ui": "^0.21.0",
  "clsx": "^2.1.0",
  "tailwind-merge": "^2.2.0",
  "tailwind-variants": "^0.2.0"
}
```

**需要安装的shadcn组件**（通过CLI）：
- Button
- Card
- Dialog
- DropdownMenu
- Separator
- Avatar
- Badge
- Input
- Textarea
- ScrollArea
- Popover
- Sidebar (SidebarProvider/SidebarContent/SidebarHeader/SidebarFooter等)

## 色彩系统

### 完全移除暗色主题

- 删除 `src/lib/stores/ui.svelte.ts` 中的 theme 状态
- 删除所有 `.dark:` 相关的 Tailwind 类
- 只保留亮色主题的 CSS 变量

### 新的色彩系统（oklch）

从 `hsl()` 切换到 `oklch()`（更现代的色彩空间，感知均匀）

```css
@layer base {
  :root {
    /* 色彩系统 - oklch，纯中性灰（无色相） */
    --background: oklch(1 0 0);                   /* 纯白 */
    --foreground: oklch(0.145 0 0);               /* 近黑 */
    --card: oklch(1 0 0);
    --card-foreground: oklch(0.145 0 0);
    --primary: oklch(0.205 0 0);                  /* 深灰 */
    --primary-foreground: oklch(0.985 0 0);
    --secondary: oklch(0.97 0 0);                 /* 浅灰气泡背景 */
    --secondary-foreground: oklch(0.205 0 0);
    --muted: oklch(0.97 0 0);
    --muted-foreground: oklch(0.556 0 0);
    --accent: oklch(0.97 0 0);
    --border: oklch(0.922 0 0);                   /* 浅灰边框 */
    --input: oklch(0.922 0 0);
    --ring: oklch(0.708 0 0);
    --sidebar: oklch(0.985 0 0);                  /* 侧边栏微灰 */
    --sidebar-foreground: oklch(0.145 0 0);
    --sidebar-border: oklch(0.922 0 0);

    /* 圆角系统 */
    --radius: 0.625rem;                           /* 10px */
    --radius-sm: calc(var(--radius) - 4px);       /* 6px */
    --radius-lg: var(--radius);                   /* 10px */
    --radius-xl: calc(var(--radius) + 4px);       /* 14px */
    --radius-2xl: calc(var(--radius) + 8px);      /* 18px */
    --radius-4xl: calc(var(--radius) + 16px);     /* 26px - 完全圆形按钮 */

    /* 侧边栏宽度 */
    --sidebar-width: 16rem;                       /* 256px */
    --sidebar-width-icon: 3rem;                   /* 48px 折叠态 */
  }
}
```

**关键变化**：
- 去掉所有色相（hue），纯中性灰系统
- `--radius` 从 0.75rem 改为 0.625rem（10px）
- 新增 `--radius-4xl` 用于完全圆形的发送按钮
- 侧边栏宽度从自定义改为 shadcn 标准的 16rem

**视觉效果**：
- 整体更加克制、专业
- 没有任何彩色accent（除非hover/focus状态）
- 边框和阴影都非常subtle

## 组件架构

### 整体布局结构

```
+page.svelte
├── SidebarProvider (shadcn-svelte组件)
│   ├── AppSidebar.svelte (新组件)
│   │   ├── SidebarHeader
│   │   │   └── ModelSelector.svelte (下拉选择器)
│   │   ├── SidebarContent
│   │   │   └── ConversationList.svelte (改造：按时间分组)
│   │   └── SidebarFooter
│   │       └── UserMenu.svelte (新组件：头像+设置弹窗)
│   └── SidebarInset (主内容区)
│       └── ChatArea.svelte (新组件)
│           ├── ChatHeader.svelte (可选：显示当前对话标题)
│           ├── MessageList.svelte (改造)
│           │   └── MessageBubble.svelte (重写)
│           └── InputArea.svelte (重写：InputGroup风格)
```

### 组件分类

**保留并改造的组件**（7个）：
1. `ConversationList.svelte` - 加时间分组逻辑
2. `MessageList.svelte` - 改用shadcn样式
3. `MessageBubble.svelte` - 完全重写（用户=气泡，助手=纯文本）
4. `InputArea.svelte` - 重写为InputGroup结构
5. `ModelSelector.svelte` - 改成shadcn DropdownMenu
6. `ProviderSettings.svelte` - 改成Dialog弹窗内容
7. `CompareView.svelte` / `SearchPanel.svelte` / `AssistantList.svelte` - 用shadcn Card重新设计

**新增组件**（4个）：
1. `AppSidebar.svelte` - 侧边栏容器（包含header/content/footer）
2. `UserMenu.svelte` - 用户头像+设置弹窗触发器
3. `ChatArea.svelte` - 聊天区容器
4. `ChatHeader.svelte` - 聊天区顶部（显示当前对话标题，可选）

**删除组件**（1个）：
- `ui.svelte.ts` 中的 theme 状态（不再需要暗色主题）

## 关键组件设计

### 1. MessageBubble.svelte（重写）

**用户消息**：右对齐气泡，浅灰背景
```svelte
<div class="group flex w-full max-w-[95%] ml-auto justify-end">
  <div class="flex w-fit max-w-full flex-col gap-2 rounded-lg bg-secondary px-4 py-3 text-sm text-foreground">
    {content}
  </div>
</div>
```

**助手消息**：左对齐纯文本，无背景
```svelte
<div class="group flex w-full max-w-[95%]">
  <div class="flex w-fit max-w-full flex-col gap-2 text-sm text-foreground">
    {@html markdownContent}
  </div>
</div>
```

**关键点**：
- 用户消息有 `bg-secondary` 背景色（浅灰气泡）
- 助手消息**没有背景**，只是纯文本
- 用户消息 `ml-auto` 右对齐
- 助手消息左对齐
- 都用 `text-sm`（14px）
- 圆角 `rounded-lg`（10px）

### 2. InputArea.svelte（重写为InputGroup）

参考shadcn的PromptInput结构：

```svelte
<form class="w-full" on:submit|preventDefault={handleSend}>
  <!-- InputGroup容器 -->
  <div class="group/input-group relative flex w-full items-center rounded-md border border-input shadow-xs
              has-[textarea:focus-visible]:border-ring has-[textarea:focus-visible]:ring-ring/50 has-[textarea:focus-visible]:ring-[3px]">

    <!-- 自适应textarea -->
    <textarea
      bind:value={message}
      class="flex-1 resize-none rounded-none border-0 bg-transparent py-3 px-4 shadow-none focus-visible:ring-0
             field-sizing-content min-h-16 max-h-48"
      placeholder="What would you like to know?"
    />

    <!-- 发送按钮：完全圆形 -->
    <button
      type="submit"
      class="size-8 rounded-4xl bg-primary text-primary-foreground hover:bg-primary/80 m-2"
    >
      <SendIcon />
    </button>
  </div>
</form>
```

**关键点**：
- 外层容器有 `border` + `shadow-xs`
- Focus时有 `ring-[3px]` 的ring效果
- Textarea用 `field-sizing-content` 自适应高度（CSS新特性）
- 发送按钮用 `rounded-4xl`（完全圆形）
- 最小高度 `min-h-16`，最大高度 `max-h-48`

### 3. ConversationList.svelte（改造：加时间分组）

**时间分组逻辑**：

```typescript
function groupConversationsByTime(conversations: Conversation[]) {
  const now = new Date();
  const today = startOfDay(now);
  const yesterday = startOfDay(subDays(now, 1));
  const last7Days = startOfDay(subDays(now, 7));
  const last30Days = startOfDay(subDays(now, 30));

  return {
    today: conversations.filter(c => new Date(c.updatedAt) >= today),
    yesterday: conversations.filter(c => {
      const date = new Date(c.updatedAt);
      return date >= yesterday && date < today;
    }),
    last7Days: conversations.filter(c => {
      const date = new Date(c.updatedAt);
      return date >= last7Days && date < yesterday;
    }),
    last30Days: conversations.filter(c => {
      const date = new Date(c.updatedAt);
      return date >= last30Days && date < last7Days;
    }),
    older: conversations.filter(c => new Date(c.updatedAt) < last30Days)
  };
}
```

**UI结构**：
```svelte
<SidebarGroup>
  {#if grouped.today.length > 0}
    <SidebarGroupLabel>Today</SidebarGroupLabel>
    <SidebarMenu>
      {#each grouped.today as conv}
        <SidebarMenuItem>
          <SidebarMenuButton>{conv.title}</SidebarMenuButton>
        </SidebarMenuItem>
      {/each}
    </SidebarMenu>
  {/if}

  {#if grouped.yesterday.length > 0}
    <SidebarGroupLabel>Yesterday</SidebarGroupLabel>
    <!-- ... -->
  {/if}

  <!-- Last 7 days, Last 30 days, Older 同理 -->
</SidebarGroup>
```

### 4. UserMenu.svelte（新组件）

侧边栏底部的用户区，点击弹出设置：

```svelte
<SidebarFooter>
  <SidebarMenu>
    <SidebarMenuItem>
      <DropdownMenu>
        <DropdownMenuTrigger asChild let:builder>
          <SidebarMenuButton use:builder.action {...builder}>
            <Avatar class="size-8">
              <AvatarFallback>U</AvatarFallback>
            </Avatar>
            <span>User</span>
          </SidebarMenuButton>
        </DropdownMenuTrigger>

        <DropdownMenuContent>
          <DropdownMenuItem on:click={() => settingsOpen = true}>
            Settings
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </SidebarMenuItem>
  </SidebarMenu>
</SidebarFooter>

<!-- 设置Dialog -->
<Dialog bind:open={settingsOpen}>
  <DialogContent class="max-w-2xl">
    <ProviderSettings />
  </DialogContent>
</Dialog>
```

## 数据流和状态管理

### 保持现有架构不变

- Rust后端完全不动（所有Tauri commands保持原样）
- 前端通过 `src/lib/utils/invoke.ts` 调用Tauri API
- 状态管理继续用Svelte 5 runes（`$state`, `$derived`, `$effect`）

### 状态管理调整

```typescript
// src/lib/stores/ui.svelte.ts（简化版）
export const ui = (() => {
  let sidebarOpen = $state(true);  // 侧边栏展开/折叠

  return {
    get sidebarOpen() { return sidebarOpen; },
    set sidebarOpen(value: boolean) { sidebarOpen = value; },
    toggleSidebar() { sidebarOpen = !sidebarOpen; }
  };
})();

// 删除 theme 相关状态（不再需要）
```

### 新增工具函数

```typescript
// src/lib/utils/date.ts（新文件）
export function groupConversationsByTime(conversations: Conversation[]) {
  // 时间分组逻辑
}

export function formatRelativeTime(date: string): string {
  // "2 hours ago", "Yesterday", etc.
}
```

```typescript
// src/lib/utils/cn.ts（新文件，shadcn标配）
import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

### 数据流保持不变

1. 用户发送消息 → `api.sendMessage()` → Tauri command → Rust provider
2. Rust通过Channel发送流式事件 → 前端接收 → 更新UI
3. 消息持久化到SQLite → 重启后从DB加载

**关键点**：
- **只改UI层**，不动业务逻辑
- shadcn-svelte组件只负责展示，不管理业务状态
- 继续用现有的 `api.listProviders()`, `api.sendMessage()` 等调用
- CompareView/SearchPanel/AssistantList 的数据流保持不变，只改样式

## 兼容性处理策略

1. **优先使用 shadcn-svelte 组件**
2. **如果组件和 Svelte 5 runes 冲突**（比如 `$state` 在组件内部不work），就手写该组件
3. **手写时参考 shadcn/ui v4 源码**，保持视觉一致性

## 不改动的部分

- Rust后端（所有Tauri commands）
- 数据库schema
- 业务逻辑和数据流
- Tauri invoke调用
- 所有provider实现（openai_compat/anthropic/gemini/ollama）
- 流式响应机制
- 消息持久化逻辑

## 改动的部分

- 所有Svelte组件的样式和结构
- CSS变量和色彩系统
- 主题系统（移除暗色主题）
- 侧边栏布局（加model selector + 时间分组 + 用户菜单）
- 消息气泡样式（用户=气泡，助手=纯文本）
- 输入框结构（改为InputGroup）
- 设置页面（全屏 → Dialog弹窗）

## 参考资源

- [shadcn/ui v4 官方文档](https://ui.shadcn.com/)
- [shadcn-svelte 文档](https://www.shadcn-svelte.com/)
- [shadcn AI Chatbot 示例](https://www.shadcn.io/ai/chatbot)
- [shadcn/ui GitHub Repository](https://github.com/shadcn-ui/ui)
- [Vercel AI Elements](https://elements.ai-sdk.dev/)

## 成功标准

1. ✅ UI视觉效果与 shadcn.io/ai/chatbot 高度一致
2. ✅ 只有亮色主题，无暗色主题
3. ✅ 侧边栏有model selector + 时间分组对话列表 + 用户菜单
4. ✅ 消息气泡：用户=浅灰气泡，助手=纯文本无背景
5. ✅ 输入框为InputGroup结构，有border + shadow + focus ring
6. ✅ 设置为Dialog弹窗，不是全屏页面
7. ✅ 所有现有功能正常工作（CompareView/SearchPanel/AssistantList）
8. ✅ 流式响应、消息持久化、provider管理等核心功能不受影响
9. ✅ 编译通过，无TypeScript错误
10. ✅ 运行 `pnpm tauri dev` 可以正常使用
