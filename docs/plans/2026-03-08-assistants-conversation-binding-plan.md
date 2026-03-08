# Orion Chat Assistants 对话绑定 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 Orion Chat 增加“新对话顶部选择 Assistant”的流程，并在设置页新增 `Assistants` 管理入口；新对话默认不绑定 Assistant，首条用户消息发出后冻结 Assistant 选择。

**Architecture:** 复用现有 `Assistant` 实体与 `conversation.assistant_id` 字段，新增会话级 Assistant 更新命令，并在聊天发送时根据 conversation 绑定的 Assistant 注入 system prompt。前端在设置页新增 `Assistants` 标签与管理面板，在聊天区顶部新增 shadcn 风格横向滚动标签栏，并把模型/参数联动约束到“当前实际选中模型”的 provider 上。

**Tech Stack:** Rust、Tauri v2 commands、SQLite、Svelte 5 runes、TypeScript、shadcn-svelte 风格组件

---

### Task 1: 补齐会话级 Assistant 绑定能力

**Files:**
- Modify: `src-tauri/src/db/conversations.rs`
- Modify: `src-tauri/src/commands/conversation.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/utils/invoke.ts`

**Step 1: 为会话表增加更新 assistant_id 的数据库方法**

在 `src-tauri/src/db/conversations.rs` 新增更新函数，接受 `conversation id` 与 `assistant_id | null`，只负责持久化字段与更新时间。

**Step 2: 在命令层新增 `update_conversation_assistant`**

在 `src-tauri/src/commands/conversation.rs` 中新增 tauri command，完成以下校验：
- conversation 存在
- assistant 为空时允许解绑
- assistant 非空时确认 assistant 存在
- 该 conversation 还没有任何未删除的用户消息，否则返回业务错误

**Step 3: 注册命令并暴露前端调用**

- 在 `src-tauri/src/lib.rs` 注册 `update_conversation_assistant`
- 在 `src/lib/utils/invoke.ts` 增加对应 `api.updateConversationAssistant(id, assistantId)` 方法

**Step 4: 先跑类型/编译边界检查**

Run: `cargo test conversations --manifest-path src-tauri/Cargo.toml`
Expected: 相关测试通过；若过滤过严至少能编译命令与数据库模块。

### Task 2: 让聊天发送链路实际使用 conversation 绑定的 Assistant prompt

**Files:**
- Modify: `src-tauri/src/commands/chat.rs`
- Modify: `src-tauri/src/models/message.rs`（仅当需要调整 system role 注入逻辑时）

**Step 1: 在发送前读取 conversation.assistant_id**

在 `src-tauri/src/commands/chat.rs` 的发送/重发链路中，读取当前 conversation，并在 `assistant_id` 非空时加载对应 Assistant。

**Step 2: 把 system prompt 注入请求上下文**

在构建 `ChatRequest.messages` 前，将 Assistant 的 `system_prompt` 作为 system message 插入到发送给 provider 的消息数组头部；不要把它写入用户可见消息表。

**Step 3: 确保空 prompt 与无 Assistant 正常回退**

如果 `assistant_id` 为空或 `system_prompt` 为空，则行为与当前实现一致。

**Step 4: 运行后端测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: Rust 单元测试通过，无新增编译错误。

### Task 3: 为 Assistant 编辑补齐 provider 感知的参数表单

**Files:**
- Create: `src/lib/components/settings/AssistantSettings.svelte`
- Modify: `src/lib/components/settings/ProviderSettings.svelte`
- Modify: `src/lib/types/index.ts`
- Modify: `src/lib/utils/invoke.ts`（若需要更强类型辅助）

**Step 1: 新建 Assistant 管理组件骨架**

在 `src/lib/components/settings/AssistantSettings.svelte` 中实现：
- Assistant 列表加载
- 新建 / 删除 / 选择
- 基础表单状态管理

**Step 2: 接入名称、prompt、默认模型与通用参数编辑**

让表单支持编辑：
- `name`
- `systemPrompt`
- `modelId`
- `temperature`
- `topP`
- `maxTokens`

**Step 3: 按绑定模型 provider 动态渲染专属参数**

复用 `ModelParamsPopover.svelte` 的 provider 分支规则，确保：
- `anthropic` 只显示 anthropic 参数
- `gemini` 只显示 gemini 参数
- `openaiCompat` 只显示 openai compatible 参数
- `ollama` 只显示 ollama 参数
- `modelId` 为空时不显示 provider 专属参数区

**Step 4: 接入保存与删除**

把表单保存映射到 `api.updateAssistant`，删除映射到 `api.deleteAssistant`，并处理空状态与默认选中逻辑。

**Step 5: 把设置页左侧导航新增 `Assistants` 标签**

在 `src/lib/components/settings/ProviderSettings.svelte` 中：
- 为 `NavItemId` 加入 `assistants`
- 添加中英文文案
- 右侧内容区在 `activeNav === 'assistants'` 时渲染 `AssistantSettings`

**Step 6: 运行前端静态检查**

Run: `npm run check`
Expected: `svelte-check` 成功，无新增类型或 a11y 警告。

### Task 4: 在聊天顶部增加 Assistant 横向标签栏并移除侧边栏入口

**Files:**
- Create: `src/lib/components/chat/AssistantTabs.svelte`
- Modify: `src/lib/components/chat/ChatArea.svelte`
- Modify: `src/routes/+page.svelte`
- Modify: `src/lib/components/sidebar/AppSidebar.svelte`
- Modify: `src/lib/components/sidebar/AssistantList.svelte` 或删除其引用
- Modify: `src/lib/stores/i18n.svelte.ts`

**Step 1: 新增 Assistant 标签栏组件**

在 `src/lib/components/chat/AssistantTabs.svelte` 中实现：
- “None / 无”标签
- 所有 Assistants 标签
- 横向滚动容器
- 选中态 / 禁用态 / hover 态
- 对外抛出选择事件

**Step 2: 把标签栏接入 ChatArea 顶部**

在 `src/lib/components/chat/ChatArea.svelte` 增加 header 区并接收：
- `assistants`
- `selectedAssistantId`
- `assistantSelectionLocked`
- `onAssistantSelect`

**Step 3: 在页面层协调会话与 Assistant 状态**

在 `src/routes/+page.svelte`：
- 加载 Assistants 列表
- 根据当前 conversation 计算 `selectedAssistantId`
- 根据消息列表判断“是否已发送首条用户消息”
- 在允许状态下调用 `api.updateConversationAssistant`
- 若选中的 Assistant 带 `modelId`，同步更新 `currentModelId`

**Step 4: 删除侧边栏中的 Assistant 管理入口**

从 `src/lib/components/sidebar/AppSidebar.svelte` 及相关布局中移除 `AssistantList` 入口，避免出现双入口。

**Step 5: 补齐顶部标签栏文案国际化**

在 `src/lib/stores/i18n.svelte.ts` 中增加：
- `assistants`
- `noAssistant`
- `assistantLocked` 等必要文案

**Step 6: 运行静态检查**

Run: `npm run check`
Expected: `svelte-check found 0 errors and 0 warnings`。

### Task 5: 处理模型与参数联动，避免 provider 参数错配

**Files:**
- Modify: `src/routes/+page.svelte`
- Modify: `src/lib/components/chat/ModelSelector.svelte`
- Modify: `src/lib/components/chat/ModelParamsPopover.svelte`
- Modify: `src/lib/components/settings/AssistantSettings.svelte`
- Modify: `src/lib/stores/modelParams.ts`（仅当需要抽取共用 provider 默认值逻辑时）

**Step 1: 明确“Assistant 只提供默认模型”规则**

在聊天页状态逻辑中实现：
- 选择 Assistant 时，如果 Assistant 绑定 `modelId`，把它写入当前 `currentModelId`
- 之后用户仍可自由切换模型

**Step 2: 让聊天参数区始终以当前 `currentModelId` 为准**

确保 `ModelParamsPopover.svelte` 的 provider 参数读取只跟随当前模型所属 provider，而不是跟随 Assistant 自身历史配置。

**Step 3: 让设置页 Assistant 表单也遵循相同 provider 规则**

在 `AssistantSettings.svelte` 中，当 `modelId` 变化时：
- 保留通用参数
- 重置/切换 provider 专属参数编辑区
- 不展示其他 provider 的参数项

**Step 4: 检查手动换模场景**

人工验证：
- Assistant 默认模型为 Claude 时，进入对话自动带 Claude
- 用户改成 Gemini 后，参数区只显示 Gemini 参数
- 再切回 OpenAI Compatible 时，参数区只显示 OpenAI Compatible 参数

**Step 5: 跑最终前端检查**

Run: `npm run check`
Expected: `svelte-check` 通过，无新增警告。

### Task 6: 为冻结规则与会话绑定增加回归验证

**Files:**
- Modify: `src-tauri/src/db/conversations.rs`
- Modify: `src-tauri/src/commands/conversation.rs`
- Modify: `docs/plans/2026-03-08-assistants-conversation-binding-design.md`（如需补充实现偏差说明）

**Step 1: 为数据库/命令补最小测试**

至少覆盖：
- 空对话可以更新 assistant
- 有用户消息的对话不能更新 assistant
- 不存在的 assistant 不能绑定

**Step 2: 运行后端测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 测试通过。

**Step 3: 运行前端检查**

Run: `npm run check`
Expected: 0 errors / 0 warnings。

**Step 4: 手动回归清单**

逐项验证：
- 新对话默认无 Assistant
- 顶部标签栏可横向滚动
- 选择 Assistant 后可自动带入默认模型
- 首条消息后标签栏禁用
- 设置页 `Assistants` 可新建、编辑、删除
- provider 参数不会错配
