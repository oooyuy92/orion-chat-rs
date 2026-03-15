# Conversation Switch Streaming State Design

**背景**
- 当前聊天页在 `src/routes/+page.svelte` 中维护一个“当前显示中的会话状态”：`messages`、`streamingMessageId`、`lastPromptTokens` 等。
- 后台流式生成时，页面用 `messageCache` 临时保存非激活会话的视图状态，切回时再恢复。
- 实际切换会话时，左栏子组件会先通过双向绑定改写父组件的 `activeConversationId`，再调用选择回调，导致父组件保存“旧会话缓存”时已经拿到了新会话 ID。

**问题根因**
- 会话选择链路存在双写：
  - `ConversationList.svelte` 通过 `bind:activeId`
  - `AppSidebar.svelte` 通过 `bind:activeConversationId`
  - `+page.svelte` 又在 `handleConversationSelect` 里自行切换 `activeConversationId`
- 在流式生成期间从会话 A 切到会话 B 时，A 的 `messages` 可能被错误缓存到 B 的 key 下，切回后显示错乱。

## 方案对比

### 方案 A：保留双向绑定，在父组件里额外记录 previousActiveConversationId
- 优点：改动集中在页面。
- 缺点：继续保留多处状态写入点，后续容易再次出现竞态。

### 方案 B：让父组件独占 `activeConversationId`，侧边栏只上报选择事件（推荐）
- 优点：单向数据流明确，`handleConversationSelect` 能稳定拿到旧会话和新会话。
- 优点：与现有 `messageCache` 方案兼容，属于最小修复。
- 缺点：需要同步调整 `AppSidebar.svelte` 和 `ConversationList.svelte` 的传参方式。

### 方案 C：把所有会话视图状态重构为全局 per-conversation store
- 优点：架构上更干净。
- 缺点：超出本次最小修复范围，风险和改动面更大。

**最终方案**
- 采用方案 B。
- 父组件 `+page.svelte` 成为 `activeConversationId` 的唯一写入者。
- `AppSidebar.svelte` 和 `ConversationList.svelte` 只接收只读 active id，并通过 `onSelect(...)` 把目标会话通知给父组件。
- 保留现有 `messageCache` 结构，但修复会话选择时的 key 归属。

## 状态边界
- 持久化状态：继续由数据库按 `conversationId` 隔离，当前无需调整。
- 前端临时状态：至少视为“按会话归属”的状态，包括 `messages`、`streamingMessageId`、`lastPromptTokens`、`groupStreamingMessages`。
- 本次先修复选择链路，后续如继续演进，可再把这些状态正式收口为 `Map<conversationId, ViewState>`。

## 验证
- 新增前端契约测试，确保：
  - `AppSidebar.svelte` 不再对 `ConversationList.svelte` 使用 `bind:activeId`
  - 页面仍通过 `handleConversationSelect` 统一处理会话切换
- 运行定向 `node --test`
- 运行 `pnpm run check`
