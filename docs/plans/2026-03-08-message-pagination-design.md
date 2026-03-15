# Orion Chat 消息分页与顶部自动加载设计

**日期**: 2026-03-08
**状态**: Approved
**目标**: 将会话消息加载改为分页模式，默认只加载最近 100 条消息；用户滚动到顶部时自动加载更早消息，并保持当前视口位置稳定不跳动。

## 背景

`docs/optimization.md` 当前列出了五项优化方向：
- 给 `MessageList` 上虚拟列表
- 流式阶段取消 Markdown 解析，结束后再解析
- `getMessages()` 改分页，先只取最近 100 条
- 删掉大多数“操作后全量 reload”
- 把超长 paste 内容外置存储

结合当前实现，这五项中最适合作为第一步的是“消息分页”：
- 当前 `get_messages` 后端命令始终返回整个会话的全部消息
- 前端页面层在很多操作后会重新调用 `api.getMessages(conversationId)`，导致长会话被反复全量载入
- `MessageList` 会把整个消息数组完整渲染出来，Assistant 消息还会进一步触发 Markdown 解析
- 因此当前的性能压力不是单点造成，而是“全量查询 + 全量传输 + 全量渲染 + Markdown 解析”叠加造成的

本轮优化的边界确认如下：
- 首屏只加载最近 `100` 条消息
- 顶部触顶时自动加载更早消息
- 加载更早消息后，用户当前视口位置必须保持稳定，不出现明显跳动
- 本轮先不引入虚拟列表
- 本轮不改变 Assistant、压缩、消息版本等已有业务规则

## 方案对比

### 方案 A：基于锚点消息的 before 分页（选定）

**做法**：
- 后端为消息查询增加分页参数：`limit` 与 `beforeMessageId`
- 不使用 offset，而是查询“某条消息之前的最近 N 条消息”
- 首次进入会话时，不传 `beforeMessageId`，返回最近 `100` 条
- 向上触顶时，传入当前最早消息的 `id`，查询这条消息之前的 `100` 条
- 后端返回结果仍保持按时间正序排列，前端将新页插入到已有数组头部

**优点**：
- 与当前按 `created_at ASC, rowid ASC` 的消息顺序兼容
- 对删除、恢复、版本切换后页边界的稳定性比 offset 更好
- 改动集中，利于先验证收益

**缺点**：
- 需要新增分页查询函数与前端分页状态管理
- 需要处理好“插入头部后视口保持不动”的滚动锚定逻辑

### 方案 B：offset 分页

**不选原因**：
- 删除消息、恢复消息、切换 active version 后，offset 容易漂移
- 数据变化后可能出现重复页、漏页或滚动体验不稳定

### 方案 C：继续全量查询，只在前端裁剪显示最近 100 条

**不选原因**：
- 无法真正降低查询、传输和内存占用
- 只减少了显示数量，不符合本轮优化目标

## 数据接口设计

### 后端查询接口

保持现有 `get_messages` 命令名可继续使用，但其返回结构改为分页结果：
- `messages`: 当前这一页的消息数组
- `hasMore`: 是否还存在更早消息

请求参数：
- `conversationId: string`
- `limit?: number`，默认 `100`
- `beforeMessageId?: string | null`

查询语义：
- **首次加载**：`beforeMessageId = null`，返回“该会话最近 `limit` 条消息”
- **加载更早消息**：`beforeMessageId = 当前前端最早消息的 id`，返回“该消息之前最近 `limit` 条消息”

返回结果中的 `messages` 必须保持为“旧 → 新”的顺序，这样前端只需执行数组头插，不需要再做倒序转换。

### 查询实现约束

消息分页查询必须继续遵守现有消息可见性规则：
- 只返回 `deleted_at IS NULL`
- 只返回 `is_active_version = 1`
- 顺序仍为 `created_at ASC, rowid ASC`

`hasMore` 的计算逻辑应以“当前页最早消息之前是否还有可见消息”为准，而不是用简单的 `messages.len() == limit` 代替，以免边界情况下误判。

## 前端状态设计

页面层新增分页状态：
- `hasMoreMessages: boolean`
- `isLoadingMoreMessages: boolean`
- `pageSize = 100`
- `initialLoadDone: boolean`（可选，用于避免未完成首屏加载时触发顶部分页）

派生信息：
- `oldestMessageId = messages[0]?.id ?? null`

状态规则：
- 切换会话时重置分页状态
- 首屏加载完成后，如果消息不足一页，则 `hasMoreMessages = false`
- 触顶加载进行中时，不允许重复请求
- 正在流式生成或自动压缩时，不触发顶部加载

## UI 与滚动行为设计

### 首屏行为

- 首次进入会话，只请求最近 `100` 条消息
- `MessageList` 初次渲染完成后保持滚动条在底部，符合聊天产品预期

### 顶部自动加载

`MessageList` 负责监听滚动容器：
- 当 `scrollTop` 接近 `0`，且满足 `hasMoreMessages && !isLoadingMoreMessages` 时，触发 `onLoadOlder()`
- 建议保留一个小阈值，而不是强依赖严格等于 `0`

### 视口锚定

为避免插入新消息页后界面跳动，采用 `scrollHeight` 差值补偿：
- 触发加载前记录 `prevScrollHeight` 与 `prevScrollTop`
- 新页插入头部并完成 DOM 更新后，读取新的 `scrollHeight`
- 将 `scrollTop` 设置为 `prevScrollTop + (newScrollHeight - prevScrollHeight)`

这样用户看到的内容位置保持不变，只是列表顶部悄悄扩展出更多旧消息。

## 与现有功能的兼容性

### 1. 流式回复

分页不能打断现有“流式阶段自动滚底”的核心体验：
- 正在流式生成时不触发顶部自动加载
- 若未来引入“用户手动离底部后不再自动滚底”，可在下一轮统一优化，不纳入本轮范围

### 2. 自动上下文压缩

自动压缩完成后，当前会话消息数可能骤减为一条摘要消息：
- 前端应直接用压缩接口返回的消息结果重置 `messages`
- 同时重置分页状态，重新计算 `hasMoreMessages`
- 不应在压缩失败时回退到“全量 reload 全会话”

### 3. 删除 / 恢复 / 重发 / 切版本

本轮不强制把所有“全量 reload”全部消除，但要求：
- 若某个交互暂时仍需重新拉取消息，也应只重新拉取“最近一页”
- 不再默认获取整个会话全部消息

### 4. Assistant prompt 与压缩摘要顺序

本轮分页不改变后端请求构建逻辑：
- 运行时仍由 `build_request_messages(...)` 在请求层动态前置 `assistant.system_prompt`
- 压缩后的摘要消息继续作为历史消息保留在 prompt 之后
- 即继续保证顺序为：`assistant prompt → summary → 后续消息`

## 风险与缓解

### 风险 1：插入旧消息后滚动跳动

**缓解**：
- 统一把滚动补偿逻辑封装在 `MessageList.svelte`
- 先保证“顶部加载不跳”，再考虑后续虚拟列表

### 风险 2：触顶连续请求导致重复页

**缓解**：
- 增加 `isLoadingMoreMessages` 防重入
- 使用最早消息 `id` 作为唯一翻页锚点

### 风险 3：现有大量 `getMessages()` 调用点遗漏

**缓解**：
- 先统一封装页面层的“刷新当前会话最近一页”入口
- 再逐个替换零散的 `api.getMessages(activeConversationId)`

## 验证策略

### 后端

补充分页查询测试，至少覆盖：
- 首次加载只返回最近 `100` 条
- 传入 `beforeMessageId` 时返回更早页
- `hasMore` 在边界条件下正确
- 查询仍排除 `deleted_at` 与非活动版本消息

### 前端

- `npm run check`
- 人工验证一个包含 200+ 消息的长会话：
  - 首次进入时只渲染最近一页
  - 滚到顶部自动加载旧消息
  - 加载后视口不跳
  - 自动压缩后分页状态正常重置

## 后续阶段建议

消息分页落地并验证收益后，建议下一批按以下顺序继续：
1. 流式阶段关闭 Markdown 解析，结束后再解析
2. 删掉大多数操作后的全量 reload，改成局部更新或分页刷新
3. 评估是否需要虚拟列表
4. 最后处理超长 paste 内容外置存储
