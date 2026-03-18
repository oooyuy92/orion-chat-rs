# Orion Chat — yoagent 执行 Agent 集成设计

**日期**: 2026-03-17
**状态**: 已审批
**作者**: Claude Opus 4.6 + 用户协作

---

## 概述

将 yoagent（Rust AI Agent 框架 v0.7.0）以深度融合方式集成进 Orion Chat，为用户提供默认开启的 Agent 模式，支持工具调用、Skills 加载和 MCP 服务器连接。

---

## 目标

- 默认开启 Agent 模式，用户可在同一会话内随时切换为普通聊天模式
- 提供 bash / read_file / write_file / edit_file / list_files / search 六种内置工具
- 兼容标准 AgentSkills 格式的 `.md` Skills 文件
- 工具授权等级由用户在设置中自定义
- 复用现有 messages 表存储工具调用历史
- 共享 Orion Provider 配置（API Key / model），Agent 使用 yoagent 的 Provider 层执行 LLM 请求
- 支持 MCP 服务器连接（stdio + HTTP 传输）

---

## 架构：方案 A 深度融合

yoagent 源码作为 Cargo workspace member 嵌入 `src-tauri/crates/yoagent/`，直接在项目中修改维护，不跟进上游更新。

```
Orion Chat Tauri Process
├── 普通聊天模式        现有 Provider 层 + stream_chat()
│
├── Agent 模式          yoagent (workspace member)
│   ├── Agent Loop      prompt → LLM stream → tool call → loop
│   ├── 内置工具        bash, read_file, write_file, edit_file, list_files, search
│   ├── Skills 加载     AgentSkills 格式 .md 文件
│   ├── MCP Client      stdio + HTTP，连接外部 MCP 服务器
│   └── Sub-agent       子任务委派（yoagent 内置）
│
├── 共享层
│   ├── Provider 配置   从 Orion DB 读取 API Key / base_url / model，注入 yoagent
│   ├── 消息存储        messages 表扩展，存储工具调用消息
│   └── Cancel Token    per-conversation，转发给 yoagent CancellationToken
│
└── 前端 (Svelte)
    ├── Agent 开关      model-row 右侧，hover 展开的 bot 图标按钮
    ├── 工具执行展示    步骤时间线（border-left 样式）
    ├── 授权弹窗        ask 等级工具弹出确认
    └── Agent 设置      工具授权 / Skills 目录 / MCP 服务器
```

---

## Section 0：yoagent 公共 API 契约

`agent/` 模块依赖以下 yoagent 核心类型（已在 yoagent v0.7.0 确认存在）：

```rust
// yoagent 暴露的核心接口
use yoagent::{
    agent_loop,           // 核心函数：运行 agent loop
    AgentLoopConfig,      // 配置：system_prompt, tools, provider_config, cancel_token, on_event
    AgentEvent,           // 事件枚举：TextDelta, ToolCallStart, ToolCallUpdate, ToolCallComplete
    ProviderConfig,       // provider 类型 + api_key + base_url + model
    AgentTool,            // trait：工具实现接口
    InputFilter,          // trait：工具执行前拦截（用于实现权限检查）
    Message,              // 消息类型（含 ToolCall / ToolResult 变体）
};

// agent_loop 签名（简化）
pub async fn agent_loop(
    config: AgentLoopConfig,
    on_event: impl Fn(AgentEvent) + Send + 'static,
) -> Result<Vec<Message>, AgentError>

// AgentLoopConfig 关键字段
pub struct AgentLoopConfig {
    pub system_prompt: String,           // Agent 系统提示词（含 Skills 内容注入）
    pub messages: Vec<Message>,          // 历史消息（含工具调用历史）
    pub tools: Vec<Box<dyn AgentTool>>,
    pub provider: ProviderConfig,
    pub cancel_token: CancellationToken,
    pub input_filter: Option<Box<dyn InputFilter>>,
}
```

**关键设计**：yoagent 的 `agent_loop` 负责在每轮工具调用后自动重组消息历史（含 tool_call / tool_result）并重新发起 LLM 请求。Orion 的 `storage.rs` 负责将每轮新增消息持久化到 SQLite；`config.rs` 在每次调用前从 DB 加载完整历史作为 `messages` 参数传入。

---

## Section 1：后端模块结构

```
src-tauri/
├── Cargo.toml                  添加 crates/yoagent 为 workspace member
├── crates/
│   └── yoagent/                yoagent 源码（直接嵌入，后续可自由修改）
└── src/
    ├── agent/
    │   ├── mod.rs              Agent 模块入口，注册 Tauri commands
    │   ├── commands.rs         Tauri commands（见下方）
    │   ├── events.rs           AgentEvent → ChatEvent 映射逻辑
    │   ├── permissions.rs      工具授权 InputFilter 实现
    │   ├── config.rs           从 Orion DB 读取配置，构建 yoagent ProviderConfig
    │   └── storage.rs          工具调用消息写入 messages 表
    └── chat/                   现有聊天模块（保持不变）
```

### Tauri Commands

```rust
// 启动 Agent 会话（当 agent_mode=true 时调用，替代普通 chat 命令）
// channel: Tauri Channel<ChatEvent>，与现有 send_message 一致
agent_chat(
    conversation_id: String,
    message: String,
    model_id: String,
    channel: Channel<ChatEvent>,
) -> Result<Message>  // 返回最终持久化的 assistant 消息

// 停止 Agent（取消 CancellationToken，若有 pending ToolAuthRequest 则自动拒绝）
agent_stop(conversation_id: String) -> Result<()>

// 用户响应授权弹窗（非阻塞，通过 AppState 中的 oneshot channel 传递结果）
agent_authorize_tool(
    tool_call_id: String,
    action: AuthAction,  // Allow | AllowSession | Deny
) -> Result<()>

// 读取/写入工具授权配置
get_tool_permissions() -> Result<ToolPermissions>
set_tool_permissions(perms: ToolPermissions) -> Result<()>

// MCP 服务器管理
add_mcp_server(config: McpServerConfig) -> Result<()>
remove_mcp_server(name: String) -> Result<()>
list_mcp_servers() -> Result<Vec<McpServerStatus>>
```

### ToolAuthRequest 等待机制

`permissions.rs` 的 `InputFilter` 实现使用 `tokio::sync::oneshot` 通道挂起 agent loop：

```rust
// AppState 新增字段
pub struct AppState {
    // ... 现有字段 ...
    pub pending_auth: Mutex<HashMap<String, oneshot::Sender<AuthAction>>>,
    pub session_tool_overrides: Mutex<HashMap<(String, String), PermissionLevel>>,
    //                                       ^conv_id  ^tool_name
}

// InputFilter 实现（简化）
impl InputFilter for PermissionFilter {
    async fn before_tool(&self, tool_name: &str, args: &str, tool_call_id: &str) -> FilterResult {
        // 1. 先检查 session-level 覆盖（AllowSession 已记录的工具）
        if let Some(level) = self.state.session_tool_overrides.lock()
            .get(&(self.conv_id.clone(), tool_name.to_string())) {
            return match level {
                PermissionLevel::Auto => FilterResult::Allow,
                PermissionLevel::Deny => FilterResult::Deny("工具已被禁用".into()),
                _ => unreachable!(),
            };
        }
        // 2. 回退到全局设置
        match self.get_global_permission(tool_name) {
            auto   => FilterResult::Allow,
            deny   => FilterResult::Deny("工具已被禁用".into()),
            ask    => {
                let (tx, rx) = oneshot::channel();
                self.state.pending_auth.lock().insert(tool_call_id.to_string(), tx);
                self.channel.send(ChatEvent::ToolAuthRequest { tool_call_id, tool_name, args });
                // 等待用户响应或 cancel_token / 超时触发
                tokio::select! {
                    Ok(action) = rx => {
                        if action == AuthAction::AllowSession {
                            // 写入 session 覆盖，下次同名工具自动放行
                            self.state.session_tool_overrides.lock()
                                .insert((self.conv_id.clone(), tool_name.to_string()), PermissionLevel::Auto);
                        }
                        match action {
                            Allow | AllowSession => FilterResult::Allow,
                            Deny => FilterResult::Deny("用户拒绝".into()),
                        }
                    },
                    _ = cancel_token.cancelled() => FilterResult::Deny("已取消".into()),
                    _ = tokio::time::sleep(Duration::from_secs(300)) => FilterResult::Deny("超时".into()),
                }
            }
        }
    }
}
```

超时时间 300 秒（5 分钟），超时自动拒绝。`agent_stop()` 触发 cancel_token 会自动退出等待。

---

## Section 2：消息存储与数据库迁移

### messages 表扩展

```sql
ALTER TABLE messages ADD COLUMN message_type TEXT NOT NULL DEFAULT 'text';
-- 取值: 'text' | 'tool_call' | 'tool_result'

ALTER TABLE messages ADD COLUMN tool_call_id TEXT;
-- tool_call 与 tool_result 通过此字段关联

ALTER TABLE messages ADD COLUMN tool_name TEXT;
-- 工具名称，如 'bash', 'read_file'

ALTER TABLE messages ADD COLUMN tool_input TEXT;
-- 工具调用参数，JSON 字符串

ALTER TABLE messages ADD COLUMN tool_error INTEGER NOT NULL DEFAULT 0;
-- tool_result 是否为错误结果（0=success, 1=error）
```

向后兼容：现有消息 `message_type` 默认为 `'text'`，无需迁移数据。

### conversations 表扩展（agent_mode 持久化）

```sql
ALTER TABLE conversations ADD COLUMN agent_mode INTEGER NOT NULL DEFAULT 1;
-- 1 = Agent 模式（默认），0 = 普通聊天模式
-- 前端切换时通过 Tauri command 同步写入
```

### agent_settings 表（新建，用于工具权限和 MCP 配置）

```sql
CREATE TABLE IF NOT EXISTS agent_settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL  -- JSON
);
-- 初始记录由 Phase 0 迁移脚本插入默认值
-- key: "tool_permissions" | "mcp_servers" | "skills_dir" | "working_dir"
```

### FTS 索引处理

现有 `messages_fts` FTS5 索引通过触发器在 `messages` 表 `INSERT` 时更新。工具调用消息（`message_type != 'text'`）应排除在 FTS 索引外。需修改迁移脚本中的 FTS 触发器：

```sql
-- 修改现有 after_insert 触发器，增加 message_type 过滤
CREATE TRIGGER messages_ai AFTER INSERT ON messages
  WHEN NEW.message_type = 'text'
BEGIN
  INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;
```

### 工具调用消息写入时机

- `ToolCallStart` 事件触发时：立即在 `messages` 表插入 `message_type='tool_call'` 行，生成 `message_id`，通过 `ChatEvent::ToolCallStart { message_id }` 通知前端（前端据此渲染占位时间线项）
- `ToolCallUpdate` 事件触发时：**仅转发给前端，不写 DB**（用于 bash 等流式输出的实时展示）
- `ToolCallEnd` 事件触发时：更新对应 `tool_call` 行的完整输出，同时插入 `message_type='tool_result'` 行（DB 写入只发生在此）

### Rust 模型扩展

```rust
pub enum MessageType { Text, ToolCall, ToolResult }

pub struct Message {
    // ... 现有字段不变 ...
    pub message_type: MessageType,
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_input: Option<String>,
    pub tool_error: bool,
}
```

### ChatEvent 扩展

```rust
pub enum ChatEvent {
    // ... 现有事件不变 ...
    ToolCallStart  { message_id: String, tool_call_id: String, tool_name: String, args: String },
    ToolCallUpdate { message_id: String, tool_call_id: String, partial_result: String },
    ToolCallEnd    { message_id: String, tool_call_id: String, result: String, is_error: bool },
    ToolAuthRequest { tool_call_id: String, tool_name: String, args: String },
}
```

---

## Section 3：事件流

```
前端调用 agent_chat()
  │
  └─ yoagent agent_loop 启动
      ├─ LLM 流式响应 ──────────► emit ToolCallStart / Delta
      ├─ 工具调用
      │   ├─ permissions.rs 检查
      │   │   ├─ auto  → 直接执行
      │   │   ├─ ask   → emit ToolAuthRequest → 等待 agent_authorize_tool()
      │   │   └─ deny  → 返回拒绝消息给 LLM，继续循环
      │   ├─ 执行中 ───────────► emit ToolCallUpdate（bash 输出流式）
      │   └─ 完成 ─────────────► emit ToolCallEnd，写入 messages 表
      ├─ LLM 继续推理 ──────────► emit Delta
      └─ 无 tool_call → 结束 ──► emit Done
```

Cancel：`agent_stop()` 触发 CancellationToken，yoagent 内部传播取消信号。

---

## Section 4：工具授权机制

### 授权等级

| 等级 | 行为 |
|------|------|
| `auto` | 自动执行，不弹窗 |
| `ask` | 每次执行前弹出确认弹窗 |
| `deny` | 禁止执行，Agent 收到工具被禁用的提示 |

### 默认配置

| 工具 | 默认等级 |
|------|---------|
| read_file | auto |
| list_files | auto |
| search | auto |
| edit_file | ask |
| write_file | ask |
| bash | ask |
| MCP 工具 | ask |

### 授权弹窗

- 显示：工具名称 + 参数摘要
- 三个操作：**允许** / **允许并记住（本次会话）** / **拒绝**
- "允许并记住"：会话级缓存，不修改全局设置

### 存储格式

```json
// agent_settings 表，key: "tool_permissions"
{
  "tool_permissions": {
    "read_file": "auto",
    "list_files": "auto",
    "search": "auto",
    "edit_file": "ask",
    "write_file": "ask",
    "bash": "ask"
  }
}
```

---

## Section 5：前端 UI

### Agent 切换按钮

位置：`InputArea.svelte` 的 `model-row`（现有 ModelSelector / ModelParamsPopover / ComboSelector 所在行），右对齐。

交互：
- **折叠态**：仅显示 Lucide `bot` 线条图标，紧凑
- **Hover 展开**：CSS `max-width` 过渡动画，展开显示 `Agent ON/OFF` 文字和徽标
- **ON 状态**：使用 `var(--primary)` 填充（深色），白色文字
- **OFF 状态**：使用 `var(--muted)` 灰色，静默样式
- 点击切换 agent_mode 状态（同一会话内可随时切换）

### 工具执行展示（步骤时间线）

工具调用过程以 `border-left` 时间线形式展示，独立于助手文本消息：

```
│ Agent 执行中
│ ✓ read_file   src/main.rs        1.2s
│ ⟳ edit_file   src/main.rs:10     ...
```

- `✓` 绿色：完成；`⟳` 黄色：进行中；`✗` 红色：错误
- 时间线项目可点击展开查看完整工具输出
- bash 工具流式输出（实时更新）

### 授权弹窗

使用现有 shadcn-svelte Dialog 组件，展示工具名称 + 参数，三个操作按钮。

---

## Section 6：Agent 设置页面

在现有设置页新增 **Agent** 分区，包含：

### 6.1 工具授权
每个工具一行，下拉选择 `自动执行` / `需要确认` / `禁用`，MCP 工具动态追加。

### 6.2 Skills 配置
- Skills 目录路径（默认 `~/.orion/skills/`）
- 扫描并列出已加载的 `.md` Skills 文件
- 每个 Skill 可独立启用/禁用

### 6.3 MCP 服务器管理
- 添加 / 编辑 / 删除 MCP 服务器
- 字段：名称、传输方式（stdio / HTTP）、命令或 URL
- 连接状态指示（已连接 / 未连接）
- MCP 工具自动出现在工具授权列表

### 6.4 工作目录
- Agent 文件操作的根路径，可在设置中修改
- **默认值**：用户 home 目录（`dirs::home_dir()`），存储于 `agent_settings` 表 key `"working_dir"`

---

## 实现分阶段建议

### Phase 0 — 后端核心集成（可独立验收）
- yoagent 嵌入为 workspace member
- `agent/` 模块骨架：commands、events、config、storage
- messages 表 schema 迁移
- `agent_chat` + `agent_stop` Tauri commands
- ChatEvent 扩展
- 基本工具（read_file / list_files / search 自动；bash / edit_file / write_file ask 默认）

### Phase 1 — 前端基础 UI（可独立验收）
- Agent 切换按钮（InputArea model-row 右侧，hover 展开）
- 工具执行时间线组件
- 授权弹窗（ask 等级工具）

### Phase 2 — 设置页面（可独立验收）
- Agent 设置分区
- 工具授权配置持久化
- Skills 目录扫描与管理

### Phase 3 — MCP 集成（可独立验收）
- MCP 服务器添加 / 删除 / 连接管理
- MCP 工具动态注册到 Agent
- MCP 工具出现在授权配置列表

---

## 不在本设计范围内

- Web 搜索工具（WebSearchTool）
- PDF / Jupyter Notebook 读取
- Sub-agent 并发执行
- LSP 集成
- Git 工具集成
- 计划模式（plan-only mode）
