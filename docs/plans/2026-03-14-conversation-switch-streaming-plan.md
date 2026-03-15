# Conversation Switch Streaming Fix Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 修复流式生成时切换会话导致消息列表串到另一会话的问题。

**Architecture:** 保持数据库与流式事件协议不变，只调整前端会话选择的数据流。`+page.svelte` 作为 `activeConversationId` 的唯一写入者，侧边栏组件只读当前 active id，并通过 `onSelect` 上报目标会话，避免旧会话缓存被写到新会话 key 下。

**Tech Stack:** Svelte 5、TypeScript、Node `node:test` 契约测试、`svelte-check`

---

### Task 1: 为会话选择所有权补失败用例

**Files:**
- Create: `src/lib/components/sidebar/conversationSelectionOwnership.test.js`
- Test: `src/lib/components/sidebar/conversationSelectionOwnership.test.js`

**Step 1: Write the failing test**
- 断言 `AppSidebar.svelte` 向 `ConversationList.svelte` 传入 `activeId={activeConversationId}`，而不是 `bind:activeId={activeConversationId}`。
- 断言 `+page.svelte` 仍通过 `onConversationSelect={handleConversationSelect}` 统一处理切换。

**Step 2: Run test to verify it fails**

Run: `node --test src/lib/components/sidebar/conversationSelectionOwnership.test.js`
Expected: FAIL，因为当前实现仍然使用双向绑定。

**Step 3: Write minimal implementation**
- 去掉 `AppSidebar.svelte` 中对子组件 `ConversationList` 的 `bind:activeId`。
- 保留 `activeConversationId` 到 `ConversationList` 的只读传递。

**Step 4: Run test to verify it passes**

Run: `node --test src/lib/components/sidebar/conversationSelectionOwnership.test.js`
Expected: PASS

### Task 2: 验证页面切换链路没有退化

**Files:**
- Modify: `src/lib/components/sidebar/AppSidebar.svelte`
- Modify: `src/routes/+page.svelte`
- Test: `src/lib/components/chat/messageSearchFocus.test.js`

**Step 1: Re-check page selection contract**
- 保持页面对 `handleConversationSelect` 的调用不变，确保消息定位和缓存恢复仍由页面层统一处理。

**Step 2: Run focused tests**

Run: `node --test src/lib/components/sidebar/conversationSelectionOwnership.test.js src/lib/components/chat/messageSearchFocus.test.js`
Expected: PASS

### Task 3: 全量前端校验

**Files:**
- Modify: `src/lib/components/sidebar/AppSidebar.svelte`
- Modify: `src/routes/+page.svelte`

**Step 1: Run project check**

Run: `pnpm run check`
Expected: `svelte-check found 0 errors and 0 warnings`
