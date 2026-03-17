# yoagent Phase 0 — 后端核心集成 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 yoagent 嵌入为 Cargo workspace member，完成数据库迁移，实现 `agent_chat` / `agent_stop` / `agent_authorize_tool` 三个 Tauri commands，前端调用后可看到 Agent 工具调用的流式事件（无前端 UI，通过日志验收）。

**Architecture:** yoagent 源码克隆到 `src-tauri/crates/yoagent/`，`src-tauri/Cargo.toml` 转为 Cargo workspace，新建 `src-tauri/src/agent/` 模块负责：从 DB 读取 provider 配置注入 yoagent、将 AgentEvent 映射为 ChatEvent、将工具调用消息持久化到 SQLite、工具授权 InputFilter。

**Tech Stack:** Rust / Tauri v2, yoagent v0.7.0, rusqlite, tokio (watch + oneshot), Cargo workspace

**实现注意事项（代码审查反馈）：**
- 本计划中的代码是**示意代码**，实现时**必须**对照项目实际 API 调整：
  - 错误模块可能是 `crate::error` 而非 `crate::errors`，按实际编译错误修正
  - DB 操作可能是 `db.with_conn(|conn| {...})` 而非 `db.get_conn()?`，参照现有代码
  - `Database` 可能不实现 `Clone`，需使用 `Arc<Database>` 或引用
  - `Channel<ChatEvent>` 可能不实现 `Clone`，需用 `Arc` 包装或改用 channel sender
- 需新增 `AppError::Agent(String)` 和 `AppError::Mcp(String)` 变体到错误枚举
- `agent_stop` 应只清理指定 conversation 的 pending_auth，不是全局清理
- `agent_chat` 中 `tool_msg_ids` 需用 `Arc<Mutex<HashMap>>` 共享，不能 clone（clone 会丢失状态）
- migrations 中需添加 `working_dir` 默认值的 INSERT
- FTS 触发器除了 INSERT 还需处理 UPDATE/DELETE 对 `message_type` 的过滤

---

## 文件结构

### 新建文件
- `src-tauri/crates/yoagent/` — yoagent 源码（git clone）
- `src-tauri/src/agent/mod.rs` — 模块入口，公开子模块
- `src-tauri/src/agent/commands.rs` — Tauri commands
- `src-tauri/src/agent/events.rs` — AgentEvent → ChatEvent 映射
- `src-tauri/src/agent/permissions.rs` — InputFilter 实现（工具授权）
- `src-tauri/src/agent/config.rs` — 从 Orion DB 构建 yoagent ProviderConfig
- `src-tauri/src/agent/storage.rs` — 工具调用消息写入 messages 表

### 修改文件
- `src-tauri/Cargo.toml` — 转为 workspace，添加 yoagent 成员和依赖
- `src-tauri/src/db/migrations.rs` — 添加新表和新列
- `src-tauri/src/models/message.rs` — 扩展 Message struct 和 ChatEvent
- `src-tauri/src/state.rs` — AppState 添加 pending_auth 和 session_tool_overrides
- `src-tauri/src/lib.rs` — 注册 agent commands，导入 agent 模块

---

## Chunk 0: yoagent 工作区配置

### Task 0: 克隆 yoagent 并配置 Cargo workspace

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/crates/yoagent/` (via git clone)

- [ ] **Step 1: 克隆 yoagent 源码到 crates 目录**

```bash
mkdir -p src-tauri/crates
git clone https://github.com/yologdev/yoagent src-tauri/crates/yoagent
# 查看 yoagent 版本
cat src-tauri/crates/yoagent/Cargo.toml | head -10
```

预期输出：看到 `name = "yoagent"` 和 `version = "0.7.*"`

- [ ] **Step 2: 确认 yoagent 的公开 API 存在**

```bash
grep -n "pub fn agent_loop\|pub struct AgentLoopConfig\|pub trait AgentTool\|pub trait InputFilter\|pub enum AgentEvent\|pub struct ProviderConfig" \
  src-tauri/crates/yoagent/src/**/*.rs | head -20
```

预期：找到上述类型定义。**记录实际 API 签名**，后续 Task 中的代码需基于真实签名调整。

- [ ] **Step 3: 将 src-tauri/Cargo.toml 转为 workspace**

读取当前 `src-tauri/Cargo.toml` 的 `[package]` 部分。在文件**顶部**添加 workspace 声明：

```toml
[workspace]
members = [".", "crates/yoagent"]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

同时在现有 `[dependencies]` 中将 yoagent 添加为 path 依赖：

```toml
yoagent = { path = "crates/yoagent" }
```

- [ ] **Step 4: 验证编译通过**

```bash
cd src-tauri && cargo check 2>&1 | tail -20
```

预期：`warning: ...` 没有 error。若有 dependency 冲突，需解决版本问题（yoagent 的依赖版本可能与 Orion 不同，优先使用较新版本）。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/crates/
git commit -m "chore: embed yoagent as cargo workspace member"
```

---

## Chunk 1: 数据库迁移

### Task 1: 扩展数据库 schema

**Files:**
- Modify: `src-tauri/src/db/migrations.rs`

- [ ] **Step 1: 查看当前 migrations.rs 末尾，确定追加位置**

```bash
tail -30 src-tauri/src/db/migrations.rs
```

- [ ] **Step 2: 在 `run()` 函数末尾追加新迁移**

找到 `run()` 函数的最后一个 `let _ = conn.execute(...)` 行，在其后添加：

```rust
    // Agent feature migrations

    // messages 表：工具调用支持
    let _ = conn.execute(
        "ALTER TABLE messages ADD COLUMN message_type TEXT NOT NULL DEFAULT 'text'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE messages ADD COLUMN tool_call_id TEXT",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE messages ADD COLUMN tool_name TEXT",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE messages ADD COLUMN tool_input TEXT",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE messages ADD COLUMN tool_error INTEGER NOT NULL DEFAULT 0",
        [],
    );

    // conversations 表：agent_mode 持久化
    let _ = conn.execute(
        "ALTER TABLE conversations ADD COLUMN agent_mode INTEGER NOT NULL DEFAULT 1",
        [],
    );

    // agent_settings 表：工具权限、MCP 配置等
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS agent_settings (
            key   TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
        );",
    )?;

    // 插入默认工具权限（若尚未存在）
    conn.execute(
        "INSERT OR IGNORE INTO agent_settings (key, value) VALUES ('tool_permissions', ?1)",
        rusqlite::params![serde_json::json!({
            "read_file": "auto",
            "list_files": "auto",
            "search": "auto",
            "edit_file": "ask",
            "write_file": "ask",
            "bash": "ask"
        }).to_string()],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO agent_settings (key, value) VALUES ('mcp_servers', '[]')",
        [],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO agent_settings (key, value) VALUES ('skills_dir', '')",
        [],
    )?;
```

- [ ] **Step 3: 修复 messages_fts 触发器以排除工具调用消息**

在同一文件中搜索 `messages_ai` 触发器定义，添加 `WHEN` 条件。找到类似：

```sql
CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
  INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;
```

改为（在 `execute_batch` 的 SQL 字符串中修改）：

```sql
CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages
  WHEN NEW.message_type = 'text'
BEGIN
  INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;
```

注意：若触发器已存在，SQLite 不允许直接 `CREATE OR REPLACE`，需先 `DROP TRIGGER IF EXISTS messages_ai` 再创建。在 `execute_batch` 中添加：

```sql
DROP TRIGGER IF EXISTS messages_ai;
CREATE TRIGGER messages_ai AFTER INSERT ON messages
  WHEN NEW.message_type = 'text'
BEGIN
  INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;
```

- [ ] **Step 4: 编译验证**

```bash
cd src-tauri && cargo check 2>&1 | grep -E "error|warning: unused" | head -20
```

- [ ] **Step 5: 编写迁移单元测试**

在 `migrations.rs` 末尾添加测试模块：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_migrations_run_idempotently() {
        let conn = Connection::open_in_memory().unwrap();
        // 第一次运行
        run(&conn).unwrap();
        // 第二次运行应不报错（所有迁移幂等）
        run(&conn).unwrap();
    }

    #[test]
    fn test_agent_tables_created() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();

        // agent_settings 表存在
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_settings WHERE key = 'tool_permissions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // messages 表有新列
        let cols: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(messages)").unwrap();
            stmt.query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };
        assert!(cols.contains(&"message_type".to_string()));
        assert!(cols.contains(&"tool_call_id".to_string()));
        assert!(cols.contains(&"agent_mode".to_string()) == false); // agent_mode is on conversations
    }

    #[test]
    fn test_conversations_agent_mode_column() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();

        let cols: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(conversations)").unwrap();
            stmt.query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };
        assert!(cols.contains(&"agent_mode".to_string()));
    }
}
```

- [ ] **Step 6: 运行迁移测试**

```bash
cd src-tauri && cargo test db::migrations::tests 2>&1 | tail -20
```

预期：`test result: ok. 3 passed`

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/db/migrations.rs
git commit -m "feat: agent DB migrations - messages, conversations, agent_settings tables"
```

---

## Chunk 2: 数据模型扩展

### Task 2: 扩展 Message struct 和 ChatEvent

**Files:**
- Modify: `src-tauri/src/models/message.rs`

- [ ] **Step 1: 读取当前 message.rs 全文**

```bash
cat src-tauri/src/models/message.rs
```

- [ ] **Step 2: 添加 MessageType 枚举和扩展 Message struct**

在 `Role` 枚举后（或文件合适位置）添加：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum MessageType {
    #[default]
    Text,
    ToolCall,
    ToolResult,
}
```

在 `Message` struct 的 `version_number` / `total_versions` 字段后追加：

```rust
    #[serde(default)]
    pub message_type: MessageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<String>,
    #[serde(default)]
    pub tool_error: bool,
```

- [ ] **Step 3: 扩展 ChatEvent 枚举**

在现有 `ChatEvent` 枚举的 `Error` 变体后添加：

```rust
    ToolCallStart {
        message_id: String,
        tool_call_id: String,
        tool_name: String,
        args: String,
    },
    ToolCallUpdate {
        message_id: String,
        tool_call_id: String,
        partial_result: String,
    },
    ToolCallEnd {
        message_id: String,
        tool_call_id: String,
        result: String,
        is_error: bool,
    },
    ToolAuthRequest {
        tool_call_id: String,
        tool_name: String,
        args: String,
    },
```

- [ ] **Step 4: 添加 AgentSettings 和 ToolPermissions 类型**

在同文件底部（或新建 `src-tauri/src/models/agent.rs`）添加：

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PermissionLevel {
    Auto,
    Ask,
    Deny,
}

impl Default for PermissionLevel {
    fn default() -> Self { PermissionLevel::Ask }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissions(pub HashMap<String, PermissionLevel>);

impl Default for ToolPermissions {
    fn default() -> Self {
        let mut map = HashMap::new();
        map.insert("read_file".to_string(), PermissionLevel::Auto);
        map.insert("list_files".to_string(), PermissionLevel::Auto);
        map.insert("search".to_string(), PermissionLevel::Auto);
        map.insert("edit_file".to_string(), PermissionLevel::Ask);
        map.insert("write_file".to_string(), PermissionLevel::Ask);
        map.insert("bash".to_string(), PermissionLevel::Ask);
        ToolPermissions(map)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AuthAction {
    Allow,
    AllowSession,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransport,
    pub command_or_url: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum McpTransport { Stdio, Http }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerStatus {
    pub config: McpServerConfig,
    pub connected: bool,
}
```

若新建了 `models/agent.rs`，在 `models/mod.rs` 中 `pub mod agent;`。

- [ ] **Step 5: 编译验证**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

预期：0 errors（可能有 unused import warnings）。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/models/
git commit -m "feat: extend Message, ChatEvent and add agent types (ToolPermissions, AuthAction)"
```

---

## Chunk 3: AppState 扩展

### Task 3: 在 AppState 中添加 Agent 状态字段

**Files:**
- Modify: `src-tauri/src/state.rs`

- [ ] **Step 1: 读取 state.rs 全文**

```bash
cat src-tauri/src/state.rs
```

- [ ] **Step 2: 添加 imports**

在文件顶部已有的 use 语句后追加：

```rust
use tokio::sync::oneshot;
use crate::models::agent::{AuthAction, PermissionLevel};
```

- [ ] **Step 3: 在 AppState struct 中添加新字段**

```rust
pub struct AppState {
    pub db: Database,
    pub data_dir: PathBuf,
    pub providers: Mutex<HashMap<String, Arc<dyn Provider>>>,
    pub cancel_tokens: Mutex<HashMap<String, watch::Sender<bool>>>,
    pub proxy_mode: Mutex<String>,
    // Agent 相关新增字段
    /// tool_call_id -> oneshot sender，用于工具授权弹窗等待
    pub pending_auth: Mutex<HashMap<String, oneshot::Sender<AuthAction>>>,
    /// (conversation_id, tool_name) -> PermissionLevel，会话级授权覆盖
    pub session_tool_overrides: Mutex<HashMap<(String, String), PermissionLevel>>,
}
```

- [ ] **Step 4: 更新 AppState::new() 或构造函数**

找到 AppState 的初始化位置（可能在 `lib.rs` 或 `state.rs`），添加新字段的初始化：

```rust
pending_auth: Mutex::new(HashMap::new()),
session_tool_overrides: Mutex::new(HashMap::new()),
```

- [ ] **Step 5: 添加辅助方法**

在 `impl AppState` 块中添加：

```rust
/// 响应工具授权请求
pub async fn resolve_auth(&self, tool_call_id: &str, action: AuthAction) -> bool {
    if let Some(tx) = self.pending_auth.lock().await.remove(tool_call_id) {
        tx.send(action).is_ok()
    } else {
        false
    }
}

/// 注册会话级工具授权覆盖
pub async fn set_session_override(
    &self,
    conv_id: &str,
    tool_name: &str,
    level: PermissionLevel,
) {
    self.session_tool_overrides
        .lock()
        .await
        .insert((conv_id.to_string(), tool_name.to_string()), level);
}

/// 获取会话级工具授权覆盖（若存在）
pub async fn get_session_override(
    &self,
    conv_id: &str,
    tool_name: &str,
) -> Option<PermissionLevel> {
    self.session_tool_overrides
        .lock()
        .await
        .get(&(conv_id.to_string(), tool_name.to_string()))
        .cloned()
}
```

- [ ] **Step 6: 编译验证**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/state.rs
git commit -m "feat: extend AppState with agent pending_auth and session_tool_overrides"
```

---

## Chunk 4: agent/ 模块 — config 和 storage

### Task 4: agent/config.rs — 从 DB 构建 yoagent ProviderConfig

**Files:**
- Create: `src-tauri/src/agent/mod.rs`
- Create: `src-tauri/src/agent/config.rs`

- [ ] **Step 1: 查看 yoagent ProviderConfig 的实际定义**

```bash
grep -n "pub struct ProviderConfig\|pub enum ProviderType\|ProviderKind" \
  src-tauri/crates/yoagent/src/**/*.rs | head -20
```

记录实际字段名（以下代码基于 spec，需按实际 API 调整）。

- [ ] **Step 2: 查看 Orion 现有 Provider 和 Model 的 DB 查询方式**

```bash
grep -n "fn get_provider\|fn get_model\|SELECT.*providers\|SELECT.*models" \
  src-tauri/src/db/*.rs | head -20
```

了解如何从 DB 读取 provider 配置（api_key、base_url、type 等）。

- [ ] **Step 3: 创建 agent/mod.rs**

```rust
// src-tauri/src/agent/mod.rs
pub mod commands;
pub mod config;
pub mod events;
pub mod permissions;
pub mod storage;
```

- [ ] **Step 4: 创建 agent/config.rs**

```rust
// src-tauri/src/agent/config.rs
use yoagent::ProviderConfig;
use crate::db::Database;
use crate::errors::AppError;

/// 根据 model_id 从 Orion DB 读取配置，构建 yoagent ProviderConfig
pub async fn build_provider_config(
    db: &Database,
    model_id: &str,
) -> Result<ProviderConfig, AppError> {
    // 查询 models + providers 联表，获取 api_key, base_url, provider type
    let conn = db.get_conn()?; // 按实际 DB API 调整

    // 示例查询（按项目实际 schema 调整列名）
    let (provider_type, api_key, base_url, model_name): (String, String, String, String) =
        conn.query_row(
            "SELECT p.type, p.api_key, p.base_url, m.name
             FROM models m
             JOIN providers p ON m.provider_id = p.id
             WHERE m.id = ?1 AND p.is_enabled = 1",
            rusqlite::params![model_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

    // 根据 provider_type 映射到 yoagent ProviderConfig
    // 注意：需按 yoagent 实际 ProviderConfig 结构调整
    let config = ProviderConfig {
        // 按 yoagent 的实际字段名填充
        // 这里是示意，实际需查阅 yoagent/src/provider/ 中的类型
        api_key,
        base_url: if base_url.is_empty() { None } else { Some(base_url) },
        model: model_name,
        // provider kind 根据 provider_type 字符串映射
        ..Default::default()
    };

    Ok(config)
}

#[cfg(test)]
mod tests {
    // config 逻辑依赖 DB，集成测试在 Task 9 中验收
}
```

**注意**：此文件中的具体字段名**必须**在 Step 1 查看 yoagent API 后调整为实际字段。

- [ ] **Step 5: 编译验证**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

### Task 5: agent/storage.rs — 工具调用消息持久化

**Files:**
- Create: `src-tauri/src/agent/storage.rs`

- [ ] **Step 1: 查看现有 Message 的 DB 插入逻辑**

```bash
grep -n "INSERT INTO messages\|fn create_message\|fn insert_message" \
  src-tauri/src/db/*.rs | head -20
```

了解现有消息插入的参数和 SQL。

- [ ] **Step 2: 编写 storage.rs 的失败测试**

```rust
// src-tauri/src/agent/storage.rs
use crate::db::Database;
use crate::models::message::{Message, MessageType, Role, MessageStatus};
use crate::errors::AppError;
use uuid::Uuid;

pub struct ToolCallRecord {
    pub message_id: String,
    pub conversation_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub tool_input: String,
}

/// 在工具调用开始时插入 tool_call 消息行，返回 message_id
pub fn insert_tool_call_start(
    db: &Database,
    conversation_id: &str,
    tool_call_id: &str,
    tool_name: &str,
    tool_input: &str,
) -> Result<String, AppError> {
    let message_id = Uuid::new_v4().to_string();
    let conn = db.get_conn()?;

    conn.execute(
        "INSERT INTO messages
            (id, conversation_id, role, content, message_type, tool_call_id, tool_name, tool_input, status, created_at, version_group_id, version_number)
         VALUES (?1, ?2, 'assistant', '', 'tool_call', ?3, ?4, ?5, 'completed', datetime('now'), ?1, 1)",
        rusqlite::params![message_id, conversation_id, tool_call_id, tool_name, tool_input],
    ).map_err(|e| AppError::Database(e.to_string()))?;

    Ok(message_id)
}

/// 工具调用完成时插入 tool_result 消息行
pub fn insert_tool_call_result(
    db: &Database,
    conversation_id: &str,
    tool_call_id: &str,
    result: &str,
    is_error: bool,
) -> Result<String, AppError> {
    let message_id = Uuid::new_v4().to_string();
    let conn = db.get_conn()?;

    conn.execute(
        "INSERT INTO messages
            (id, conversation_id, role, content, message_type, tool_call_id, tool_error, status, created_at, version_group_id, version_number)
         VALUES (?1, ?2, 'tool', ?3, 'tool_result', ?4, ?5, 'completed', datetime('now'), ?1, 1)",
        rusqlite::params![message_id, conversation_id, result, tool_call_id, is_error as i32],
    ).map_err(|e| AppError::Database(e.to_string()))?;

    Ok(message_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn setup_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        crate::db::migrations::run(&db.get_conn().unwrap()).unwrap();
        // 插入必要的测试 conversation
        db.get_conn().unwrap().execute(
            "INSERT INTO conversations (id, title, created_at, updated_at) VALUES ('test-conv', 'test', datetime('now'), datetime('now'))",
            [],
        ).unwrap();
        db
    }

    #[test]
    fn test_insert_tool_call_start_returns_id() {
        let db = setup_db();
        let msg_id = insert_tool_call_start(
            &db,
            "test-conv",
            "call-123",
            "read_file",
            r#"{"path":"src/main.rs"}"#,
        ).unwrap();
        assert!(!msg_id.is_empty());

        let count: i64 = db.get_conn().unwrap()
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE tool_call_id = 'call-123' AND message_type = 'tool_call'",
                [],
                |r| r.get(0),
            ).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_insert_tool_result() {
        let db = setup_db();
        insert_tool_call_start(&db, "test-conv", "call-456", "bash", "{}").unwrap();
        let result_id = insert_tool_call_result(
            &db, "test-conv", "call-456", "exit 0", false,
        ).unwrap();
        assert!(!result_id.is_empty());

        let is_error: i32 = db.get_conn().unwrap()
            .query_row(
                "SELECT tool_error FROM messages WHERE id = ?1",
                rusqlite::params![result_id],
                |r| r.get(0),
            ).unwrap();
        assert_eq!(is_error, 0);
    }

    #[test]
    fn test_tool_result_not_indexed_in_fts() {
        let db = setup_db();
        insert_tool_call_start(&db, "test-conv", "call-789", "bash", "{}").unwrap();
        insert_tool_call_result(&db, "test-conv", "call-789", "some output", false).unwrap();

        // FTS 索引中不应包含工具调用消息
        let count: i64 = db.get_conn().unwrap()
            .query_row(
                "SELECT COUNT(*) FROM messages_fts WHERE content MATCH 'some output'",
                [],
                |r| r.get(0),
            ).unwrap();
        assert_eq!(count, 0);
    }
}
```

- [ ] **Step 3: 运行测试（预期失败，因 Database API 未知）**

```bash
cd src-tauri && cargo test agent::storage::tests 2>&1 | tail -20
```

根据实际错误调整 `db.get_conn()` 调用方式，对照现有 DB 代码修正 API。

- [ ] **Step 4: 修复实现直到测试通过**

```bash
cd src-tauri && cargo test agent::storage::tests 2>&1 | tail -5
```

预期：`test result: ok. 3 passed`

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/agent/
git commit -m "feat: agent/storage - tool_call and tool_result message persistence with tests"
```

---

## Chunk 5: agent/ 模块 — permissions 和 events

### Task 6: agent/permissions.rs — 工具授权 InputFilter

**Files:**
- Create: `src-tauri/src/agent/permissions.rs`

- [ ] **Step 1: 查看 yoagent InputFilter trait 的实际定义**

```bash
grep -n "pub trait InputFilter\|fn before_tool\|FilterResult\|async fn before" \
  src-tauri/crates/yoagent/src/**/*.rs | head -20
```

记录实际 trait 方法签名，调整下方代码。

- [ ] **Step 2: 编写失败测试**

```rust
// src-tauri/src/agent/permissions.rs
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};
use tauri::ipc::Channel;
use yoagent::{InputFilter, FilterResult};
use crate::models::message::ChatEvent;
use crate::models::agent::{AuthAction, PermissionLevel, ToolPermissions};
use crate::state::AppState;

pub struct PermissionFilter {
    pub state: Arc<AppState>,
    pub conv_id: String,
    pub channel: Channel<ChatEvent>,
    pub permissions: ToolPermissions,
}

// 注意：async_trait 宏可能需要根据 yoagent 的实际 trait 定义调整
#[async_trait::async_trait]
impl InputFilter for PermissionFilter {
    async fn before_tool(
        &self,
        tool_name: &str,
        args: &str,
        tool_call_id: &str,
    ) -> FilterResult {
        // 1. 检查 session-level 覆盖
        if let Some(level) = self.state
            .get_session_override(&self.conv_id, tool_name)
            .await
        {
            return match level {
                PermissionLevel::Auto => FilterResult::Allow,
                PermissionLevel::Deny => FilterResult::Deny("工具已被禁用".to_string()),
                PermissionLevel::Ask => unreachable!("session override 不应为 Ask"),
            };
        }

        // 2. 回退到全局配置
        let level = self.permissions.0
            .get(tool_name)
            .cloned()
            .unwrap_or(PermissionLevel::Ask);

        match level {
            PermissionLevel::Auto => FilterResult::Allow,
            PermissionLevel::Deny => FilterResult::Deny("工具已被禁用".to_string()),
            PermissionLevel::Ask => {
                let (tx, rx) = oneshot::channel::<AuthAction>();
                self.state
                    .pending_auth
                    .lock()
                    .await
                    .insert(tool_call_id.to_string(), tx);

                let _ = self.channel.send(ChatEvent::ToolAuthRequest {
                    tool_call_id: tool_call_id.to_string(),
                    tool_name: tool_name.to_string(),
                    args: args.to_string(),
                });

                tokio::select! {
                    Ok(action) = rx => {
                        if action == AuthAction::AllowSession {
                            self.state
                                .set_session_override(
                                    &self.conv_id,
                                    tool_name,
                                    PermissionLevel::Auto,
                                )
                                .await;
                        }
                        match action {
                            AuthAction::Allow | AuthAction::AllowSession => FilterResult::Allow,
                            AuthAction::Deny => FilterResult::Deny("用户拒绝".to_string()),
                        }
                    },
                    _ = tokio::time::sleep(Duration::from_secs(300)) => {
                        self.state.pending_auth.lock().await.remove(tool_call_id);
                        FilterResult::Deny("授权超时".to_string())
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 3: 编译验证，调整 InputFilter trait 方法名/签名**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

按编译错误调整 `before_tool` 方法签名，使其符合 yoagent 的实际 InputFilter trait 定义。

### Task 7: agent/events.rs — AgentEvent → ChatEvent 映射

**Files:**
- Create: `src-tauri/src/agent/events.rs`

- [ ] **Step 1: 查看 yoagent AgentEvent 的实际变体**

```bash
grep -n "pub enum AgentEvent\|TextDelta\|ToolCallStart\|ToolCallUpdate\|ToolCallComplete\|ToolCallEnd" \
  src-tauri/crates/yoagent/src/**/*.rs | head -20
```

- [ ] **Step 2: 编写事件映射（按实际变体调整）**

```rust
// src-tauri/src/agent/events.rs
use tauri::ipc::Channel;
use yoagent::AgentEvent;
use crate::models::message::ChatEvent;
use crate::agent::storage::{insert_tool_call_start, insert_tool_call_result};
use crate::db::Database;

/// 将 yoagent AgentEvent 转换为 ChatEvent 并发送到前端 Channel
/// 同时处理 ToolCallStart/End 的 DB 持久化
pub async fn handle_agent_event(
    event: AgentEvent,
    channel: &Channel<ChatEvent>,
    db: &Database,
    conversation_id: &str,
    // 追踪 tool_call_id -> message_id 的映射（ToolCallStart 写入，ToolCallEnd 使用）
    tool_msg_ids: &mut std::collections::HashMap<String, String>,
) {
    // 注意：以下变体名称是基于 spec 的估计，需按 yoagent 实际 AgentEvent 调整
    match event {
        // LLM 文字输出
        AgentEvent::TextDelta { content, message_id } => {
            let _ = channel.send(ChatEvent::Delta {
                message_id,
                content,
            });
        }

        // 工具调用开始
        AgentEvent::ToolCallStart { tool_call_id, tool_name, args } => {
            match insert_tool_call_start(db, conversation_id, &tool_call_id, &tool_name, &args) {
                Ok(msg_id) => {
                    tool_msg_ids.insert(tool_call_id.clone(), msg_id.clone());
                    let _ = channel.send(ChatEvent::ToolCallStart {
                        message_id: msg_id,
                        tool_call_id,
                        tool_name,
                        args,
                    });
                }
                Err(e) => {
                    eprintln!("Failed to persist tool_call_start: {e}");
                }
            }
        }

        // 工具输出流式更新（仅转发前端，不写 DB）
        AgentEvent::ToolCallUpdate { tool_call_id, partial_result } => {
            if let Some(msg_id) = tool_msg_ids.get(&tool_call_id) {
                let _ = channel.send(ChatEvent::ToolCallUpdate {
                    message_id: msg_id.clone(),
                    tool_call_id,
                    partial_result,
                });
            }
        }

        // 工具调用完成
        AgentEvent::ToolCallEnd { tool_call_id, result, is_error } |
        AgentEvent::ToolCallComplete { tool_call_id, result, is_error } => {
            let msg_id = tool_msg_ids.get(&tool_call_id).cloned().unwrap_or_default();
            match insert_tool_call_result(db, conversation_id, &tool_call_id, &result, is_error) {
                Ok(result_msg_id) => {
                    let _ = channel.send(ChatEvent::ToolCallEnd {
                        message_id: result_msg_id,
                        tool_call_id,
                        result,
                        is_error,
                    });
                }
                Err(e) => eprintln!("Failed to persist tool_result: {e}"),
            }
        }

        // 其他事件按需处理
        _ => {}
    }
}
```

**重要**：`AgentEvent` 的变体名**必须**根据 Step 1 的实际查询结果修改。

- [ ] **Step 3: 编译验证**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/agent/events.rs src-tauri/src/agent/permissions.rs
git commit -m "feat: agent/events AgentEvent→ChatEvent mapping, agent/permissions InputFilter"
```

---

## Chunk 6: Tauri Commands 和注册

### Task 8: agent/commands.rs — 实现 Tauri Commands

**Files:**
- Create: `src-tauri/src/agent/commands.rs`

- [ ] **Step 1: 查看现有 send_message 的实现结构作为参照**

```bash
sed -n '1,50p' src-tauri/src/commands/chat.rs
```

- [ ] **Step 2: 实现 agent commands**

```rust
// src-tauri/src/agent/commands.rs
use std::sync::Arc;
use std::collections::HashMap;
use tauri::{State, ipc::Channel};
use yoagent::{agent_loop, AgentLoopConfig};
use yoagent::tools::{BashTool, ReadFileTool, WriteFileTool, EditFileTool, ListFilesTool, SearchTool};
use crate::state::AppState;
use crate::models::message::{ChatEvent, Message, MessageType, Role, MessageStatus};
use crate::models::agent::{AuthAction, ToolPermissions};
use crate::errors::{AppResult, AppError};
use crate::agent::{config, permissions, events};
use crate::db::Database;
use uuid::Uuid;

/// 启动 Agent 对话
#[tauri::command]
pub async fn agent_chat(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    message: String,
    model_id: String,
    channel: Channel<ChatEvent>,
) -> AppResult<Message> {
    // 1. 构建 yoagent ProviderConfig
    let provider_config = config::build_provider_config(&state.db, &model_id).await?;

    // 2. 加载工具权限配置
    let permissions_json: String = state.db.get_conn()?
        .query_row(
            "SELECT value FROM agent_settings WHERE key = 'tool_permissions'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| serde_json::to_string(&ToolPermissions::default()).unwrap());
    let tool_perms: ToolPermissions = serde_json::from_str(&permissions_json)
        .unwrap_or_default();

    // 3. 读取工作目录
    let working_dir: String = state.db.get_conn()?
        .query_row(
            "SELECT value FROM agent_settings WHERE key = 'working_dir'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| dirs::home_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string());

    // 4. 加载历史消息（从 DB，含工具调用历史）
    // 按实际 DB API 调整
    let history = load_conversation_messages(&state.db, &conversation_id)?;

    // 5. 持久化用户消息
    let user_msg_id = Uuid::new_v4().to_string();
    // 按实际 insert_message API 调整
    insert_user_message(&state.db, &conversation_id, &user_msg_id, &message)?;

    // 6. 创建 cancel token（复用现有 watch channel 机制）
    let cancel_rx = state.create_cancel_token(&conversation_id).await;
    // 将 watch::Receiver<bool> 转换为 yoagent 的 CancellationToken
    let cancel_token = watch_to_cancellation_token(cancel_rx);

    // 7. 构建内置工具列表
    let tools: Vec<Box<dyn yoagent::AgentTool>> = vec![
        Box::new(BashTool::new(&working_dir)),
        Box::new(ReadFileTool::new(&working_dir)),
        Box::new(WriteFileTool::new(&working_dir)),
        Box::new(EditFileTool::new(&working_dir)),
        Box::new(ListFilesTool::new(&working_dir)),
        Box::new(SearchTool::new(&working_dir)),
    ];

    // 8. 构建 PermissionFilter
    let permission_filter = permissions::PermissionFilter {
        state: Arc::clone(&state),
        conv_id: conversation_id.clone(),
        channel: channel.clone(),
        permissions: tool_perms,
    };

    // 9. 运行 agent loop
    let db = state.db.clone();
    let conv_id = conversation_id.clone();
    let mut tool_msg_ids: HashMap<String, String> = HashMap::new();
    let ch = channel.clone();

    // 助手消息 ID（最终文本消息）
    let assistant_msg_id = Uuid::new_v4().to_string();
    let _ = channel.send(ChatEvent::Started {
        message_id: assistant_msg_id.clone(),
    });

    let config = AgentLoopConfig {
        system_prompt: "You are a helpful assistant with access to tools.".to_string(),
        messages: history,
        tools,
        provider: provider_config,
        cancel_token,
        input_filter: Some(Box::new(permission_filter)),
    };

    let result_messages = agent_loop(config, move |event| {
        // on_event callback — 注意这是同步回调，需要 block_on 或 spawn
        // 按 yoagent 实际回调签名调整
        let db = db.clone();
        let conv_id = conv_id.clone();
        let ch = ch.clone();
        let tool_ids = tool_msg_ids.clone(); // 简化处理
        tokio::spawn(async move {
            let mut ids = tool_ids;
            events::handle_agent_event(event, &ch, &db, &conv_id, &mut ids).await;
        });
    }).await.map_err(|e| AppError::Agent(e.to_string()))?;

    // 10. 持久化最终 assistant 消息
    let final_text = extract_final_text(&result_messages);
    let assistant_msg = insert_assistant_message(
        &state.db,
        &conversation_id,
        &assistant_msg_id,
        &final_text,
        &model_id,
    )?;

    state.remove_cancel_token(&conversation_id).await;
    let _ = channel.send(ChatEvent::Finished {
        message_id: assistant_msg_id,
    });

    Ok(assistant_msg)
}

/// 停止 Agent（取消 + 拒绝所有 pending 授权请求）
#[tauri::command]
pub async fn agent_stop(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
) -> AppResult<()> {
    state.cancel_conversation(&conversation_id).await;
    // 清理 pending auth requests
    state.pending_auth.lock().await.retain(|_, _| false);
    Ok(())
}

/// 用户响应工具授权弹窗
#[tauri::command]
pub async fn agent_authorize_tool(
    state: State<'_, Arc<AppState>>,
    tool_call_id: String,
    action: AuthAction,
) -> AppResult<()> {
    let resolved = state.resolve_auth(&tool_call_id, action).await;
    if !resolved {
        return Err(AppError::NotFound(format!("No pending auth for {tool_call_id}")));
    }
    Ok(())
}

/// 读取工具权限配置
#[tauri::command]
pub async fn get_tool_permissions(
    state: State<'_, Arc<AppState>>,
) -> AppResult<ToolPermissions> {
    let json: String = state.db.get_conn()?
        .query_row(
            "SELECT value FROM agent_settings WHERE key = 'tool_permissions'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    serde_json::from_str(&json).map_err(|e| AppError::Serialization(e.to_string()))
}

/// 写入工具权限配置
#[tauri::command]
pub async fn set_tool_permissions(
    state: State<'_, Arc<AppState>>,
    perms: ToolPermissions,
) -> AppResult<()> {
    let json = serde_json::to_string(&perms)
        .map_err(|e| AppError::Serialization(e.to_string()))?;
    state.db.get_conn()?
        .execute(
            "INSERT OR REPLACE INTO agent_settings (key, value) VALUES ('tool_permissions', ?1)",
            rusqlite::params![json],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

// ── 内部辅助函数（按项目实际 DB API 实现）──

fn load_conversation_messages(
    db: &Database,
    conversation_id: &str,
) -> AppResult<Vec<yoagent::Message>> {
    // 读取 messages 表，将 Orion Message 转换为 yoagent Message
    // 包含 tool_call / tool_result 类型的消息，以正确重组工具调用历史
    todo!("按实际 DB API 实现")
}

fn insert_user_message(
    db: &Database,
    conversation_id: &str,
    message_id: &str,
    content: &str,
) -> AppResult<()> {
    todo!("复用现有 insert_message 逻辑")
}

fn insert_assistant_message(
    db: &Database,
    conversation_id: &str,
    message_id: &str,
    content: &str,
    model_id: &str,
) -> AppResult<Message> {
    todo!("复用现有 insert_message 逻辑")
}

fn extract_final_text(messages: &[yoagent::Message]) -> String {
    // 从 yoagent 返回的消息列表中提取最后一条 assistant 文本消息
    messages.iter()
        .rev()
        .find_map(|m| {
            // 按 yoagent Message 实际 API 调整
            if m.role == yoagent::Role::Assistant {
                Some(m.content_text().unwrap_or_default())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn watch_to_cancellation_token(
    mut rx: tokio::sync::watch::Receiver<bool>,
) -> tokio_util::sync::CancellationToken {
    let token = tokio_util::sync::CancellationToken::new();
    let t = token.clone();
    tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            if *rx.borrow() {
                t.cancel();
                break;
            }
        }
    });
    token
}
```

**重要注意事项：**
- `todo!()` 标注的函数需要按实际 DB API 实现，对照 `src-tauri/src/db/` 中现有的 message 操作
- `watch_to_cancellation_token` 需要在 Cargo.toml 中添加 `tokio-util` 依赖（若 yoagent 已依赖则直接复用）
- yoagent 的 `agent_loop` 回调签名需按实际 API 调整（可能是 async closure 或 channel）

- [ ] **Step 3: 编译验证并实现 todo!()**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -30
```

逐一实现 `todo!()` 函数，参照 `src-tauri/src/commands/chat.rs` 中现有的消息持久化逻辑。

### Task 9: 注册 agent commands 到 Tauri

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 lib.rs 中添加 agent 模块**

找到 `mod commands;` 或类似声明，添加：

```rust
mod agent;
```

- [ ] **Step 2: 注册 commands**

找到 `tauri::generate_handler!` 宏，添加 agent commands：

```rust
tauri::generate_handler![
    // ... 现有 commands ...
    agent::commands::agent_chat,
    agent::commands::agent_stop,
    agent::commands::agent_authorize_tool,
    agent::commands::get_tool_permissions,
    agent::commands::set_tool_permissions,
]
```

- [ ] **Step 3: 完整编译**

```bash
cd src-tauri && cargo build 2>&1 | tail -30
```

解决所有编译错误。

- [ ] **Step 4: 启动应用验收**

```bash
# 在项目根目录
pnpm tauri dev 2>&1 | head -50
```

在浏览器 DevTools Console 中执行：

```javascript
// 测试 get_tool_permissions
const { invoke } = window.__TAURI__.core;
const perms = await invoke('get_tool_permissions');
console.log(perms); // 应显示默认权限配置
```

预期：看到包含 `read_file: "auto"`, `bash: "ask"` 等的权限对象。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/agent/commands.rs src-tauri/src/lib.rs src-tauri/src/agent/mod.rs
git commit -m "feat: agent commands - agent_chat, agent_stop, agent_authorize_tool, permissions CRUD"
```

---

## Chunk 7: 端到端验收

### Task 10: 端到端冒烟测试

**Goal:** 通过 DevTools 直接调用 `agent_chat`，验证工具调用流程完整运作。

- [ ] **Step 1: 准备测试环境**

确保有一个可用的 LLM provider 配置（在 Orion 设置页已配置 API key）。启动应用：

```bash
pnpm tauri dev
```

- [ ] **Step 2: 在 DevTools Console 中测试 agent_chat**

```javascript
const { invoke, Channel } = window.__TAURI__.core;

// 创建 channel 监听事件
const events = [];
const channel = new Channel();
channel.onmessage = (event) => {
    events.push(event);
    console.log('AgentEvent:', JSON.stringify(event));
};

// 获取一个 conversation_id（从 UI 或创建新的）
const convId = 'test-agent-conv-1';

// 发送 agent_chat 请求
try {
    const result = await invoke('agent_chat', {
        conversationId: convId,
        message: '请列出当前目录的文件',
        modelId: '<替换为你的 model_id>',
        channel: channel,
    });
    console.log('Final message:', result);
    console.log('Total events:', events.length);
    console.log('Tool calls:', events.filter(e => e.type === 'toolCallStart'));
} catch (e) {
    console.error('Error:', e);
}
```

预期：
- 看到 `Started`、`Delta`、`ToolCallStart { tool_name: "list_files" }`、`ToolCallEnd`、`Finished` 等事件
- 由于 `list_files` 默认权限为 `auto`，不应看到 `ToolAuthRequest`

- [ ] **Step 3: 测试授权弹窗流程**

```javascript
// 先注册 channel 监听 ToolAuthRequest
channel.onmessage = async (event) => {
    console.log('Event:', event.type, event);
    if (event.type === 'toolAuthRequest') {
        console.log('Auth requested for:', event.toolName);
        // 模拟用户点击"允许"
        await invoke('agent_authorize_tool', {
            toolCallId: event.toolCallId,
            action: 'allow',
        });
    }
};

// 发送一个会触发 bash 工具的请求
await invoke('agent_chat', {
    conversationId: convId,
    message: '运行 echo hello',
    modelId: '<model_id>',
    channel: channel,
});
```

预期：看到 `ToolAuthRequest { tool_name: "bash" }`，然后在 `invoke('agent_authorize_tool', ...)` 后继续执行。

- [ ] **Step 4: 验证 DB 持久化**

```javascript
// 查询 messages 表验证工具调用记录
const messages = await invoke('get_messages', { conversationId: convId });
console.log(messages.filter(m => m.messageType !== 'text'));
// 应看到 tool_call 和 tool_result 类型的消息
```

- [ ] **Step 5: 最终提交**

```bash
git add -A
git commit -m "feat: Phase 0 complete - yoagent backend core integration"
```

---

## 验收标准

Phase 0 完成的标志：
- [ ] `cargo build` 无 error
- [ ] `cargo test agent::storage::tests` 全部通过
- [ ] `cargo test db::migrations::tests` 全部通过
- [ ] DevTools 中 `invoke('get_tool_permissions')` 返回正确默认配置
- [ ] 向 Agent 发送消息后，DevTools Console 显示完整的工具调用事件流
- [ ] `list_files` 工具（auto 权限）自动执行，不弹授权请求
- [ ] `bash` 工具（ask 权限）触发 `ToolAuthRequest` 事件
- [ ] DB 中可查询到 `message_type = 'tool_call'` 和 `'tool_result'` 的消息记录

---

## 已知 API 不确定性（实现前必须核查）

以下内容基于 spec 中的 API 契约，**实际实现时必须对照 yoagent 源码调整**：

1. `AgentEvent` 的实际变体名（`TextDelta`? `Delta`? `ToolCallComplete`? `ToolCallEnd`?）
2. `agent_loop` 的回调方式（同步 `on_event: Fn`? 异步? Channel?）
3. `ProviderConfig` 的实际字段名和构造方式
4. yoagent 内置工具的构造函数签名（`BashTool::new(dir)`?）
5. `InputFilter` trait 的实际方法签名
6. yoagent `Message` 类型与 Orion `Message` 之间的字段映射

**在开始 Chunk 4-6 之前，先完整阅读 `src-tauri/crates/yoagent/src/` 的主要文件。**
