# Orion Chat Assistants 对话绑定设计

**日期**: 2026-03-08
**状态**: Approved
**目标**: 让新对话默认不带任何 prompt，并允许用户在聊天顶部通过横向滚动标签栏为“尚未发送首条消息”的对话选择 Assistant；Assistant 的管理统一放到设置页独立的 `Assistants` 标签中。

## 背景

当前项目已经具备 `Assistant` 数据模型、数据库表以及增删改接口：
- `Assistant` 支持名称、`system_prompt`、默认模型、通用参数和扩展参数
- `Conversation` 已经包含 `assistant_id`
- 但现有 UI 中 `Assistant` 仍停留在侧边栏列表，未形成“设置里统一管理 + 新对话顶部选择”的完整使用链路

本次需求确认后的关键约束如下：
- 前后端与 UI 统一使用 `Assistant / Assistants` 命名
- 新建对话默认 `assistantId = null`
- 顶部 Assistant 选择仅允许发生在“新建且尚未发送首条用户消息”的对话中
- 一旦该对话已经发送首条消息，Assistant 不可再修改
- 侧边栏中现有 `AssistantList` 删除
- 设置页新增独立 `Assistants` 选项卡统一管理 Assistant
- Assistant 可以绑定默认模型，也可以留空
- 若 Assistant 绑定了默认模型，选中它时只把该模型作为聊天页默认值；用户仍可手动切换模型
- 用户手动切换模型后，参数编辑区必须自动切换到当前模型所属 provider 的参数集合，不能出现 provider 参数错配

## 方案选择

### 方案 A：复用现有 Assistant 实体，在 UI 中重组入口与使用流程（选定）

**做法**：
- 保持 Rust、SQLite、TypeScript 里的 `Assistant` 结构不变
- 通过设置页新增 `Assistants` 标签完成管理
- 在聊天区顶部新增 Assistant 标签栏完成“新对话选择”
- 新增或扩展 conversation 更新接口，用于在空对话中绑定 `assistantId`

**优点**：
- 复用现有后端数据结构和 CRUD 能力
- 变更聚焦在交互流程，而不是重命名底层模型
- 风险最小，最符合当前需求范围

**缺点**：
- 需要把现有侧边栏 Assistant 入口迁走
- 需要补足会话级 Assistant 选择与参数同步逻辑

### 方案 B：完整重构为新的 Agent 概念

**不选原因**：
- 与“复用现有 Assistant”决策冲突
- 改动面过大，且会引入一套与当前模型重复的实体

### 方案 C：保留侧边栏 AssistantList，同时再增加设置页和顶部选择

**不选原因**：
- 信息架构重复
- 用户难以判断“管理入口”和“使用入口”之间的关系
- 与“统一在设置页管理”冲突

## 信息架构

### 设置页

设置页左侧导航新增 `Assistants`：
- 与 `Model Service`、`General Settings` 等并列
- 点击后右侧内容区域显示 Assistant 管理界面

Assistant 管理界面采用“两栏式”：
- 左侧列表：展示所有 Assistants，支持新建、选择、删除
- 右侧表单：编辑当前 Assistant 的完整配置

表单内容包括：
- 名称
- System Prompt
- 默认模型（可为空）
- 通用参数：`temperature`、`topP`、`maxTokens`
- Provider 专属参数：依据当前绑定模型所属 provider 动态展示

### 聊天页

聊天页主区域顶部新增一个 header 区：
- 横向滚动的 Assistant 标签栏（shadcn 风格）
- 保留模型选择与参数入口在同一区域或相邻区域

Assistant 标签栏内容：
- 一个“无 Assistant”的标签项
- 若干 Assistant 标签项
- 标签栏横向可滚动，适配数量较多的场景

标签栏的状态规则：
- 当对话还没有任何用户消息时：可点击切换
- 当对话已有首条用户消息后：变为只读禁用态

## 数据流设计

### 1. 新建对话

- 用户点击“新对话”时，仍通过现有 `create_conversation` 创建记录
- 新对话初始 `assistant_id = null`
- 聊天页顶部默认选中“无 Assistant”

### 2. 在空对话中选择 Assistant

- 用户点击顶部某个 Assistant 标签
- 前端调用新的 conversation 更新接口，将当前 `conversation.assistantId` 更新为对应 Assistant ID
- 如果该 Assistant 配置了默认模型：
  - 前端把聊天页当前选中模型切换为该默认模型
  - 同时刷新参数面板上下文，使其依据该模型所属 provider 展示参数
- 如果该 Assistant 未配置默认模型：
  - 仅绑定 `assistantId`
  - 当前聊天模型保持不变

### 3. 发送首条消息

- 发送消息时，后端根据当前 `conversation.assistant_id` 读取 Assistant
- 若 Assistant 的 `system_prompt` 非空，则把它作为 system message 注入对话上下文
- 实际使用的模型以聊天页当前选中的模型为准
- 实际使用的参数也以当前聊天页参数面板状态为准

### 4. 用户手动切换模型

- 如果用户在已选定 Assistant 的空对话中手动切换模型：
  - 允许切换
  - 聊天参数面板改为读取新模型所属 provider 的参数模板
  - 不显示旧 provider 的专属参数
- 发送时只提交当前模型对应 provider 的参数

### 5. 对话冻结条件

“是否还能改 Assistant”的判断依据不是对话创建时间，而是：
- 当前对话中是否已经存在首条 `role = user` 且未删除的消息

只要已经发送过用户消息：
- 顶部 Assistant 标签栏禁用
- 不允许再次修改 `assistantId`

## 参数模型设计

### Assistant 自身配置

Assistant 配置分为两层：
- **通用参数**：`temperature`、`topP`、`maxTokens`
- **Provider 专属参数**：存放在 `extraParams`

Provider 专属参数遵循“当前绑定模型决定可编辑项”的规则：
- 绑定 `Anthropic` 模型：展示 `Anthropic` 参数
- 绑定 `Gemini` 模型：展示 `Gemini` 参数
- 绑定 `OpenAI Compatible` 模型：展示 OpenAI Compatible 参数
- 绑定 `Ollama` 模型：展示 Ollama 参数
- 模型为空：不展示 provider 专属参数，只允许编辑通用参数

### 聊天发送参数优先级

发送消息时建议采用如下优先级：
1. 当前聊天页用户已显式选择/调整的模型与参数
2. 若当前聊天页尚未覆盖，而 Assistant 提供默认模型，则使用 Assistant 默认模型作为初始值
3. 若 Assistant 未提供默认模型，则沿用当前对话已有模型选择

这样可以满足：
- Assistant 作为“默认模板”而不是强制锁定
- 用户切换模型后，provider 参数永远与当前模型一致

## 后端改动设计

### Conversation 命令

新增一个会话级接口，职责单一：
- 更新某个 conversation 的 `assistant_id`
- 同时执行合法性校验：
  - conversation 存在
  - assistant 存在或允许为 `null`
  - 若该对话已存在用户消息，则拒绝更新并返回错误

建议命名：
- `update_conversation_assistant`

### Chat 命令

`send_message` / `resend_message` / 相关流式逻辑需要补充：
- 根据 `conversation.assistant_id` 读取 Assistant
- 如 `system_prompt` 存在，则在请求消息数组前部加入 system message

这里需要保证：
- 只把 Assistant prompt 作为系统上下文注入，不污染用户可见消息列表
- 不改变现有消息存储结构

## 前端改动设计

### 设置页导航

在 `src/lib/components/settings/ProviderSettings.svelte`：
- 为左侧导航增加 `assistants` 这一稳定 nav id
- 文案走现有 i18n 体系
- 右侧渲染新增的 Assistant 管理面板

### Assistant 管理面板

建议拆分出专门组件，例如：
- `src/lib/components/settings/AssistantSettings.svelte`

职责：
- 加载与展示所有 Assistants
- 新建 / 删除 / 选择当前 Assistant
- 编辑名称、prompt、默认模型、通用参数、provider 专属参数
- 保存回 `updateAssistant`

### 聊天顶部 Assistant 标签栏

建议新增组件，例如：
- `src/lib/components/chat/AssistantTabs.svelte`

职责：
- 展示“无 Assistant”与所有 Assistants
- 支持横向滚动
- 根据当前 conversation 状态决定是否可切换
- 发出 `onSelect(assistantId | null)` 事件

### 聊天页状态协调

在 `src/routes/+page.svelte` 中补充：
- 当前会话详情中的 `assistantId` 管理
- 加载 Assistants 列表
- 判断当前会话是否已发送用户消息
- 在空对话里切换 Assistant 后更新会话
- 如 Assistant 带默认模型，同步更新 `currentModelId`

必要时还需要把 `ChatArea` 扩展为接收：
- 当前 conversation 对应的 `assistantId`
- Assistants 列表
- 是否允许切换 Assistant

## 错误处理

### 前端

- 如果更新 `assistantId` 失败：
  - 回滚 UI 的选中态
  - 显示错误提示或至少输出明确日志
- 如果对话已被冻结（已有用户消息）但前端仍尝试更新：
  - 前端按钮禁用
  - 同时以后端错误作为兜底

### 后端

- conversation 不存在：返回 `NotFound`
- assistant 不存在：返回 `NotFound`
- 对话已有用户消息：返回明确业务错误
- Assistant prompt 为空：允许发送，不注入 system message

## 测试策略

### Rust / 数据层

优先补充数据库与命令层验证：
- `update_conversation_assistant` 能更新空对话的 `assistant_id`
- 有用户消息的对话不能更新 `assistant_id`
- 删除或不存在的 Assistant 不能被绑定

### 前端 / 静态检查

项目当前以 `npm run check` 为主，因此本次至少验证：
- 新增设置导航与组件类型正确
- 聊天顶部 Assistant 标签栏无类型错误
- 所有新增 props / 事件 / i18n 文案接线正确
- 无新增 a11y warning

### 交互回归

手动回归重点：
- 新建对话默认无 Assistant
- 空对话可切换 Assistant
- 首条消息后 Assistant 变只读
- 选中带默认模型的 Assistant 后，模型自动变为默认值
- 用户再手动切换模型后，参数面板切换到对应 provider
- 设置页编辑不同 provider 模型时，不会显示错配参数

## 影响范围

预计涉及的主要文件：
- `src/routes/+page.svelte`
- `src/lib/components/chat/ChatArea.svelte`
- `src/lib/components/chat/ModelSelector.svelte`
- `src/lib/components/chat/ModelParamsPopover.svelte`
- `src/lib/components/sidebar/AppSidebar.svelte`
- `src/lib/components/sidebar/AssistantList.svelte`（删除引用或移除）
- `src/lib/components/settings/ProviderSettings.svelte`
- `src/lib/utils/invoke.ts`
- `src/lib/types/index.ts`
- `src-tauri/src/commands/conversation.rs`
- `src-tauri/src/commands/chat.rs`
- `src-tauri/src/db/conversations.rs`

## 非目标

本次不做：
- 中途修改已有消息对话的 Assistant
- 把 Assistant prompt 落成用户可见消息
- 新增独立的 Agent 概念
- 重构整个模型参数存储系统
- 为前端引入新的测试框架
