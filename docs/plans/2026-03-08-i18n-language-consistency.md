# Orion Chat 中英文本地化一致性 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 让 Orion Chat 在中文模式下显示中文界面文案，在英文模式下显示英文界面文案，并确保语言切换在全局 UI 中即时生效。

**Architecture:** 新增一个共享的前端语言状态模块，统一负责读取/保存当前语言和提供通用翻译辅助方法。聊天区、侧边栏、通用 UI 组件与设置页全部改为消费同一语言源，避免局部页面各自维护语言状态导致中英混杂。

**Tech Stack:** Svelte 5 runes、Tauri plugin store、TypeScript、SvelteKit

---

### Task 1: 建立共享语言状态与基础翻译工具

**Files:**
- Create: `src/lib/stores/i18n.svelte.ts`
- Modify: `src/routes/+layout.svelte`
- Modify: `src/lib/utils/date.ts`

**Step 1: 新增共享语言状态模块**

在 `src/lib/stores/i18n.svelte.ts` 中提供：
- 当前语言状态（`zh` / `en`）
- 初始化语言（从 `settings.json` 读取）
- 切换语言并持久化
- 常用界面词条与格式化辅助函数（例如相对时间、粘贴块长度标签、会话分组标题、角色标签）

**Step 2: 在应用入口加载语言设置**

在 `src/routes/+layout.svelte` 中调用语言初始化逻辑，确保所有页面首次渲染时共享同一语言源。

**Step 3: 让时间分组与相对时间支持语言切换**

把 `src/lib/utils/date.ts` 中固定英文时间文案改为接收语言参数或复用共享语言模块，避免侧边栏在中文模式仍显示英文时间标签。

**Step 4: 运行静态检查验证基础模块可用**

Run: `npm run check`
Expected: `svelte-check` 成功结束，无新增类型错误。

### Task 2: 替换聊天区、侧边栏与通用组件硬编码文案

**Files:**
- Modify: `src/routes/+page.svelte`
- Modify: `src/routes/settings/+page.svelte`
- Modify: `src/lib/components/chat/InputArea.svelte`
- Modify: `src/lib/components/chat/MessageBubble.svelte`
- Modify: `src/lib/components/chat/MessageList.svelte`
- Modify: `src/lib/components/chat/ModelParamsPopover.svelte`
- Modify: `src/lib/components/chat/ModelSelector.svelte`
- Modify: `src/lib/components/sidebar/AssistantList.svelte`
- Modify: `src/lib/components/sidebar/ConversationList.svelte`
- Modify: `src/lib/components/sidebar/SearchPanel.svelte`
- Modify: `src/lib/components/sidebar/UserMenu.svelte`
- Modify: `src/lib/components/ui/dialog/dialog-content.svelte`
- Modify: `src/lib/components/ui/sheet/sheet-content.svelte`
- Modify: `src/lib/components/ui/sidebar/sidebar-rail.svelte`
- Modify: `src/lib/components/ui/sidebar/sidebar-trigger.svelte`
- Modify: `src/lib/components/ui/sidebar/sidebar.svelte`

**Step 1: 聊天区文案接入共享翻译**

把输入框占位符、发送/停止按钮、消息空状态、消息操作按钮、错误提示、思考折叠按钮、参数面板标签、模型选择默认文案等改为来自共享语言模块。

**Step 2: 侧边栏文案接入共享翻译**

把新建对话、会话分组标题、加载态、空状态、上下文菜单、搜索面板、助手列表、用户菜单等硬编码文案替换为共享翻译。

**Step 3: 通用可访问性文案接入共享翻译**

把 `Close`、`Toggle Sidebar`、移动端 `Sidebar` 标题/描述等 `aria-label` / `sr-only` 文案也替换成对应语言，保证无障碍文本不再固定英文。

**Step 4: 运行静态检查验证组件改动**

Run: `npm run check`
Expected: `svelte-check` 成功结束，无新增类型错误。

### Task 3: 收口设置页语言源并清理残留混合文案

**Files:**
- Modify: `src/lib/components/settings/ProviderSettings.svelte`

**Step 1: 设置页改为使用共享语言状态**

移除设置页内部独立的语言持久化来源，让语言选择控件直接读写共享语言状态，确保切换后聊天页、侧边栏、设置页同步更新。

**Step 2: 修复设置页剩余硬编码文案**

处理设置页导航分组、默认开关状态、GitHub 按钮、处理中状态、确认弹窗、成功/失败提示、示例占位符等残留中英硬编码，确保设置页本身也完全遵循当前语言。

**Step 3: 重新核对实现范围**

逐项核对以下页面/组件：
- 首页空状态
- 聊天输入区
- 消息气泡操作
- 参数面板
- 模型选择器
- 会话列表与时间分组
- 搜索面板
- 助手列表
- 用户菜单
- 设置页导航与各设置分区
- 通用关闭/侧边栏无障碍文案

**Step 4: 运行最终验证**

Run: `npm run check`
Expected: `svelte-check` 成功结束，无新增类型错误。
