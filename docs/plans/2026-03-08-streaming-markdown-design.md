# Orion Chat 流式阶段纯文本预览设计

**日期**: 2026-03-08
**状态**: Approved
**目标**: 在 Assistant 消息流式输出期间跳过 Markdown 解析与代码高亮，只显示保留换行的纯文本预览；消息完成后再一次性渲染最终 Markdown。

## 背景

在当前实现中，`MessageBubble.svelte` 会对 Assistant 消息内容直接执行 `renderMarkdown(message.content)`，对 reasoning 也执行 `renderMarkdown(message.reasoning)`：
- 流式响应期间，每收到一个 chunk，`messages[idx].content` 都会被拼接更新
- Svelte 随即重新计算派生值，触发 `marked.parse(...)`
- 如果内容中包含代码块，还会进一步触发 `highlight.js`

这意味着一条长回复在 streaming 阶段会重复进行大量 Markdown 解析和代码高亮，造成：
- CPU 占用抖动明显
- 前端内存分配更频繁
- 代码类回复的流式滚动更卡顿

在 `docs/optimization.md` 的五项优化中，这一项适合作为“消息分页”之后的下一步：
- 影响面集中在单个组件
- 不需要改动后端协议
- 收益直接体现在最频繁的渲染热点上

本轮需求边界确认如下：
- 仅在 **Assistant 消息处于 `streaming` 状态** 时禁用 Markdown 解析
- 流式期间显示为**纯文本预览**，保留换行
- 消息进入 `done` 或 `error` 状态后，恢复现有 Markdown 渲染
- reasoning 在 streaming 阶段也采用同样策略
- 本轮不做节流解析或增量解析

## 方案对比

### 方案 A：流式阶段完全跳过 Markdown 解析（选定）

**做法**：
- 对 `message.status === 'streaming'` 的 Assistant 消息，不调用 `renderMarkdown`
- 改为直接显示原始文本，并使用 `white-space: pre-wrap` 保留换行
- 流结束后，沿用现有 `renderMarkdown(...)` 路径输出最终 HTML

**优点**：
- 改动最小，逻辑清晰
- 性能收益稳定，不依赖节流参数
- 不需要修改后端和消息数据结构

**缺点**：
- 流式期间没有富文本样式
- 代码块、列表、标题、链接等要等到完成后才显示最终效果

### 方案 B：流式阶段节流 Markdown 解析

**不选原因**：
- 仍然会重复做 Markdown 解析，只是频率下降
- 需要设计节流窗口、收尾刷新时机，复杂度明显上升

### 方案 C：增量 Markdown 渲染

**不选原因**：
- 对未闭合代码块、列表、引用块等语法不稳
- 实现复杂且维护成本高，不适合作为当前这轮优化的第二步

## 组件设计

### MessageBubble 行为分层

在 `src/lib/components/chat/MessageBubble.svelte` 中，Assistant 消息的渲染分为两种模式：

#### 1. Streaming 预览模式

条件：
- `message.role === 'assistant'`
- `message.status === 'streaming'`

行为：
- `content` 直接作为纯文本展示
- `reasoning` 直接作为纯文本展示
- 使用纯文本容器样式保留换行与连续空格体验
- 不调用 `renderMarkdown`
- 不执行 `highlight.js`

#### 2. Final 渲染模式

条件：
- `message.role === 'assistant'`
- `message.status !== 'streaming'`

行为：
- 继续沿用现有 `renderMarkdown(message.content)`
- reasoning 继续沿用 `renderMarkdown(message.reasoning)`
- 保留当前代码高亮、列表、链接等最终样式

## 数据流设计

### 当前流式更新路径

当前页面层在收到流事件后会：
- 找到 streaming message
- 将 `delta` 追加到 `message.content`
- 将 reasoning chunk 追加到 `message.reasoning`
- 最终在 `finished` 事件时把状态改为 `done`

本轮不改变这条数据流，只改变视图层如何解释 `streaming` 状态：
- streaming：纯文本预览
- done/error：Markdown 渲染

因此这轮优化不会影响：
- Assistant prompt 注入
- 自动压缩
- 消息版本生成与切换
- 消息复制、编辑、删除等现有命令

## 样式设计

### 纯文本预览样式

为 streaming 阶段新增轻量样式类，例如：
- `.message-plain`
- `.reasoning-plain`

关键点：
- `white-space: pre-wrap`，保留换行
- `word-break: break-word` 或等效规则，避免超长内容撑破布局
- 字体、字号、颜色尽量与最终消息风格接近，避免完成时视觉跳变过大

## 兼容性考虑

### 1. 用户消息

用户消息本身没有走 Assistant Markdown 渲染路径，因此保持现状即可。

### 2. 错误消息

如果 Assistant 消息最终状态为 `error`：
- 维持当前最终显示路径即可
- 因为错误消息通常是短文本，即便走 Markdown 也无明显性能问题

### 3. reasoning 展开/收起

reasoning 在 streaming 阶段改成纯文本后：
- 展开逻辑保持不变
- 仅内容容器从 HTML 改为纯文本
- 结束后再恢复为富文本块

### 4. 复制与编辑

复制与编辑基于原始 `message.content` 和 `message.reasoning` 数据，不依赖 Markdown HTML，因此不受影响。

## 风险与缓解

### 风险 1：流式期间视觉较“朴素”

**缓解**：
- 保持字体、间距和配色与最终气质接近
- 仅牺牲富文本，不牺牲可读性

### 风险 2：完成瞬间从纯文本切换到富文本，用户感知到轻微重排

**缓解**：
- 这是有意接受的折中
- 比起每个 chunk 都重排，结束时只重排一次整体成本更低

### 风险 3：某些逻辑仍在 streaming 时意外调用 Markdown

**缓解**：
- 把 `isStreamingAssistant` 判断集中在 `MessageBubble.svelte`
- 只保留一个渲染分支入口，避免局部遗漏

## 验证策略

### 前端

- `pnpm run check`
- 手动验证一条长代码回复：
  - streaming 阶段显示纯文本
  - 结束后自动切换为 Markdown + 代码高亮
- 手动验证 reasoning：
  - streaming 阶段展开后是纯文本
  - 结束后恢复富文本显示

### 性能观察

重点对比优化前后：
- 流式代码回复时的界面顺滑度
- 长回复时 CPU 抖动
- 多会话切换后 WebView 内存增长速度

## 后续建议

完成本轮后，建议继续按这个顺序推进：
1. 删掉大多数操作后的全量 reload，尽量改成局部更新或分页刷新
2. 再评估是否还需要虚拟列表
3. 最后处理超长 paste 内容外置存储
