# Message Virtualization Design

## Background

当前消息列表已经具备：
- 仅加载最近一页消息，滚动到顶部自动继续加载更旧消息
- 新消息/流式消息到来时默认滚到底部
- `MessageBubble` 高度是动态的，会受 Markdown 渲染、推理展开折叠、Paste 展开、版本切换等影响

随着单会话消息数继续增大，即使已经做了分页，前端仍会因为一次性渲染当前页内全部 DOM 节点而出现：
- 初次进入会话和切换会话时渲染成本偏高
- 多轮流式更新期间布局抖动放大
- 打开多个长对话时内存与布局开销偏高

## Goals

1. 为聊天消息列表增加自实现窗口化渲染，减少同时存在的消息 DOM 数量。
2. 保持现有交互不变：顶部自动加载更旧消息、底部跟随、流式增量、推理折叠、Paste 展开、版本切换都继续可用。
3. 支持动态高度消息，不要求固定行高。
4. 尽量把改动收敛在 `MessageList`，避免影响消息协议、业务逻辑和 Tauri 命令层。

## Non-Goals

1. 本批不引入第三方虚拟列表库。
2. 本批不改变后端分页协议。
3. 本批不重做 `MessageBubble` 内部结构。
4. 本批不额外实现消息搜索跳转或按索引定位。

## Approach

### 1. 视口窗口化

在 `MessageList.svelte` 内维护：
- `viewportHeight`
- `scrollTop`
- `startIndex`
- `endIndex`
- `overscan`
- `topSpacerHeight`
- `bottomSpacerHeight`

渲染时只输出 `[startIndex, endIndex]` 范围内的消息项，并通过上下 spacer 占位来保持整体滚动高度近似不变。

### 2. 动态高度缓存

新增一个纯前端 helper 模块，负责：
- 根据 `message.id` 维护已测量高度缓存
- 使用估算高度为未测量项提供初值
- 根据当前 `scrollTop + viewportHeight` 计算应该渲染的索引窗口
- 计算前后 spacer 总高度

`MessageList.svelte` 使用 `ResizeObserver` 监听已渲染消息容器高度变化，并回写缓存。这样可以覆盖：
- 流式内容增长
- 推理区展开/折叠
- Paste 内容加载后高度变化
- 版本切换后的高度变化

### 3. 顶部加载与锚点保持

保留现有“滚动接近顶部自动加载更旧消息”的行为，但把 prepend 后的位置恢复改成显式锚点：
- 记录加载前容器 `scrollHeight` 和 `scrollTop`
- 旧消息 prepend 完成并重新计算窗口后，将 `scrollTop` 调整为 `旧 scrollTop + (新 scrollHeight - 旧 scrollHeight)`

这样旧消息插入时用户视觉位置不跳动。

### 4. 底部跟随策略

仅当用户处于“接近底部”状态时，新增消息或流式增长才自动滚到底部；否则保持当前阅读位置。

判断方式：
- `distanceToBottom <= threshold` 视为跟随底部
- 流式消息高度持续增长时，若当前处于 bottom-follow，则持续更新到底部

### 5. 与现有业务保持解耦

- `ChatArea.svelte` 继续只传递消息数组和分页状态
- `+page.svelte` 继续负责加载最新页、加载更旧页、拼接消息和处理流事件
- 新增逻辑主要集中在 `MessageList.svelte` 与新的虚拟化 helper

## Data Flow

1. `+page.svelte` 传入完整 `messages`（当前已加载页面段）
2. `MessageList.svelte` 根据缓存高度和滚动位置计算可见窗口
3. 仅渲染窗口内消息，并为每个消息项注册高度测量
4. 高度变化后更新缓存并重新计算窗口
5. 若处于 bottom-follow，消息增长后自动调整到列表底部
6. 若触发顶部加载，prepend 完成后恢复锚点

## Testing Strategy

由于仓库当前没有成熟的前端组件测试基础设施，本批采取“两层验证”：

1. 为纯 helper 增加 Node 原生测试，覆盖：
   - 窗口区间计算
   - spacer 高度计算
   - prepend 后索引/滚动恢复相关的基础数学逻辑
2. 用 `pnpm run check` 验证 Svelte/TypeScript 集成正确
3. 手动关注以下回归场景：
   - 长对话滚动到顶部自动加载
   - prepend 后视口不跳
   - 流式回复期间底部跟随正常
   - 推理折叠/展开后高度更新正常
   - Paste 展开后布局正常

## Risks and Mitigations

1. **动态高度频繁变化导致抖动**
   - 使用 `ResizeObserver` 增量更新单项高度，不在每次滚动时全量重新测量。
2. **prepend 与窗口化共同作用导致滚动跳动**
   - 保留显式锚点恢复逻辑，并在 DOM 更新后用最新 `scrollHeight` 计算偏移。
3. **流式内容增长时渲染窗口错位**
   - 将流式消息所在项的高度变化纳入同一测量管线，必要时在 bottom-follow 状态下同步滚底。
4. **估算高度不准导致初始 spacer 偏差**
   - 为未测量项使用温和默认值，待首轮渲染后由实测高度纠正。
