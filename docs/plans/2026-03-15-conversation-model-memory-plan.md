# Conversation Model Memory Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 让每个会话都能记住自己选中的模型，并在切换会话后正确恢复。

**Architecture:** 在后端补齐 `conversation.model_id` 的更新命令，在前端把模型选择事件提升到页面层并持久化到当前会话。会话切换时统一按 `conversation.modelId > assistant.modelId > 第一个可用模型` 的优先级恢复，避免模型在不同会话间串用。

**Tech Stack:** Rust、Tauri commands、SQLite、Svelte 5、TypeScript、Node `node:test`、`svelte-check`

---

### Task 1: 为会话模型记忆补失败用例

**Files:**
- Create: `src/lib/components/chat/conversationModelMemory.test.js`
- Modify: `src-tauri/src/db/conversations.rs`

**Step 1: Write the failing test**
- 前端契约测试断言：
  - `src/lib/utils/invoke.ts` 暴露 `updateConversationModel`
  - `src/routes/+page.svelte` 使用 `conversation?.modelId`
  - `src/routes/+page.svelte` 调用 `api.updateConversationModel(`
- Rust 单元测试断言：
  - 更新会话 `model_id` 后，重新读取得到新值

**Step 2: Run test to verify it fails**

Run: `node --test src/lib/components/chat/conversationModelMemory.test.js`
Expected: FAIL，因为前端还没有会话模型更新与恢复链路。

Run: `cargo test test_update_model_binding --manifest-path src-tauri/Cargo.toml`
Expected: FAIL，因为数据库层尚无 `update_model`。

**Step 3: Write minimal implementation**
- 见 Task 2 和 Task 3。

**Step 4: Run test to verify it passes**

Run: `node --test src/lib/components/chat/conversationModelMemory.test.js`
Expected: PASS

Run: `cargo test test_update_model_binding --manifest-path src-tauri/Cargo.toml`
Expected: PASS

### Task 2: 补齐后端 conversation model 更新能力

**Files:**
- Modify: `src-tauri/src/db/conversations.rs`
- Modify: `src-tauri/src/commands/conversation.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/utils/invoke.ts`
- Modify: `src/lib/api/web/impl.ts`

**Step 1: Add database update helper**
- 在 `src-tauri/src/db/conversations.rs` 新增 `update_model(conn, id, model_id)`。

**Step 2: Add command**
- 在 `src-tauri/src/commands/conversation.rs` 新增 `update_conversation_model`。
- 对非空 `model_id` 做存在性校验。

**Step 3: Register and expose API**
- 在 `src-tauri/src/lib.rs` 注册命令。
- 在 `src/lib/utils/invoke.ts` 和 `src/lib/api/web/impl.ts` 增加对应前端方法。

**Step 4: Verify backend test**

Run: `cargo test test_update_model_binding --manifest-path src-tauri/Cargo.toml`
Expected: PASS

### Task 3: 前端持久化并恢复会话模型

**Files:**
- Modify: `src/lib/components/chat/ChatArea.svelte`
- Modify: `src/lib/components/chat/InputArea.svelte`
- Modify: `src/routes/+page.svelte`
- Test: `src/lib/components/chat/conversationModelMemory.test.js`

**Step 1: Plumb model selection event**
- `InputArea` 通过 `ModelSelector.onSelect` 向上抛出模型选择事件。
- `ChatArea` 再转发给页面层。

**Step 2: Persist model selection**
- 页面层新增 `handleModelSelect(modelId)`：
  - 更新 `currentModelId`
  - 调用 `api.updateConversationModel(activeConversationId, modelId)`
  - 同步本地 `conversations`

**Step 3: Restore model on conversation switch**
- 提炼模型恢复函数，优先读取 `conversation.modelId`，其次 `assistant.modelId`，最后首个可用模型。
- 在 `loadModels`、`loadAssistants`、`refreshConversationState`、`handleConversationSelect` 相关链路中统一使用。

**Step 4: Keep assistant default as initial value**
- `handleAssistantSelect` 选中带默认模型的 Assistant 时，同步把该模型写入当前 conversation。

**Step 5: Verify focused frontend test**

Run: `node --test src/lib/components/chat/conversationModelMemory.test.js`
Expected: PASS

### Task 4: 全量前端校验

**Files:**
- Modify: `src/routes/+page.svelte`
- Modify: `src/lib/components/chat/ChatArea.svelte`
- Modify: `src/lib/components/chat/InputArea.svelte`

**Step 1: Run focused tests**

Run: `node --test src/lib/components/chat/conversationModelMemory.test.js src/lib/components/chat/messageSearchFocus.test.js`
Expected: PASS

**Step 2: Run type and template checks**

Run: `pnpm run check`
Expected: `svelte-check found 0 errors and 0 warnings`
