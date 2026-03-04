# shadcn UI 重构实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将orion-chat-rs的UI完全改造为shadcn风格，保持亮色主题，保留所有现有功能

**Architecture:** 引入shadcn-svelte组件库，替换色彩系统为oklch纯中性灰，重写消息气泡（用户=气泡，助手=纯文本），重写输入框为InputGroup结构，侧边栏加model selector + 时间分组对话列表 + 用户菜单，设置改为Dialog弹窗

**Tech Stack:** Svelte 5, SvelteKit, shadcn-svelte, bits-ui, Tailwind CSS v4, Tauri v2

---

## Phase 1: 环境准备和依赖安装

### Task 1: 安装shadcn-svelte和相关依赖

**Files:**
- Modify: `package.json`
- Modify: `pnpm-lock.yaml`

**Step 1: 安装shadcn-svelte依赖**

Run:
```bash
pnpm add shadcn-svelte bits-ui clsx tailwind-merge tailwind-variants
```

Expected: Dependencies installed successfully

**Step 2: 初始化shadcn-svelte**

Run:
```bash
npx shadcn-svelte@latest init
```

Expected:
- Creates `src/lib/components/ui/` directory
- Creates `components.json` config file
- Updates `tailwind.config.js`

**Step 3: 验证安装**

Run:
```bash
pnpm check
```

Expected: No TypeScript errors

**Step 4: Commit**

```bash
git add package.json pnpm-lock.yaml components.json tailwind.config.js src/lib/components/ui/
git commit -m "chore: install shadcn-svelte and initialize config

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 2: 安装shadcn组件

**Files:**
- Create: `src/lib/components/ui/button.svelte`
- Create: `src/lib/components/ui/card.svelte`
- Create: `src/lib/components/ui/dialog.svelte`
- Create: `src/lib/components/ui/dropdown-menu.svelte`
- Create: `src/lib/components/ui/separator.svelte`
- Create: `src/lib/components/ui/avatar.svelte`
- Create: `src/lib/components/ui/badge.svelte`
- Create: `src/lib/components/ui/input.svelte`
- Create: `src/lib/components/ui/textarea.svelte`
- Create: `src/lib/components/ui/scroll-area.svelte`
- Create: `src/lib/components/ui/popover.svelte`
- Create: `src/lib/components/ui/sidebar.svelte`

**Step 1: 安装Button组件**

Run:
```bash
npx shadcn-svelte@latest add button
```

Expected: Creates `src/lib/components/ui/button.svelte`

**Step 2: 安装Card组件**

Run:
```bash
npx shadcn-svelte@latest add card
```

Expected: Creates `src/lib/components/ui/card.svelte`

**Step 3: 安装Dialog组件**

Run:
```bash
npx shadcn-svelte@latest add dialog
```

Expected: Creates `src/lib/components/ui/dialog.svelte`

**Step 4: 安装DropdownMenu组件**

Run:
```bash
npx shadcn-svelte@latest add dropdown-menu
```

Expected: Creates `src/lib/components/ui/dropdown-menu.svelte`

**Step 5: 安装其余组件**

Run:
```bash
npx shadcn-svelte@latest add separator avatar badge input textarea scroll-area popover sidebar
```

Expected: All components created in `src/lib/components/ui/`

**Step 6: 验证安装**

Run:
```bash
pnpm check
```

Expected: No TypeScript errors

**Step 7: Commit**

```bash
git add src/lib/components/ui/
git commit -m "chore: add shadcn-svelte UI components

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Phase 2: 色彩系统和CSS重构

### Task 3: 替换色彩系统为oklch

**Files:**
- Modify: `src/app.css`

**Step 1: 备份当前app.css**

Run:
```bash
cp src/app.css src/app.css.backup
```

Expected: Backup created

**Step 2: 替换CSS变量为oklch色彩系统**

Replace the entire `:root` section in `src/app.css` with:

```css
@layer base {
  :root {
    /* 色彩系统 - oklch，纯中性灰（无色相） */
    --background: oklch(1 0 0);
    --foreground: oklch(0.145 0 0);
    --card: oklch(1 0 0);
    --card-foreground: oklch(0.145 0 0);
    --primary: oklch(0.205 0 0);
    --primary-foreground: oklch(0.985 0 0);
    --secondary: oklch(0.97 0 0);
    --secondary-foreground: oklch(0.205 0 0);
    --muted: oklch(0.97 0 0);
    --muted-foreground: oklch(0.556 0 0);
    --accent: oklch(0.97 0 0);
    --accent-foreground: oklch(0.205 0 0);
    --destructive: oklch(0.577 0.245 27.325);
    --border: oklch(0.922 0 0);
    --input: oklch(0.922 0 0);
    --ring: oklch(0.708 0 0);
    --sidebar: oklch(0.985 0 0);
    --sidebar-foreground: oklch(0.145 0 0);
    --sidebar-border: oklch(0.922 0 0);
    --surface: oklch(0.98 0 0);

    /* 圆角系统 */
    --radius: 0.625rem;
    --radius-sm: calc(var(--radius) - 4px);
    --radius-lg: var(--radius);
    --radius-xl: calc(var(--radius) + 4px);
    --radius-2xl: calc(var(--radius) + 8px);
    --radius-4xl: calc(var(--radius) + 16px);

    /* 侧边栏宽度 */
    --sidebar-width: 16rem;
    --sidebar-width-icon: 3rem;
  }
}
```

**Step 3: 删除所有.dark相关的CSS规则**

Remove all `.dark` selectors and their contents from `src/app.css`

**Step 4: 验证编译**

Run:
```bash
pnpm build
```

Expected: Build succeeds

**Step 5: Commit**

```bash
git add src/app.css
git commit -m "style: replace color system with oklch and remove dark theme

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 4: 创建工具函数

**Files:**
- Create: `src/lib/utils/cn.ts`
- Create: `src/lib/utils/date.ts`

**Step 1: 创建cn工具函数**

Create `src/lib/utils/cn.ts`:

```typescript
import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

**Step 2: 创建date工具函数**

Create `src/lib/utils/date.ts`:

```typescript
import type { Conversation } from '$lib/types';

export function groupConversationsByTime(conversations: Conversation[]) {
  const now = new Date();
  const today = startOfDay(now);
  const yesterday = startOfDay(subDays(now, 1));
  const last7Days = startOfDay(subDays(now, 7));
  const last30Days = startOfDay(subDays(now, 30));

  return {
    today: conversations.filter((c) => new Date(c.updatedAt) >= today),
    yesterday: conversations.filter((c) => {
      const date = new Date(c.updatedAt);
      return date >= yesterday && date < today;
    }),
    last7Days: conversations.filter((c) => {
      const date = new Date(c.updatedAt);
      return date >= last7Days && date < yesterday;
    }),
    last30Days: conversations.filter((c) => {
      const date = new Date(c.updatedAt);
      return date >= last30Days && date < last7Days;
    }),
    older: conversations.filter((c) => new Date(c.updatedAt) < last30Days),
  };
}

export function formatRelativeTime(date: string): string {
  const now = new Date();
  const target = new Date(date);
  const diffMs = now.getTime() - target.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return 'Just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return target.toLocaleDateString();
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
```

**Step 3: 验证编译**

Run:
```bash
pnpm check
```

Expected: No TypeScript errors

**Step 4: Commit**

```bash
git add src/lib/utils/cn.ts src/lib/utils/date.ts
git commit -m "feat: add cn and date utility functions

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Phase 3: 核心组件重写

### Task 5: 重写MessageBubble组件

**Files:**
- Modify: `src/lib/components/chat/MessageBubble.svelte`

**Step 1: 读取当前MessageBubble实现**

Run:
```bash
cat src/lib/components/chat/MessageBubble.svelte
```

Expected: See current implementation

**Step 2: 重写MessageBubble组件**

Replace entire content of `src/lib/components/chat/MessageBubble.svelte` with:

```svelte
<script lang="ts">
  import type { Message } from '$lib/types';
  import { renderMarkdown } from '$lib/utils/markdown';

  type Props = {
    message: Message;
  };

  let { message }: Props = $props();

  const isUser = message.role === 'user';
  const markdownContent = $derived(
    message.role === 'assistant' ? renderMarkdown(message.content) : message.content,
  );
</script>

{#if isUser}
  <!-- 用户消息：右对齐气泡，浅灰背景 -->
  <div class="group flex w-full max-w-[95%] ml-auto justify-end">
    <div class="flex w-fit max-w-full flex-col gap-2 rounded-lg bg-secondary px-4 py-3 text-sm text-foreground">
      {message.content}
    </div>
  </div>
{:else}
  <!-- 助手消息：左对齐纯文本，无背景 -->
  <div class="group flex w-full max-w-[95%]">
    <div class="flex w-fit max-w-full flex-col gap-2 text-sm text-foreground">
      {@html markdownContent}
    </div>
  </div>
{/if}
```

**Step 3: 验证编译**

Run:
```bash
pnpm check
```

Expected: No TypeScript errors

**Step 4: Commit**

```bash
git add src/lib/components/chat/MessageBubble.svelte
git commit -m "refactor: rewrite MessageBubble with shadcn style

- User messages: right-aligned bubble with bg-secondary
- Assistant messages: left-aligned plain text, no background

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 6: 重写InputArea组件

**Files:**
- Modify: `src/lib/components/chat/InputArea.svelte`

**Step 1: 读取当前InputArea实现**

Run:
```bash
cat src/lib/components/chat/InputArea.svelte
```

Expected: See current implementation

**Step 2: 重写InputArea为InputGroup结构**

Replace entire content of `src/lib/components/chat/InputArea.svelte` with:

```svelte
<script lang="ts">
  import { api } from '$lib/utils/invoke';
  import type { ChatEvent } from '$lib/types';
  import { Channel } from '@tauri-apps/api/core';

  type Props = {
    conversationId: string;
    modelId: string;
    onEvent?: (event: ChatEvent) => void;
  };

  let { conversationId, modelId, onEvent }: Props = $props();

  let message = $state('');
  let sending = $state(false);

  async function handleSend(e: Event) {
    e.preventDefault();
    if (!message.trim() || sending) return;

    const content = message;
    message = '';
    sending = true;

    try {
      const channel = new Channel<ChatEvent>();
      channel.onmessage = (event) => {
        onEvent?.(event);
      };

      await api.sendMessage(conversationId, content, modelId, channel);
    } catch (error) {
      console.error('Failed to send message:', error);
    } finally {
      sending = false;
    }
  }
</script>

<form class="w-full p-4" on:submit={handleSend}>
  <!-- InputGroup容器 -->
  <div
    class="group/input-group relative flex w-full items-center rounded-md border border-input shadow-xs
           has-[textarea:focus-visible]:border-ring has-[textarea:focus-visible]:ring-ring/50 has-[textarea:focus-visible]:ring-[3px]"
  >
    <!-- 自适应textarea -->
    <textarea
      bind:value={message}
      disabled={sending}
      class="flex-1 resize-none rounded-none border-0 bg-transparent py-3 px-4 shadow-none focus-visible:ring-0
             field-sizing-content min-h-16 max-h-48 text-sm"
      placeholder="What would you like to know?"
      on:keydown={(e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
          e.preventDefault();
          handleSend(e);
        }
      }}
    />

    <!-- 发送按钮：完全圆形 -->
    <button
      type="submit"
      disabled={!message.trim() || sending}
      class="size-8 rounded-4xl bg-primary text-primary-foreground hover:bg-primary/80 disabled:opacity-50 m-2 flex items-center justify-center"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="m22 2-7 20-4-9-9-4Z" />
        <path d="M22 2 11 13" />
      </svg>
    </button>
  </div>
</form>
```

**Step 3: 验证编译**

Run:
```bash
pnpm check
```

Expected: No TypeScript errors

**Step 4: Commit**

```bash
git add src/lib/components/chat/InputArea.svelte
git commit -m "refactor: rewrite InputArea as InputGroup structure

- Border + shadow-xs container
- Focus ring effect
- Auto-resize textarea with field-sizing-content
- Fully rounded send button

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---


### Task 7: 改造ConversationList组件（加时间分组）

**Files:**
- Modify: `src/lib/components/sidebar/ConversationList.svelte`

**Step 1: 读取当前ConversationList实现**

Run:
```bash
cat src/lib/components/sidebar/ConversationList.svelte
```

Expected: See current implementation

**Step 2: 改造ConversationList加入时间分组**

Replace entire content with time-grouped version using shadcn Sidebar components

**Step 3: 验证编译**

Run: `pnpm check`
Expected: No TypeScript errors

**Step 4: Commit**

```bash
git add src/lib/components/sidebar/ConversationList.svelte
git commit -m "refactor: add time-based grouping to ConversationList

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Phase 4: 新组件创建

### Task 8: 创建UserMenu组件

**Files:**
- Create: `src/lib/components/sidebar/UserMenu.svelte`

**Step 1: 创建UserMenu组件**

Create component with Avatar + DropdownMenu + Settings Dialog

**Step 2: 验证编译**

Run: `pnpm check`
Expected: No TypeScript errors

**Step 3: Commit**

```bash
git add src/lib/components/sidebar/UserMenu.svelte
git commit -m "feat: create UserMenu component with settings dialog

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 9: 创建AppSidebar组件

**Files:**
- Create: `src/lib/components/sidebar/AppSidebar.svelte`

**Step 1: 创建AppSidebar组件**

Create component combining ModelSelector + ConversationList + UserMenu

**Step 2: 验证编译**

Run: `pnpm check`
Expected: No TypeScript errors

**Step 3: Commit**

```bash
git add src/lib/components/sidebar/AppSidebar.svelte
git commit -m "feat: create AppSidebar component

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 10: 创建ChatArea组件

**Files:**
- Create: `src/lib/components/chat/ChatArea.svelte`

**Step 1: 创建ChatArea组件**

Create component combining MessageList + InputArea

**Step 2: 验证编译**

Run: `pnpm check`
Expected: No TypeScript errors

**Step 3: Commit**

```bash
git add src/lib/components/chat/ChatArea.svelte
git commit -m "feat: create ChatArea component

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Phase 5: 主页面重构

### Task 11: 重构+page.svelte使用新组件

**Files:**
- Modify: `src/routes/+page.svelte`

**Step 1: 读取当前+page.svelte**

Run: `cat src/routes/+page.svelte`
Expected: See current implementation

**Step 2: 重构+page.svelte**

Replace with SidebarProvider + AppSidebar + ChatArea layout

**Step 3: 验证编译**

Run: `pnpm check`
Expected: No TypeScript errors

**Step 4: 测试构建**

Run: `pnpm build`
Expected: Build succeeds

**Step 5: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "refactor: rebuild main page with shadcn layout

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Phase 6: ModelSelector重构

### Task 12: 重构ModelSelector为DropdownMenu

**Files:**
- Modify: `src/lib/components/chat/ModelSelector.svelte`

**Step 1: 读取当前ModelSelector**

Run: `cat src/lib/components/chat/ModelSelector.svelte`
Expected: See current implementation

**Step 2: 重构为shadcn DropdownMenu**

Replace with DropdownMenu-based implementation

**Step 3: 验证编译**

Run: `pnpm check`
Expected: No TypeScript errors

**Step 4: Commit**

```bash
git add src/lib/components/chat/ModelSelector.svelte
git commit -m "refactor: rebuild ModelSelector as shadcn DropdownMenu

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Phase 7: 状态管理简化

### Task 13: 简化ui.svelte.ts（移除theme）

**Files:**
- Modify: `src/lib/stores/ui.svelte.ts`

**Step 1: 读取当前ui.svelte.ts**

Run: `cat src/lib/stores/ui.svelte.ts`
Expected: See current implementation with theme state

**Step 2: 移除theme相关状态**

Keep only sidebarOpen state, remove theme

**Step 3: 验证编译**

Run: `pnpm check`
Expected: No TypeScript errors

**Step 4: Commit**

```bash
git add src/lib/stores/ui.svelte.ts
git commit -m "refactor: remove theme state from ui store

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Phase 8: 额外功能组件重构

### Task 14: 重构ProviderSettings为Dialog内容

**Files:**
- Modify: `src/lib/components/settings/ProviderSettings.svelte`

**Step 1: 读取当前ProviderSettings**

Run: `cat src/lib/components/settings/ProviderSettings.svelte`
Expected: See current implementation

**Step 2: 重构使用shadcn Card**

Update to use Card components for provider list

**Step 3: 验证编译**

Run: `pnpm check`
Expected: No TypeScript errors

**Step 4: Commit**

```bash
git add src/lib/components/settings/ProviderSettings.svelte
git commit -m "refactor: rebuild ProviderSettings with shadcn Card

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Phase 9: 最终验证和测试

### Task 15: 完整编译和运行测试

**Files:**
- None (verification only)

**Step 1: 清理并重新构建**

Run: `pnpm clean && pnpm install && pnpm build`
Expected: Build succeeds with no errors

**Step 2: 运行TypeScript检查**

Run: `pnpm check`
Expected: 0 errors, 0 warnings

**Step 3: 运行Rust测试**

Run: `cd src-tauri && cargo test`
Expected: All tests pass

**Step 4: 启动开发服务器（手动测试）**

Run: `pnpm tauri dev`

Expected:
- App launches successfully
- Sidebar shows model selector + time-grouped conversations + user menu
- Messages display correctly (user=bubble, assistant=plain text)
- Input area has border + shadow + focus ring
- Settings dialog opens from user menu
- All existing features work (streaming, persistence, etc.)

**Step 5: 最终commit**

```bash
git add -A
git commit -m "chore: final verification and cleanup

All shadcn UI redesign tasks completed

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## 成功标准验证清单

运行 `pnpm tauri dev` 后，手动验证以下项目：

- [ ] UI视觉效果与 shadcn.io/ai/chatbot 高度一致
- [ ] 只有亮色主题，无暗色主题
- [ ] 侧边栏有model selector + 时间分组对话列表 + 用户菜单
- [ ] 消息气泡：用户=浅灰气泡，助手=纯文本无背景
- [ ] 输入框为InputGroup结构，有border + shadow + focus ring
- [ ] 设置为Dialog弹窗，不是全屏页面
- [ ] 所有现有功能正常工作
- [ ] 流式响应、消息持久化、provider管理等核心功能不受影响
- [ ] 编译通过，无TypeScript错误
- [ ] 运行 `pnpm tauri dev` 可以正常使用

---

## 注意事项

1. **shadcn-svelte兼容性**：如果某个组件和Svelte 5 runes冲突，参考shadcn/ui v4源码手写
2. **CSS新特性**：`field-sizing-content` 需要较新的浏览器支持
3. **Tauri Channel**：确保Channel事件处理正确
4. **时间分组**：确保时区处理正确
5. **性能**：大量对话时，考虑虚拟滚动优化

---

## 后续优化（可选）

1. 添加对话搜索功能
2. 添加对话重命名功能
3. 添加对话删除确认Dialog
4. 优化MessageList的虚拟滚动
5. 添加键盘快捷键
6. 添加CompareView/SearchPanel/AssistantList的shadcn风格重构

