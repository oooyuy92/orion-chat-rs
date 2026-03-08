# Sidebar Search Mode Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在左栏“开始对话”按钮下加入搜索框，支持正文/paste 搜索并切换到搜索结果模式，点击结果后自动定位到命中消息。

**Architecture:** 后端新增结构化搜索结果命令，为消息正文和 paste 正文提供统一结果片段。前端在 `ConversationList` 中加入搜索框并切换渲染模式，同时把点击事件升级为可携带 `messageId` 的选择行为。页面层负责跨分页定位消息，`MessageList` 负责虚拟列表中的滚动与高亮。

**Tech Stack:** Rust + rusqlite FTS、Tauri commands、Svelte 5、TypeScript、Node `node:test` 契约测试、`cargo test`、`svelte-check`

---

### Task 1: 后端结构化搜索结果

**Files:**
- Modify: `src-tauri/src/db/messages.rs`
- Modify: `src-tauri/src/db/paste_blobs.rs`
- Modify: `src-tauri/src/commands/search.rs`
- Modify: `src-tauri/src/models/mod.rs`
- Create: `src-tauri/src/models/search.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/commands/search.rs`

**Step 1: Write the failing test**
- 为 `search_sidebar_results` 写两个最小测试：
  - 普通消息命中返回 `messageId + snippet`
  - paste 正文命中返回 paste 片段，而不是占位符原文

**Step 2: Run test to verify it fails**

Run: `cargo test sidebar_search --manifest-path src-tauri/Cargo.toml`
Expected: FAIL，因为命令与模型尚未存在。

**Step 3: Write minimal implementation**
- 新建 `SearchSidebarResult` 模型。
- 新增命令 `search_sidebar_results(query)`。
- `db::messages` 新增搜索片段查询。
- `db::paste_blobs` 新增按 query 返回 `message_id + snippet` 能力。
- 在命令层合并结果并按相关性 / 时间排序。

**Step 4: Run test to verify it passes**

Run: `cargo test sidebar_search --manifest-path src-tauri/Cargo.toml`
Expected: PASS

### Task 2: 前端搜索结果模式

**Files:**
- Modify: `src/lib/types/index.ts`
- Modify: `src/lib/utils/invoke.ts`
- Modify: `src/lib/components/sidebar/ConversationList.svelte`
- Modify: `src/lib/components/sidebar/AppSidebar.svelte`
- Test: `src/lib/components/sidebar/sidebarSearchMode.test.js`

**Step 1: Write the failing test**
- 为 `ConversationList.svelte` 写契约测试，断言：
  - 存在搜索框
  - 调用 `api.searchSidebarResults(`
  - 搜索状态下渲染搜索结果模式

**Step 2: Run test to verify it fails**

Run: `node --test src/lib/components/sidebar/sidebarSearchMode.test.js`
Expected: FAIL

**Step 3: Write minimal implementation**
- 在开始对话按钮下加入搜索框。
- 新增搜索状态、结果列表、300ms 防抖。
- 空查询显示原会话分组；非空显示搜索结果模式。
- 合并后端正文结果与前端标题/Assistant 名称模糊结果。

**Step 4: Run test to verify it passes**

Run: `node --test src/lib/components/sidebar/sidebarSearchMode.test.js`
Expected: PASS

### Task 3: 会话打开后自动定位到命中消息

**Files:**
- Modify: `src/routes/+page.svelte`
- Modify: `src/lib/components/chat/ChatArea.svelte`
- Modify: `src/lib/components/chat/MessageList.svelte`
- Test: `src/lib/components/chat/messageSearchFocus.test.js`

**Step 1: Write the failing test**
- 为 `MessageList.svelte` 和页面链路写契约测试，断言：
  - `MessageList` 接收 `focusedMessageId`
  - 页面层在选择搜索结果时会传递 `messageId`

**Step 2: Run test to verify it fails**

Run: `node --test src/lib/components/chat/messageSearchFocus.test.js`
Expected: FAIL

**Step 3: Write minimal implementation**
- 页面层增加待定位消息状态。
- 若目标消息未加载到当前页，则持续调用 `loadOlderMessages` 直到找到或没有更多消息。
- `MessageList` 增加目标消息滚动与高亮能力。

**Step 4: Run test to verify it passes**

Run: `node --test src/lib/components/chat/messageSearchFocus.test.js`
Expected: PASS

### Task 4: 全量验证

**Files:**
- Test: `src-tauri/src/commands/search.rs`
- Test: `src/lib/components/sidebar/sidebarSearchMode.test.js`
- Test: `src/lib/components/chat/messageSearchFocus.test.js`

**Step 1: Run focused backend tests**

Run: `cargo test sidebar_search --manifest-path src-tauri/Cargo.toml`
Expected: PASS

**Step 2: Run focused frontend tests**

Run: `node --test src/lib/components/sidebar/sidebarSearchMode.test.js src/lib/components/chat/messageSearchFocus.test.js`
Expected: PASS

**Step 3: Run project checks**

Run: `pnpm run check`
Expected: `svelte-check found 0 errors and 0 warnings`

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS
