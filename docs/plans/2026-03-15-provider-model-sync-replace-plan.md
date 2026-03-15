# Provider Model Sync Replace Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 让 provider 模型同步按本次远端 `synced` 集合覆盖，而不是把历史同步结果持续累积。

**Architecture:** 在 `fetch_models` 背后增加一个数据库协调函数，统一负责 upsert 本次远端模型、清理过期未引用的 `synced` 模型，并保留 `manual` 模型与仍被引用的旧模型。

**Tech Stack:** Rust、SQLite、Tauri commands、Node `node:test`、`svelte-check`

---

### Task 1: 为同步替换策略补失败测试

**Files:**
- Modify: `src-tauri/src/commands/provider.rs`

**Step 1: Write the failing test**
- 新增单元测试断言：
  - 旧 `synced` 模型在本次结果缺席时会被删除
  - `manual` 模型保留
  - 被 `conversations` / `assistants` 引用的旧 `synced` 模型保留

**Step 2: Run test to verify it fails**

Run: `cargo test replace_synced_models --manifest-path src-tauri/Cargo.toml`
Expected: FAIL，因为协调函数尚未实现。

**Step 3: Write minimal implementation**
- 见 Task 2。

**Step 4: Run test to verify it passes**

Run: `cargo test replace_synced_models --manifest-path src-tauri/Cargo.toml`
Expected: PASS

### Task 2: 实现 provider 同步集合替换

**Files:**
- Modify: `src-tauri/src/commands/provider.rs`

**Step 1: Add helper**
- 新增 `replace_synced_models_for_provider(...)`，在一个数据库事务/闭包中完成 upsert + prune。

**Step 2: Wire fetch_models**
- 让 `fetch_models` 调用该 helper，而不是只做单纯 upsert。

**Step 3: Keep manual and referenced stale models**
- 清理 SQL 只针对：
  - `provider_id = ?`
  - `source = 'synced'`
  - 不在本次 fetched ids
  - 且未被 `assistants` / `conversations` 引用

**Step 4: Verify**

Run: `cargo test replace_synced_models --manifest-path src-tauri/Cargo.toml`
Expected: PASS

### Task 3: 回归检查

**Files:**
- Modify: `src-tauri/src/commands/provider.rs`
- Modify: `src/lib/components/settings/ProviderSettings.svelte`（如果需要消息文案调整）

**Step 1: Run focused Rust test**

Run: `cargo test provider --manifest-path src-tauri/Cargo.toml`
Expected: 相关 provider 测试通过

**Step 2: Run frontend type check**

Run: `pnpm run check`
Expected: `svelte-check found 0 errors and 0 warnings`
