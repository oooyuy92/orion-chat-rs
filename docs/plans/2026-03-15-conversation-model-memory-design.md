# Conversation Model Memory Design

**背景**
- `conversations` 表和前后端 `Conversation` 类型已经包含 `model_id / modelId` 字段。
- 当前聊天页在 [src/routes/+page.svelte](/Users/oooyuy/Github/orion-chat-rs/src/routes/+page.svelte) 中只维护一个全局 `currentModelId`。
- 用户手动切换模型时，前端没有把模型写回当前会话；切换会话时，也没有优先从 `conversation.modelId` 恢复。

**问题根因**
- 后端缺少更新会话模型的命令与数据库方法。
- 前端模型选择器只改变内存中的 `currentModelId`，没有持久化。
- 会话切换时仅根据 `assistant.modelId` 做同步，导致没有 assistant 默认模型的会话继续沿用上一个会话的模型。

## 方案对比

### 方案 A：只做前端临时缓存
- 优点：改动小。
- 缺点：刷新应用后丢失，和已有 `conversation.modelId` 字段重复。

### 方案 B：按会话持久化记忆（推荐）
- 用户手动切换模型时立即写回 `conversation.modelId`。
- 切换会话时按明确优先级恢复模型。
- 优点：和现有数据模型一致，重启后仍能记住。

### 方案 C：记全局最近模型
- 优点：实现最简单。
- 缺点：不符合“每个对话单独记忆”的需求。

**最终方案**
- 采用方案 B。
- 模型恢复优先级固定为：
  1. `conversation.modelId`
  2. `assistant.modelId`
  3. 第一个可用模型
- Assistant 绑定的默认模型只作为会话初始值，不覆盖用户后续手动切换。

## 数据流

### 1. 切换会话
- 页面层重新加载或读取当前会话后，优先读取 `conversation.modelId`。
- 若该模型不存在或为空，再回退到 `assistant.modelId`。
- 再没有时，回退到首个可用模型，避免继续沿用上一会话的模型。

### 2. 用户手动切换模型
- `ModelSelector` 选择后，将事件抛给页面层。
- 页面层更新当前 `currentModelId`，并调用新的 `update_conversation_model` 接口持久化。
- 本地 `conversations` 列表同步更新对应会话的 `modelId`，避免下一次刷新前状态不一致。

### 3. 选择 Assistant
- 若选中的 Assistant 绑定了默认模型，则把该模型作为当前会话模型，并同步写回 `conversation.modelId`。
- 若 Assistant 未绑定默认模型，则保留当前会话已有模型。

## 验证
- 新增前端契约测试，确保：
  - 前端 API 暴露 `updateConversationModel`
  - 页面层恢复模型时引用 `conversation.modelId`
  - 页面层在模型选择时调用 `api.updateConversationModel`
- 新增 Rust 单元测试，覆盖会话 `model_id` 更新。
- 运行定向 `node --test`、`cargo test` 和 `pnpm run check`。
