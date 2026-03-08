# Message Virtualization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为聊天消息列表增加自实现的动态高度虚拟化，降低长对话和多对话场景下的前端渲染与内存开销，同时保持顶部分页加载与底部跟随体验。

**Architecture:** 新增一个纯前端窗口计算 helper，负责高度缓存、窗口区间和 spacer 计算；`MessageList.svelte` 负责滚动状态、`ResizeObserver` 测量和 DOM 渲染窗口；页面层继续保留现有消息加载和流式处理逻辑，尽量不动业务协议。

**Tech Stack:** Svelte 5、TypeScript/ESM、Node 原生 `node:test`、Tauri 前端。

---

### Task 1: 提取并测试虚拟窗口计算 helper

**Files:**
- Create: `src/lib/components/chat/messageVirtualization.js`
- Create: `src/lib/components/chat/messageVirtualization.test.js`

**Step 1: Write the failing test**

为 helper 写 Node 原生测试，至少覆盖：
- 当消息总数较少时返回全量渲染窗口
- 当消息较多时根据 `scrollTop`、`viewportHeight`、`overscan` 返回正确窗口
- 正确计算 `topSpacerHeight` 与 `bottomSpacerHeight`
- 未测量消息使用默认估算高度

**Step 2: Run test to verify it fails**

Run: `node --test src/lib/components/chat/messageVirtualization.test.js`
Expected: FAIL，提示 helper 模块或目标函数不存在。

**Step 3: Write minimal implementation**

在 `messageVirtualization.js` 中实现最小可用函数，例如：
- `estimateTotalHeight(...)`
- `getMeasuredHeight(...)`
- `calculateVirtualWindow(...)`

要求：
- 仅处理纯数据，不依赖 DOM
- 高度缓存以 `Map<string, number>` 形式传入
- 结果包含 `startIndex`、`endIndex`、`topSpacerHeight`、`bottomSpacerHeight`

**Step 4: Run test to verify it passes**

Run: `node --test src/lib/components/chat/messageVirtualization.test.js`
Expected: PASS

**Step 5: Commit**

Run:
```bash
git add src/lib/components/chat/messageVirtualization.js src/lib/components/chat/messageVirtualization.test.js
git commit -m "test: add virtual window helper"
```

### Task 2: 在 MessageList 中接入窗口化渲染与动态测量

**Files:**
- Modify: `src/lib/components/chat/MessageList.svelte`
- Reference: `src/lib/components/chat/MessageBubble.svelte`

**Step 1: Write the failing test**

基于 Task 1 helper 增加一条更贴近 UI 的失败用例，覆盖“滚动到中部时不会继续渲染列表头尾所有消息”的窗口边界行为。

**Step 2: Run test to verify it fails**

Run: `node --test src/lib/components/chat/messageVirtualization.test.js`
Expected: FAIL，说明窗口边界与当前 helper/配置不一致。

**Step 3: Write minimal implementation**

在 `MessageList.svelte` 中：
- 引入 helper 计算可见窗口
- 维护 `scrollTop`、`viewportHeight`、`isNearBottom`
- 仅渲染窗口内消息
- 增加 top/bottom spacer
- 为每个已渲染项注册 `ResizeObserver` 并更新高度缓存
- 保留现有顶部自动加载旧消息逻辑
- 继续支持 prepend 锚点恢复
- 仅在 near-bottom 时对新增/流式消息滚底

**Step 4: Run test to verify it passes**

Run: `node --test src/lib/components/chat/messageVirtualization.test.js`
Expected: PASS

**Step 5: Commit**

Run:
```bash
git add src/lib/components/chat/MessageList.svelte src/lib/components/chat/messageVirtualization.js src/lib/components/chat/messageVirtualization.test.js
git commit -m "feat: virtualize message list rendering"
```

### Task 3: 做集成校验并收口边界行为

**Files:**
- Modify: `src/lib/components/chat/ChatArea.svelte` (only if prop wiring needs adjustment)
- Modify: `src/routes/+page.svelte` (only if paging hooks need adjustment)
- Reference: `src/lib/components/chat/MessageBubble.svelte`

**Step 1: Write the failing test**

如果 Task 2 暴露新的窗口计算边界，再先把对应 helper 回归用例补进 `messageVirtualization.test.js`。

**Step 2: Run test to verify it fails**

Run: `node --test src/lib/components/chat/messageVirtualization.test.js`
Expected: FAIL，准确反映边界问题。

**Step 3: Write minimal implementation**

仅在确有必要时微调：
- `ChatArea.svelte` 的传参
- `+page.svelte` 的加载后状态同步

重点保证：
- 滚动到顶部自动加载仍然有效
- prepend 后视口不跳动
- 新消息与流式消息在 near-bottom 时跟随到底部
- 用户已离开底部时不强制抢焦点

**Step 4: Run test to verify it passes**

Run:
- `node --test src/lib/components/chat/messageVirtualization.test.js`
- `pnpm run check`

Expected:
- helper tests PASS
- `svelte-check` 0 errors / 0 warnings

**Step 5: Commit**

Run:
```bash
git add src/lib/components/chat/MessageList.svelte src/lib/components/chat/ChatArea.svelte src/routes/+page.svelte src/lib/components/chat/messageVirtualization.js src/lib/components/chat/messageVirtualization.test.js
git commit -m "refactor: stabilize virtualized chat scrolling"
```
