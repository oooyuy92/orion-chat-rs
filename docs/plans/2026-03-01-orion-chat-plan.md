# Orion Chat 实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 用 Tauri v2 + Rust + Svelte 5 构建极致轻量的多模型 AI Chat 桌面客户端。

**Architecture:** Rust 重后端架构。所有业务逻辑（API 通信、流式解析、数据持久化、Provider 抽象）在 Rust 侧，Svelte 5 前端只做纯渲染层。通过 Tauri Commands + Channel<T> 通信。

**Tech Stack:** Tauri v2, Rust, Svelte 5 (SvelteKit SPA), rusqlite + FTS5, reqwest + eventsource-stream, TailwindCSS v4, bits-ui

**Design Doc:** `docs/plans/2026-03-01-orion-chat-design.md`

---

## Phase 1: 项目脚手架

### Task 1: 初始化 Tauri v2 + SvelteKit 项目

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src/app.html`
- Create: `src/routes/+layout.svelte`
- Create: `src/routes/+page.svelte`
- Create: `package.json`
- Create: `svelte.config.js`
- Create: `vite.config.ts`
- Create: `tsconfig.json`

**Step 1: 用 Tauri CLI 创建项目**

```bash
cd /home/cc/Github/orion-chat-rs
# 保留已有的 docs/ 和 .git/
pnpm create tauri-app --template sveltekit-ts --manager pnpm . --force
```

如果 `create tauri-app` 不支持在已有目录初始化，手动创建：

```bash
# 前端: SvelteKit
pnpm create svelte@latest . --template skeleton --types typescript
pnpm add -D @sveltejs/adapter-static
pnpm add -D @tauri-apps/cli@latest
```

**Step 2: 安装 Tauri v2 依赖**

```bash
cd /home/cc/Github/orion-chat-rs
pnpm add @tauri-apps/api@latest
pnpm add -D @tauri-apps/cli@latest
pnpm tauri init
```

**Step 3: 配置 SvelteKit 为 SPA 模式**

修改 `svelte.config.js`:

```javascript
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({ fallback: 'index.html' })
  }
};
```

**Step 4: 安装前端依赖**

```bash
pnpm add -D tailwindcss@latest @tailwindcss/vite
pnpm add bits-ui highlight.js marked katex @tanstack/svelte-virtual
```

**Step 5: 配置 Rust 依赖**

编辑 `src-tauri/Cargo.toml`，确保包含：

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-store = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["stream", "json"] }
eventsource-stream = "0.2"
rusqlite = { version = "0.32", features = ["bundled"] }
uuid = { version = "1", features = ["v4"] }
async-trait = "0.1"
thiserror = "2"
futures-util = "0.3"
```

**Step 6: 验证项目能编译运行**

```bash
cd /home/cc/Github/orion-chat-rs
pnpm tauri dev
```

Expected: Tauri 窗口打开，显示 SvelteKit 默认页面。

**Step 7: Commit**

```bash
git add -A
git commit -m "feat: scaffold Tauri v2 + SvelteKit + Rust project"
```

---

## Phase 2: Rust 数据模型 + 错误处理

### Task 2: 定义统一错误类型

**Files:**
- Create: `src-tauri/src/error.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: 创建 error.rs**

```rust
// src-tauri/src/error.rs
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Cancelled")]
    Cancelled,
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

**Step 2: 在 lib.rs 中声明模块**

```rust
// src-tauri/src/lib.rs
pub mod error;
```

**Step 3: 验证编译**

```bash
cd /home/cc/Github/orion-chat-rs/src-tauri && cargo check
```

Expected: 编译通过，无错误。

**Step 4: Commit**

```bash
git add src-tauri/src/error.rs src-tauri/src/lib.rs
git commit -m "feat: add unified error type with thiserror"
```

### Task 3: 定义核心数据模型

**Files:**
- Create: `src-tauri/src/models/mod.rs`
- Create: `src-tauri/src/models/provider.rs`
- Create: `src-tauri/src/models/message.rs`
- Create: `src-tauri/src/models/conversation.rs`
- Create: `src-tauri/src/models/assistant.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: 创建 models/provider.rs**

```rust
// src-tauri/src/models/provider.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    OpenaiCompat,
    Anthropic,
    Gemini,
    Ollama,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub r#type: ProviderType,
    pub api_key: Option<String>,
    pub base_url: String,
    pub proxy: Option<String>,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub display_name: Option<String>,
    pub max_tokens: Option<u32>,
    pub is_vision: bool,
    pub supports_thinking: bool,
    pub is_enabled: bool,
}

// --- 通用参数 ---
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonParams {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    #[serde(default = "default_true")]
    pub stream: bool,
}

fn default_true() -> bool { true }

// --- 供应商特有参数 ---
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider_type")]
pub enum ProviderParams {
    OpenaiCompat {
        #[serde(skip_serializing_if = "Option::is_none")]
        frequency_penalty: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        presence_penalty: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning_effort: Option<ReasoningEffort>,
        #[serde(skip_serializing_if = "Option::is_none")]
        seed: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_completion_tokens: Option<u32>,
    },
    Anthropic {
        #[serde(skip_serializing_if = "Option::is_none")]
        top_k: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        thinking: Option<AnthropicThinking>,
        #[serde(skip_serializing_if = "Option::is_none")]
        effort: Option<AnthropicEffort>,
    },
    Gemini {
        #[serde(skip_serializing_if = "Option::is_none")]
        thinking_budget: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        thinking_level: Option<GeminiThinkingLevel>,
    },
    Ollama {
        #[serde(skip_serializing_if = "Option::is_none")]
        think: Option<OllamaThink>,
        #[serde(skip_serializing_if = "Option::is_none")]
        num_ctx: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        repeat_penalty: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_p: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        keep_alive: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort { Low, Medium, High }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicThinking {
    Adaptive,
    Enabled { budget_tokens: u32 },
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnthropicEffort { Low, Medium, High, Max }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeminiThinkingLevel { Minimal, Low, Medium, High }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OllamaThink {
    Bool(bool),
    Level(String),
}
```

**Step 2: 创建 models/message.rs**

```rust
// src-tauri/src/models/message.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    Streaming,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: Role,
    pub content: String,
    pub model_id: Option<String>,
    pub reasoning: Option<String>,
    pub token_prompt: Option<u32>,
    pub token_completion: Option<u32>,
    pub status: MessageStatus,
    pub created_at: String,
}

/// 发送给 Provider 的聊天消息（精简版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

/// 流式事件，通过 Tauri Channel 推送到前端
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum ChatEvent {
    Started { message_id: String },
    Delta { content: String },
    Reasoning { content: String },
    Usage { prompt_tokens: u32, completion_tokens: u32 },
    Finished { message_id: String },
    Error { message: String },
}

/// Provider 接收的请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub common: super::provider::CommonParams,
    pub provider_params: super::provider::ProviderParams,
}
```

**Step 3: 创建 models/conversation.rs**

```rust
// src-tauri/src/models/conversation.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub assistant_id: Option<String>,
    pub model_id: Option<String>,
    pub is_pinned: bool,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}
```

**Step 4: 创建 models/assistant.rs**

```rust
// src-tauri/src/models/assistant.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assistant {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub system_prompt: Option<String>,
    pub model_id: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub extra_params: serde_json::Value,
    pub sort_order: i32,
    pub created_at: String,
}
```

**Step 5: 创建 models/mod.rs 并更新 lib.rs**

```rust
// src-tauri/src/models/mod.rs
pub mod assistant;
pub mod conversation;
pub mod message;
pub mod provider;

pub use assistant::*;
pub use conversation::*;
pub use message::*;
pub use provider::*;
```

在 `lib.rs` 中添加:

```rust
pub mod error;
pub mod models;
```

**Step 6: 编写模型单元测试**

在 `src-tauri/src/models/provider.rs` 底部添加:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_params_serde_openai() {
        let params = ProviderParams::OpenaiCompat {
            frequency_penalty: Some(0.5),
            presence_penalty: None,
            reasoning_effort: Some(ReasoningEffort::High),
            seed: None,
            max_completion_tokens: Some(4096),
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"provider_type\":\"OpenaiCompat\""));
        let deserialized: ProviderParams = serde_json::from_str(&json).unwrap();
        match deserialized {
            ProviderParams::OpenaiCompat { frequency_penalty, .. } => {
                assert_eq!(frequency_penalty, Some(0.5));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_anthropic_thinking_adaptive() {
        let thinking = AnthropicThinking::Adaptive;
        let json = serde_json::to_string(&thinking).unwrap();
        assert!(json.contains("\"type\":\"Adaptive\""));
    }

    #[test]
    fn test_ollama_think_untagged() {
        let think_bool: OllamaThink = serde_json::from_str("true").unwrap();
        matches!(think_bool, OllamaThink::Bool(true));

        let think_level: OllamaThink = serde_json::from_str("\"high\"").unwrap();
        matches!(think_level, OllamaThink::Level(_));
    }
}
```

**Step 7: 运行测试**

```bash
cd /home/cc/Github/orion-chat-rs/src-tauri && cargo test
```

Expected: 所有测试通过。

**Step 8: Commit**

```bash
git add src-tauri/src/models/
git commit -m "feat: add core data models with serde serialization"
```

---

## Phase 3: 数据库层

### Task 4: SQLite 初始化 + Schema 迁移

**Files:**
- Create: `src-tauri/src/db/mod.rs`
- Create: `src-tauri/src/db/migrations.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: 创建 db/mod.rs — 连接管理**

```rust
// src-tauri/src/db/mod.rs
pub mod migrations;
pub mod conversations;
pub mod messages;
pub mod assistants;

use rusqlite::Connection;
use std::sync::Mutex;
use crate::error::AppResult;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> AppResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn: Mutex::new(conn) };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn with_conn<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&Connection) -> AppResult<T>,
    {
        let conn = self.conn.lock().unwrap();
        f(&conn)
    }

    fn run_migrations(&self) -> AppResult<()> {
        self.with_conn(|conn| {
            migrations::run(conn)?;
            Ok(())
        })
    }
}
```

**Step 2: 创建 db/migrations.rs — Schema 定义**

```rust
// src-tauri/src/db/migrations.rs
use rusqlite::Connection;
use crate::error::AppResult;

pub fn run(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS providers (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            type        TEXT NOT NULL,
            api_key     TEXT,
            base_url    TEXT NOT NULL,
            proxy       TEXT,
            is_enabled  INTEGER NOT NULL DEFAULT 1,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS models (
            id                TEXT PRIMARY KEY,
            provider_id       TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
            name              TEXT NOT NULL,
            display_name      TEXT,
            max_tokens        INTEGER,
            is_vision         INTEGER NOT NULL DEFAULT 0,
            supports_thinking INTEGER NOT NULL DEFAULT 0,
            is_enabled        INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS assistants (
            id            TEXT PRIMARY KEY,
            name          TEXT NOT NULL,
            icon          TEXT,
            system_prompt TEXT,
            model_id      TEXT REFERENCES models(id),
            temperature   REAL,
            top_p         REAL,
            max_tokens    INTEGER,
            extra_params  TEXT DEFAULT '{}',
            sort_order    INTEGER NOT NULL DEFAULT 0,
            created_at    TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS conversations (
            id           TEXT PRIMARY KEY,
            title        TEXT NOT NULL,
            assistant_id TEXT REFERENCES assistants(id),
            model_id     TEXT REFERENCES models(id),
            is_pinned    INTEGER NOT NULL DEFAULT 0,
            sort_order   INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS messages (
            id               TEXT PRIMARY KEY,
            conversation_id  TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            role             TEXT NOT NULL,
            content          TEXT NOT NULL,
            model_id         TEXT,
            reasoning        TEXT,
            token_prompt     INTEGER,
            token_completion INTEGER,
            status           TEXT NOT NULL DEFAULT 'done',
            created_at       TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
            content,
            content=messages,
            content_rowid=rowid
        );

        CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
            INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
        END;

        CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content)
                VALUES('delete', old.rowid, old.content);
        END;
    ")?;
    Ok(())
}
```

**Step 3: 更新 lib.rs**

```rust
pub mod error;
pub mod models;
pub mod db;
```

**Step 4: 编写数据库初始化测试**

在 `db/mod.rs` 底部添加:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_init() {
        let db = Database::new(":memory:").unwrap();
        db.with_conn(|conn| {
            let count: i64 = conn.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='messages'",
                [],
                |row| row.get(0),
            )?;
            assert_eq!(count, 1);
            Ok(())
        }).unwrap();
    }

    #[test]
    fn test_fts5_table_created() {
        let db = Database::new(":memory:").unwrap();
        db.with_conn(|conn| {
            let count: i64 = conn.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='messages_fts'",
                [],
                |row| row.get(0),
            )?;
            assert_eq!(count, 1);
            Ok(())
        }).unwrap();
    }
}
```

**Step 5: 运行测试**

```bash
cd /home/cc/Github/orion-chat-rs/src-tauri && cargo test db::
```

Expected: 2 个测试通过。

**Step 6: Commit**

```bash
git add src-tauri/src/db/ src-tauri/src/lib.rs
git commit -m "feat: add SQLite database layer with schema migrations and FTS5"
```

### Task 5: 对话和消息 CRUD 操作

**Files:**
- Create: `src-tauri/src/db/conversations.rs`
- Create: `src-tauri/src/db/messages.rs`

**Step 1: 创建 db/conversations.rs**

```rust
// src-tauri/src/db/conversations.rs
use rusqlite::{params, Connection};
use crate::error::{AppError, AppResult};
use crate::models::Conversation;

pub fn create(conn: &Connection, conv: &Conversation) -> AppResult<()> {
    conn.execute(
        "INSERT INTO conversations (id, title, assistant_id, model_id, is_pinned, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![conv.id, conv.title, conv.assistant_id, conv.model_id, conv.is_pinned, conv.sort_order],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Conversation> {
    conn.query_row(
        "SELECT id, title, assistant_id, model_id, is_pinned, sort_order, created_at, updated_at
         FROM conversations WHERE id = ?1",
        params![id],
        |row| Ok(Conversation {
            id: row.get(0)?,
            title: row.get(1)?,
            assistant_id: row.get(2)?,
            model_id: row.get(3)?,
            is_pinned: row.get(4)?,
            sort_order: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        }),
    ).map_err(|_| AppError::NotFound(format!("Conversation {id} not found")))
}

pub fn list(conn: &Connection) -> AppResult<Vec<Conversation>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, assistant_id, model_id, is_pinned, sort_order, created_at, updated_at
         FROM conversations ORDER BY is_pinned DESC, updated_at DESC"
    )?;
    let rows = stmt.query_map([], |row| Ok(Conversation {
        id: row.get(0)?,
        title: row.get(1)?,
        assistant_id: row.get(2)?,
        model_id: row.get(3)?,
        is_pinned: row.get(4)?,
        sort_order: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    }))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn update_title(conn: &Connection, id: &str, title: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE conversations SET title = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![title, id],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn touch(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE conversations SET updated_at = datetime('now') WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}
```

**Step 2: 创建 db/messages.rs**

```rust
// src-tauri/src/db/messages.rs
use rusqlite::{params, Connection};
use crate::error::AppResult;
use crate::models::{Message, MessageStatus, Role};

pub fn create(conn: &Connection, msg: &Message) -> AppResult<()> {
    conn.execute(
        "INSERT INTO messages (id, conversation_id, role, content, model_id, reasoning, token_prompt, token_completion, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            msg.id,
            msg.conversation_id,
            serde_json::to_string(&msg.role).unwrap().trim_matches('"'),
            msg.content,
            msg.model_id,
            msg.reasoning,
            msg.token_prompt,
            msg.token_completion,
            serde_json::to_string(&msg.status).unwrap().trim_matches('"'),
        ],
    )?;
    Ok(())
}

pub fn list_by_conversation(conn: &Connection, conversation_id: &str) -> AppResult<Vec<Message>> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, role, content, model_id, reasoning, token_prompt, token_completion, status, created_at
         FROM messages WHERE conversation_id = ?1 ORDER BY created_at ASC"
    )?;
    let rows = stmt.query_map(params![conversation_id], |row| {
        let role_str: String = row.get(2)?;
        let status_str: String = row.get(8)?;
        Ok(Message {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            role: serde_json::from_str(&format!("\"{role_str}\"")).unwrap_or(Role::User),
            content: row.get(3)?,
            model_id: row.get(4)?,
            reasoning: row.get(5)?,
            token_prompt: row.get(6)?,
            token_completion: row.get(7)?,
            status: serde_json::from_str(&format!("\"{status_str}\"")).unwrap_or(MessageStatus::Done),
            created_at: row.get(9)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn update_content(conn: &Connection, id: &str, content: &str, reasoning: Option<&str>, token_prompt: Option<u32>, token_completion: Option<u32>) -> AppResult<()> {
    conn.execute(
        "UPDATE messages SET content = ?1, reasoning = ?2, token_prompt = ?3, token_completion = ?4, status = 'done' WHERE id = ?5",
        params![content, reasoning, token_prompt, token_completion, id],
    )?;
    Ok(())
}

pub fn set_error(conn: &Connection, id: &str, error_msg: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE messages SET content = ?1, status = 'error' WHERE id = ?2",
        params![error_msg, id],
    )?;
    Ok(())
}

pub fn search(conn: &Connection, query: &str) -> AppResult<Vec<Message>> {
    let mut stmt = conn.prepare(
        "SELECT m.id, m.conversation_id, m.role, m.content, m.model_id, m.reasoning, m.token_prompt, m.token_completion, m.status, m.created_at
         FROM messages m
         JOIN messages_fts fts ON m.rowid = fts.rowid
         WHERE messages_fts MATCH ?1
         ORDER BY rank"
    )?;
    let rows = stmt.query_map(params![query], |row| {
        let role_str: String = row.get(2)?;
        let status_str: String = row.get(8)?;
        Ok(Message {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            role: serde_json::from_str(&format!("\"{role_str}\"")).unwrap_or(Role::User),
            content: row.get(3)?,
            model_id: row.get(4)?,
            reasoning: row.get(5)?,
            token_prompt: row.get(6)?,
            token_completion: row.get(7)?,
            status: serde_json::from_str(&format!("\"{status_str}\"")).unwrap_or(MessageStatus::Done),
            created_at: row.get(9)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}
```

**Step 3: 编写 CRUD 测试**

在 `db/conversations.rs` 底部:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_conversation_crud() {
        let db = Database::new(":memory:").unwrap();
        db.with_conn(|conn| {
            let conv = Conversation {
                id: "conv-1".into(),
                title: "Test Chat".into(),
                assistant_id: None,
                model_id: None,
                is_pinned: false,
                sort_order: 0,
                created_at: String::new(),
                updated_at: String::new(),
            };
            create(conn, &conv)?;
            let fetched = get(conn, "conv-1")?;
            assert_eq!(fetched.title, "Test Chat");

            update_title(conn, "conv-1", "Renamed")?;
            let fetched = get(conn, "conv-1")?;
            assert_eq!(fetched.title, "Renamed");

            let all = list(conn)?;
            assert_eq!(all.len(), 1);

            delete(conn, "conv-1")?;
            let all = list(conn)?;
            assert_eq!(all.len(), 0);
            Ok(())
        }).unwrap();
    }
}
```

在 `db/messages.rs` 底部:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::Conversation;

    fn setup_db() -> Database {
        let db = Database::new(":memory:").unwrap();
        db.with_conn(|conn| {
            crate::db::conversations::create(conn, &Conversation {
                id: "conv-1".into(),
                title: "Test".into(),
                assistant_id: None,
                model_id: None,
                is_pinned: false,
                sort_order: 0,
                created_at: String::new(),
                updated_at: String::new(),
            })?;
            Ok(())
        }).unwrap();
        db
    }

    #[test]
    fn test_message_create_and_list() {
        let db = setup_db();
        db.with_conn(|conn| {
            let msg = Message {
                id: "msg-1".into(),
                conversation_id: "conv-1".into(),
                role: Role::User,
                content: "Hello world".into(),
                model_id: None,
                reasoning: None,
                token_prompt: None,
                token_completion: None,
                status: MessageStatus::Done,
                created_at: String::new(),
            };
            create(conn, &msg)?;
            let msgs = list_by_conversation(conn, "conv-1")?;
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0].content, "Hello world");
            Ok(())
        }).unwrap();
    }

    #[test]
    fn test_fts_search() {
        let db = setup_db();
        db.with_conn(|conn| {
            create(conn, &Message {
                id: "msg-1".into(),
                conversation_id: "conv-1".into(),
                role: Role::User,
                content: "Rust programming language".into(),
                model_id: None, reasoning: None,
                token_prompt: None, token_completion: None,
                status: MessageStatus::Done, created_at: String::new(),
            })?;
            create(conn, &Message {
                id: "msg-2".into(),
                conversation_id: "conv-1".into(),
                role: Role::Assistant,
                content: "Python is also great".into(),
                model_id: None, reasoning: None,
                token_prompt: None, token_completion: None,
                status: MessageStatus::Done, created_at: String::new(),
            })?;
            let results = search(conn, "Rust")?;
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].id, "msg-1");
            Ok(())
        }).unwrap();
    }
}
```

**Step 4: 运行测试**

```bash
cd /home/cc/Github/orion-chat-rs/src-tauri && cargo test
```

Expected: 所有测试通过。

**Step 5: Commit**

```bash
git add src-tauri/src/db/
git commit -m "feat: add conversation and message CRUD with FTS5 search"
```

### Task 6: 助手 CRUD 操作

**Files:**
- Create: `src-tauri/src/db/assistants.rs`

**Step 1: 创建 db/assistants.rs**

```rust
// src-tauri/src/db/assistants.rs
use rusqlite::{params, Connection};
use crate::error::{AppError, AppResult};
use crate::models::Assistant;

pub fn create(conn: &Connection, asst: &Assistant) -> AppResult<()> {
    conn.execute(
        "INSERT INTO assistants (id, name, icon, system_prompt, model_id, temperature, top_p, max_tokens, extra_params, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            asst.id, asst.name, asst.icon, asst.system_prompt, asst.model_id,
            asst.temperature, asst.top_p, asst.max_tokens,
            serde_json::to_string(&asst.extra_params).unwrap(),
            asst.sort_order,
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Assistant> {
    conn.query_row(
        "SELECT id, name, icon, system_prompt, model_id, temperature, top_p, max_tokens, extra_params, sort_order, created_at
         FROM assistants WHERE id = ?1",
        params![id],
        |row| {
            let extra_str: String = row.get(8)?;
            Ok(Assistant {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                system_prompt: row.get(3)?,
                model_id: row.get(4)?,
                temperature: row.get(5)?,
                top_p: row.get(6)?,
                max_tokens: row.get(7)?,
                extra_params: serde_json::from_str(&extra_str).unwrap_or_default(),
                sort_order: row.get(9)?,
                created_at: row.get(10)?,
            })
        },
    ).map_err(|_| AppError::NotFound(format!("Assistant {id} not found")))
}

pub fn list(conn: &Connection) -> AppResult<Vec<Assistant>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, icon, system_prompt, model_id, temperature, top_p, max_tokens, extra_params, sort_order, created_at
         FROM assistants ORDER BY sort_order ASC"
    )?;
    let rows = stmt.query_map([], |row| {
        let extra_str: String = row.get(8)?;
        Ok(Assistant {
            id: row.get(0)?,
            name: row.get(1)?,
            icon: row.get(2)?,
            system_prompt: row.get(3)?,
            model_id: row.get(4)?,
            temperature: row.get(5)?,
            top_p: row.get(6)?,
            max_tokens: row.get(7)?,
            extra_params: serde_json::from_str(&extra_str).unwrap_or_default(),
            sort_order: row.get(9)?,
            created_at: row.get(10)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn update(conn: &Connection, asst: &Assistant) -> AppResult<()> {
    conn.execute(
        "UPDATE assistants SET name=?1, icon=?2, system_prompt=?3, model_id=?4, temperature=?5, top_p=?6, max_tokens=?7, extra_params=?8, sort_order=?9 WHERE id=?10",
        params![
            asst.name, asst.icon, asst.system_prompt, asst.model_id,
            asst.temperature, asst.top_p, asst.max_tokens,
            serde_json::to_string(&asst.extra_params).unwrap(),
            asst.sort_order, asst.id,
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM assistants WHERE id = ?1", params![id])?;
    Ok(())
}
```

**Step 2: 编写测试（在文件底部）**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_assistant_crud() {
        let db = Database::new(":memory:").unwrap();
        db.with_conn(|conn| {
            let asst = Assistant {
                id: "asst-1".into(),
                name: "Code Helper".into(),
                icon: None,
                system_prompt: Some("You are a coding assistant.".into()),
                model_id: None,
                temperature: Some(0.7),
                top_p: None,
                max_tokens: Some(4096),
                extra_params: serde_json::json!({}),
                sort_order: 0,
                created_at: String::new(),
            };
            create(conn, &asst)?;
            let fetched = get(conn, "asst-1")?;
            assert_eq!(fetched.name, "Code Helper");
            assert_eq!(fetched.temperature, Some(0.7));

            let all = list(conn)?;
            assert_eq!(all.len(), 1);

            delete(conn, "asst-1")?;
            assert_eq!(list(conn)?.len(), 0);
            Ok(())
        }).unwrap();
    }
}
```

**Step 3: 运行测试**

```bash
cd /home/cc/Github/orion-chat-rs/src-tauri && cargo test db::assistants
```

Expected: 测试通过。

**Step 4: Commit**

```bash
git add src-tauri/src/db/assistants.rs
git commit -m "feat: add assistant CRUD operations"
```

---

## Phase 4: Provider Trait + OpenAI 兼容实现

### Task 7: 定义 Provider Trait

**Files:**
- Create: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: 创建 providers/mod.rs**

```rust
// src-tauri/src/providers/mod.rs
pub mod openai_compat;
pub mod anthropic;
pub mod gemini;
pub mod ollama;

use async_trait::async_trait;
use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::error::AppResult;
use crate::models::{ChatEvent, ChatRequest, ModelInfo};

#[async_trait]
pub trait Provider: Send + Sync {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        channel: Channel<ChatEvent>,
        cancel: watch::Receiver<bool>,
    ) -> AppResult<()>;

    async fn list_models(&self) -> AppResult<Vec<ModelInfo>>;

    async fn validate(&self) -> AppResult<bool>;
}
```

**Step 2: 更新 lib.rs**

```rust
pub mod error;
pub mod models;
pub mod db;
pub mod providers;
```

**Step 3: 验证编译**

```bash
cd /home/cc/Github/orion-chat-rs/src-tauri && cargo check
```

Expected: 编译通过（provider 子模块暂时为空文件）。

**Step 4: Commit**

```bash
git add src-tauri/src/providers/mod.rs src-tauri/src/lib.rs
git commit -m "feat: define Provider trait for LLM abstraction"
```

### Task 8: 实现 OpenAI 兼容 Provider

**Files:**
- Create: `src-tauri/src/providers/openai_compat.rs`

**Step 1: 创建 openai_compat.rs**

这是最核心的 provider，DeepSeek、Groq、Together 等都走这个协议。

```rust
// src-tauri/src/providers/openai_compat.rs
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::error::{AppError, AppResult};
use crate::models::*;
use super::Provider;

pub struct OpenAICompatProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAICompatProvider {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    fn build_messages(&self, messages: &[ChatMessage]) -> Vec<serde_json::Value> {
        messages.iter().map(|m| {
            serde_json::json!({
                "role": m.role,
                "content": m.content,
            })
        }).collect()
    }

    fn build_body(&self, request: &ChatRequest) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": request.model,
            "messages": self.build_messages(&request.messages),
            "stream": request.common.stream,
        });
        let obj = body.as_object_mut().unwrap();

        if let Some(t) = request.common.temperature { obj.insert("temperature".into(), t.into()); }
        if let Some(p) = request.common.top_p { obj.insert("top_p".into(), p.into()); }
        if let Some(m) = request.common.max_tokens { obj.insert("max_tokens".into(), m.into()); }

        if let ProviderParams::OpenaiCompat {
            frequency_penalty, presence_penalty, reasoning_effort,
            seed, max_completion_tokens,
        } = &request.provider_params {
            if let Some(fp) = frequency_penalty { obj.insert("frequency_penalty".into(), (*fp).into()); }
            if let Some(pp) = presence_penalty { obj.insert("presence_penalty".into(), (*pp).into()); }
            if let Some(re) = reasoning_effort { obj.insert("reasoning_effort".into(), serde_json::to_value(re).unwrap()); }
            if let Some(s) = seed { obj.insert("seed".into(), (*s).into()); }
            if let Some(mct) = max_completion_tokens { obj.insert("max_completion_tokens".into(), (*mct).into()); }
        }
        body
    }
}

#[async_trait]
impl Provider for OpenAICompatProvider {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        channel: Channel<ChatEvent>,
        cancel: watch::Receiver<bool>,
    ) -> AppResult<()> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = self.build_body(&request);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!("HTTP {status}: {text}")));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            if *cancel.borrow() {
                return Err(AppError::Cancelled);
            }
            let bytes = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() || line == "data: [DONE]" { continue; }
                if !line.starts_with("data: ") { continue; }

                let json_str = &line[6..];
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let Some(choices) = data["choices"].as_array() {
                        for choice in choices {
                            let delta = &choice["delta"];
                            if let Some(content) = delta["content"].as_str() {
                                if !content.is_empty() {
                                    let _ = channel.send(ChatEvent::Delta { content: content.to_string() });
                                }
                            }
                            // reasoning_content for DeepSeek R1 / o-series
                            if let Some(reasoning) = delta["reasoning_content"].as_str() {
                                if !reasoning.is_empty() {
                                    let _ = channel.send(ChatEvent::Reasoning { content: reasoning.to_string() });
                                }
                            }
                        }
                    }
                    // Usage in final chunk
                    if let Some(usage) = data.get("usage") {
                        if let (Some(pt), Some(ct)) = (usage["prompt_tokens"].as_u64(), usage["completion_tokens"].as_u64()) {
                            let _ = channel.send(ChatEvent::Usage {
                                prompt_tokens: pt as u32,
                                completion_tokens: ct as u32,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn list_models(&self) -> AppResult<Vec<ModelInfo>> {
        let url = format!("{}/v1/models", self.base_url);
        let resp = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send().await?;
        let body: serde_json::Value = resp.json().await?;
        let models = body["data"].as_array()
            .map(|arr| arr.iter().filter_map(|m| {
                Some(ModelInfo {
                    id: m["id"].as_str()?.to_string(),
                    provider_id: String::new(),
                    name: m["id"].as_str()?.to_string(),
                    display_name: None,
                    max_tokens: None,
                    is_vision: false,
                    supports_thinking: false,
                    is_enabled: true,
                })
            }).collect())
            .unwrap_or_default();
        Ok(models)
    }

    async fn validate(&self) -> AppResult<bool> {
        let models = self.list_models().await;
        Ok(models.is_ok())
    }
}
```

**Step 2: 验证编译**

```bash
cd /home/cc/Github/orion-chat-rs/src-tauri && cargo check
```

**Step 3: Commit**

```bash
git add src-tauri/src/providers/openai_compat.rs
git commit -m "feat: implement OpenAI-compatible provider with SSE streaming"
```

### Task 9: 实现 Anthropic Provider

**Files:**
- Create: `src-tauri/src/providers/anthropic.rs`

**Step 1: 创建 anthropic.rs**

Anthropic 的 SSE 格式与 OpenAI 不同，使用 `event:` + `data:` 两行配对。

```rust
// src-tauri/src/providers/anthropic.rs
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::error::{AppError, AppResult};
use crate::models::*;
use super::Provider;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com".into()),
        }
    }

    fn build_body(&self, request: &ChatRequest) -> serde_json::Value {
        // Anthropic: system 单独提取，messages 只含 user/assistant
        let system_prompt: Option<String> = request.messages.iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone());

        let messages: Vec<serde_json::Value> = request.messages.iter()
            .filter(|m| m.role != Role::System)
            .map(|m| serde_json::json!({
                "role": m.role,
                "content": m.content,
            }))
            .collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.common.max_tokens.unwrap_or(4096),
            "stream": true,
        });
        let obj = body.as_object_mut().unwrap();

        if let Some(sys) = system_prompt {
            obj.insert("system".into(), sys.into());
        }
        if let Some(t) = request.common.temperature {
            obj.insert("temperature".into(), t.into());
        }
        if let Some(p) = request.common.top_p {
            obj.insert("top_p".into(), p.into());
        }

        if let ProviderParams::Anthropic { top_k, thinking, effort } = &request.provider_params {
            if let Some(k) = top_k { obj.insert("top_k".into(), (*k).into()); }
            if let Some(t) = thinking {
                obj.insert("thinking".into(), serde_json::to_value(t).unwrap());
                // 当 thinking 启用时，移除 temperature（Anthropic 要求）
                obj.remove("temperature");
            }
            if let Some(e) = effort {
                obj.insert("effort".into(), serde_json::to_value(e).unwrap());
            }
        }
        body
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        channel: Channel<ChatEvent>,
        cancel: watch::Receiver<bool>,
    ) -> AppResult<()> {
        let url = format!("{}/v1/messages", self.base_url);
        let body = self.build_body(&request);

        let response = self.client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!("HTTP {status}: {text}")));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut current_event = String::new();

        while let Some(chunk) = stream.next().await {
            if *cancel.borrow() { return Err(AppError::Cancelled); }
            let bytes = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.starts_with("event: ") {
                    current_event = line[7..].to_string();
                    continue;
                }
                if !line.starts_with("data: ") { continue; }

                let json_str = &line[6..];
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(json_str) {
                    match current_event.as_str() {
                        "content_block_delta" => {
                            let delta = &data["delta"];
                            match delta["type"].as_str() {
                                Some("text_delta") => {
                                    if let Some(text) = delta["text"].as_str() {
                                        let _ = channel.send(ChatEvent::Delta { content: text.to_string() });
                                    }
                                }
                                Some("thinking_delta") => {
                                    if let Some(thinking) = delta["thinking"].as_str() {
                                        let _ = channel.send(ChatEvent::Reasoning { content: thinking.to_string() });
                                    }
                                }
                                _ => {}
                            }
                        }
                        "message_delta" => {
                            if let Some(usage) = data.get("usage") {
                                if let Some(ot) = usage["output_tokens"].as_u64() {
                                    let _ = channel.send(ChatEvent::Usage {
                                        prompt_tokens: 0, // Anthropic 在 message_start 中给 input_tokens
                                        completion_tokens: ot as u32,
                                    });
                                }
                            }
                        }
                        "message_start" => {
                            if let Some(usage) = data["message"]["usage"].as_object() {
                                if let Some(it) = usage.get("input_tokens").and_then(|v| v.as_u64()) {
                                    let _ = channel.send(ChatEvent::Usage {
                                        prompt_tokens: it as u32,
                                        completion_tokens: 0,
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    async fn list_models(&self) -> AppResult<Vec<ModelInfo>> {
        // Anthropic 没有 list models API，返回硬编码列表
        Ok(vec![
            ModelInfo { id: "claude-opus-4-6-20260205".into(), provider_id: String::new(), name: "claude-opus-4-6-20260205".into(), display_name: Some("Claude Opus 4.6".into()), max_tokens: Some(128000), is_vision: true, supports_thinking: true, is_enabled: true },
            ModelInfo { id: "claude-sonnet-4-6-20260205".into(), provider_id: String::new(), name: "claude-sonnet-4-6-20260205".into(), display_name: Some("Claude Sonnet 4.6".into()), max_tokens: Some(128000), is_vision: true, supports_thinking: true, is_enabled: true },
            ModelInfo { id: "claude-haiku-4-5-20251001".into(), provider_id: String::new(), name: "claude-haiku-4-5-20251001".into(), display_name: Some("Claude Haiku 4.5".into()), max_tokens: Some(8192), is_vision: true, supports_thinking: false, is_enabled: true },
        ])
    }

    async fn validate(&self) -> AppResult<bool> {
        // 发一个最小请求验证 key
        let resp = self.client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&serde_json::json!({
                "model": "claude-haiku-4-5-20251001",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "hi"}],
            }))
            .send().await?;
        Ok(resp.status().is_success())
    }
}
```

**Step 2: 验证编译**

```bash
cd /home/cc/Github/orion-chat-rs/src-tauri && cargo check
```

**Step 3: Commit**

```bash
git add src-tauri/src/providers/anthropic.rs
git commit -m "feat: implement Anthropic Claude provider with thinking support"
```

### Task 10: 实现 Gemini Provider

**Files:**
- Create: `src-tauri/src/providers/gemini.rs`

**Step 1: 创建 gemini.rs**

Gemini 使用 REST API，SSE 格式为 `data: {json}\n\n`，但请求体结构完全不同。

```rust
// src-tauri/src/providers/gemini.rs
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::error::{AppError, AppResult};
use crate::models::*;
use super::Provider;

pub struct GeminiProvider {
    client: Client,
    api_key: String,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self { client: Client::new(), api_key }
    }

    fn build_body(&self, request: &ChatRequest) -> serde_json::Value {
        let contents: Vec<serde_json::Value> = request.messages.iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "model",
                    _ => "user",
                };
                serde_json::json!({ "role": role, "parts": [{ "text": m.content }] })
            }).collect();

        let system_instruction = request.messages.iter()
            .find(|m| m.role == Role::System)
            .map(|m| serde_json::json!({ "parts": [{ "text": m.content }] }));

        let mut body = serde_json::json!({ "contents": contents });
        let obj = body.as_object_mut().unwrap();

        if let Some(sys) = system_instruction {
            obj.insert("systemInstruction".into(), sys);
        }

        // generationConfig
        let mut gen_config = serde_json::Map::new();
        if let Some(t) = request.common.temperature { gen_config.insert("temperature".into(), t.into()); }
        if let Some(p) = request.common.top_p { gen_config.insert("topP".into(), p.into()); }
        if let Some(m) = request.common.max_tokens { gen_config.insert("maxOutputTokens".into(), m.into()); }

        if let ProviderParams::Gemini { thinking_budget, thinking_level, .. } = &request.provider_params {
            let mut thinking_config = serde_json::Map::new();
            if let Some(budget) = thinking_budget {
                thinking_config.insert("thinkingBudget".into(), (*budget).into());
            }
            if let Some(level) = thinking_level {
                thinking_config.insert("thinkingLevel".into(), serde_json::to_value(level).unwrap());
            }
            if !thinking_config.is_empty() {
                gen_config.insert("thinkingConfig".into(), thinking_config.into());
            }
        }

        if !gen_config.is_empty() {
            obj.insert("generationConfig".into(), gen_config.into());
        }
        body
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        channel: Channel<ChatEvent>,
        cancel: watch::Receiver<bool>,
    ) -> AppResult<()> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            request.model, self.api_key
        );
        let body = self.build_body(&request);

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!("HTTP {status}: {text}")));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            if *cancel.borrow() { return Err(AppError::Cancelled); }
            let bytes = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if !line.starts_with("data: ") { continue; }
                let json_str = &line[6..];

                if let Ok(data) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let Some(candidates) = data["candidates"].as_array() {
                        for candidate in candidates {
                            if let Some(parts) = candidate["content"]["parts"].as_array() {
                                for part in parts {
                                    let is_thought = part["thought"].as_bool().unwrap_or(false);
                                    if let Some(text) = part["text"].as_str() {
                                        if is_thought {
                                            let _ = channel.send(ChatEvent::Reasoning { content: text.to_string() });
                                        } else {
                                            let _ = channel.send(ChatEvent::Delta { content: text.to_string() });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(usage) = data.get("usageMetadata") {
                        let pt = usage["promptTokenCount"].as_u64().unwrap_or(0) as u32;
                        let ct = usage["candidatesTokenCount"].as_u64().unwrap_or(0) as u32;
                        if pt > 0 || ct > 0 {
                            let _ = channel.send(ChatEvent::Usage { prompt_tokens: pt, completion_tokens: ct });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn list_models(&self) -> AppResult<Vec<ModelInfo>> {
        let url = format!("https://generativelanguage.googleapis.com/v1beta/models?key={}", self.api_key);
        let resp = self.client.get(&url).send().await?;
        let body: serde_json::Value = resp.json().await?;
        let models = body["models"].as_array()
            .map(|arr| arr.iter().filter_map(|m| {
                let name = m["name"].as_str()?.strip_prefix("models/")?;
                if !name.contains("gemini") { return None; }
                Some(ModelInfo {
                    id: name.to_string(),
                    provider_id: String::new(),
                    name: name.to_string(),
                    display_name: m["displayName"].as_str().map(String::from),
                    max_tokens: m["outputTokenLimit"].as_u64().map(|v| v as u32),
                    is_vision: true,
                    supports_thinking: name.contains("2.5") || name.contains("3"),
                    is_enabled: true,
                })
            }).collect())
            .unwrap_or_default();
        Ok(models)
    }

    async fn validate(&self) -> AppResult<bool> {
        let models = self.list_models().await;
        Ok(models.is_ok() && !models.unwrap().is_empty())
    }
}
```

**Step 2: 验证编译 + Commit**

```bash
cd /home/cc/Github/orion-chat-rs/src-tauri && cargo check
git add src-tauri/src/providers/gemini.rs
git commit -m "feat: implement Google Gemini provider with thinking support"
```

### Task 11: 实现 Ollama Provider

**Files:**
- Create: `src-tauri/src/providers/ollama.rs`

**Step 1: 创建 ollama.rs**

Ollama 使用 NDJSON（每行一个 JSON），不是标准 SSE。

```rust
// src-tauri/src/providers/ollama.rs
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::error::{AppError, AppResult};
use crate::models::*;
use super::Provider;

pub struct OllamaProvider {
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".into()),
        }
    }

    fn build_body(&self, request: &ChatRequest) -> serde_json::Value {
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| {
            serde_json::json!({ "role": m.role, "content": m.content })
        }).collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });
        let obj = body.as_object_mut().unwrap();

        let mut options = serde_json::Map::new();
        if let Some(t) = request.common.temperature { options.insert("temperature".into(), t.into()); }
        if let Some(p) = request.common.top_p { options.insert("top_p".into(), p.into()); }
        if let Some(m) = request.common.max_tokens { options.insert("num_predict".into(), m.into()); }

        if let ProviderParams::Ollama { think, num_ctx, repeat_penalty, min_p, keep_alive } = &request.provider_params {
            if let Some(t) = think { obj.insert("think".into(), serde_json::to_value(t).unwrap()); }
            if let Some(n) = num_ctx { options.insert("num_ctx".into(), (*n).into()); }
            if let Some(r) = repeat_penalty { options.insert("repeat_penalty".into(), (*r).into()); }
            if let Some(mp) = min_p { options.insert("min_p".into(), (*mp).into()); }
            if let Some(ka) = keep_alive { obj.insert("keep_alive".into(), ka.clone().into()); }
        }

        if !options.is_empty() {
            obj.insert("options".into(), options.into());
        }
        body
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        channel: Channel<ChatEvent>,
        cancel: watch::Receiver<bool>,
    ) -> AppResult<()> {
        let url = format!("{}/api/chat", self.base_url);
        let body = self.build_body(&request);

        let response = self.client
            .post(&url)
            .json(&body)
            .send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!("HTTP {status}: {text}")));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            if *cancel.borrow() { return Err(AppError::Cancelled); }
            let bytes = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            // NDJSON: 每行一个 JSON
            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();
                if line.is_empty() { continue; }

                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(msg) = data.get("message") {
                        if let Some(content) = msg["content"].as_str() {
                            if !content.is_empty() {
                                let _ = channel.send(ChatEvent::Delta { content: content.to_string() });
                            }
                        }
                        if let Some(thinking) = msg["thinking"].as_str() {
                            if !thinking.is_empty() {
                                let _ = channel.send(ChatEvent::Reasoning { content: thinking.to_string() });
                            }
                        }
                    }
                    if data["done"].as_bool() == Some(true) {
                        let pt = data["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
                        let ct = data["eval_count"].as_u64().unwrap_or(0) as u32;
                        let _ = channel.send(ChatEvent::Usage { prompt_tokens: pt, completion_tokens: ct });
                    }
                }
            }
        }
        Ok(())
    }

    async fn list_models(&self) -> AppResult<Vec<ModelInfo>> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self.client.get(&url).send().await?;
        let body: serde_json::Value = resp.json().await?;
        let models = body["models"].as_array()
            .map(|arr| arr.iter().filter_map(|m| {
                Some(ModelInfo {
                    id: m["name"].as_str()?.to_string(),
                    provider_id: String::new(),
                    name: m["name"].as_str()?.to_string(),
                    display_name: None,
                    max_tokens: None,
                    is_vision: false,
                    supports_thinking: false,
                    is_enabled: true,
                })
            }).collect())
            .unwrap_or_default();
        Ok(models)
    }

    async fn validate(&self) -> AppResult<bool> {
        let resp = self.client.get(format!("{}/api/version", self.base_url)).send().await;
        Ok(resp.is_ok())
    }
}
```

**Step 2: 验证编译 + Commit**

```bash
cd /home/cc/Github/orion-chat-rs/src-tauri && cargo check
git add src-tauri/src/providers/ollama.rs
git commit -m "feat: implement Ollama provider with NDJSON streaming"
```

---

## Phase 5: Tauri Commands — 连接前后端

### Task 12: 实现 AppState 和 Provider 路由

**Files:**
- Create: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/main.rs`

**Step 1: 创建 state.rs — 全局应用状态**

```rust
// src-tauri/src/state.rs
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

use crate::db::Database;
use crate::models::ProviderType;
use crate::providers::{Provider, openai_compat::OpenAICompatProvider, anthropic::AnthropicProvider, gemini::GeminiProvider, ollama::OllamaProvider};

pub struct AppState {
    pub db: Database,
    pub providers: Mutex<HashMap<String, Arc<dyn Provider>>>,
    pub cancel_sender: watch::Sender<bool>,
    pub cancel_receiver: watch::Receiver<bool>,
}

impl AppState {
    pub fn new(db_path: &str) -> Self {
        let (tx, rx) = watch::channel(false);
        Self {
            db: Database::new(db_path).expect("Failed to init database"),
            providers: Mutex::new(HashMap::new()),
            cancel_sender: tx,
            cancel_receiver: rx,
        }
    }

    pub fn register_provider(&self, id: &str, provider_type: &ProviderType, api_key: &str, base_url: &str) {
        let provider: Arc<dyn Provider> = match provider_type {
            ProviderType::OpenaiCompat => Arc::new(OpenAICompatProvider::new(api_key.into(), base_url.into())),
            ProviderType::Anthropic => Arc::new(AnthropicProvider::new(api_key.into(), Some(base_url.into()))),
            ProviderType::Gemini => Arc::new(GeminiProvider::new(api_key.into())),
            ProviderType::Ollama => Arc::new(OllamaProvider::new(Some(base_url.into()))),
        };
        self.providers.lock().unwrap().insert(id.to_string(), provider);
    }

    pub fn get_provider(&self, id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.lock().unwrap().get(id).cloned()
    }
}
```

**Step 2: 更新 main.rs — 注册 state**

```rust
// src-tauri/src/main.rs
use orion_chat_rs::state::AppState;

fn main() {
    let app_data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("orion-chat");
    std::fs::create_dir_all(&app_data_dir).ok();
    let db_path = app_data_dir.join("chat.db").to_string_lossy().to_string();

    tauri::Builder::default()
        .manage(AppState::new(&db_path))
        .plugin(tauri_plugin_store::Builder::new().build())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

注意: 需要在 Cargo.toml 添加 `dirs = "6"` 依赖。

**Step 3: 更新 lib.rs**

```rust
pub mod error;
pub mod models;
pub mod db;
pub mod providers;
pub mod state;
pub mod commands;
```

**Step 4: 验证编译 + Commit**

```bash
cd /home/cc/Github/orion-chat-rs/src-tauri && cargo check
git add src-tauri/src/state.rs src-tauri/src/main.rs src-tauri/src/lib.rs
git commit -m "feat: add AppState with provider registry and cancel mechanism"
```

### Task 13: 实现聊天 Tauri Commands

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/chat.rs`

**Step 1: 创建 commands/mod.rs**

```rust
// src-tauri/src/commands/mod.rs
pub mod chat;
pub mod conversation;
pub mod provider;
pub mod assistant;
pub mod search;
pub mod export;
```

**Step 2: 创建 commands/chat.rs — 核心聊天命令**

```rust
// src-tauri/src/commands/chat.rs
use tauri::{ipc::Channel, State};
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::*;
use crate::state::AppState;

#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    conversation_id: String,
    content: String,
    model_id: String,
    channel: Channel<ChatEvent>,
) -> AppResult<Message> {
    // 1. 查找 model 对应的 provider
    let provider_id = state.db.with_conn(|conn| {
        let pid: String = conn.query_row(
            "SELECT provider_id FROM models WHERE id = ?1", [&model_id], |r| r.get(0)
        )?;
        Ok(pid)
    })?;

    let provider = state.get_provider(&provider_id)
        .ok_or_else(|| crate::error::AppError::NotFound("Provider not configured".into()))?;

    // 2. 保存 user message
    let user_msg = Message {
        id: Uuid::new_v4().to_string(),
        conversation_id: conversation_id.clone(),
        role: Role::User,
        content: content.clone(),
        model_id: None,
        reasoning: None,
        token_prompt: None,
        token_completion: None,
        status: MessageStatus::Done,
        created_at: String::new(),
    };
    state.db.with_conn(|conn| crate::db::messages::create(conn, &user_msg))?;

    // 3. 创建 assistant message 占位
    let asst_msg_id = Uuid::new_v4().to_string();
    let asst_msg = Message {
        id: asst_msg_id.clone(),
        conversation_id: conversation_id.clone(),
        role: Role::Assistant,
        content: String::new(),
        model_id: Some(model_id.clone()),
        reasoning: None,
        token_prompt: None,
        token_completion: None,
        status: MessageStatus::Streaming,
        created_at: String::new(),
    };
    state.db.with_conn(|conn| crate::db::messages::create(conn, &asst_msg))?;

    // 4. 加载对话历史
    let messages = state.db.with_conn(|conn| {
        crate::db::messages::list_by_conversation(conn, &conversation_id)
    })?;
    let chat_messages: Vec<ChatMessage> = messages.iter()
        .filter(|m| m.status != MessageStatus::Error)
        .map(|m| ChatMessage { role: m.role.clone(), content: m.content.clone() })
        .collect();

    // 5. 构建请求
    let model_name = state.db.with_conn(|conn| {
        let name: String = conn.query_row(
            "SELECT name FROM models WHERE id = ?1", [&model_id], |r| r.get(0)
        )?;
        Ok(name)
    })?;

    let request = ChatRequest {
        model: model_name,
        messages: chat_messages,
        common: CommonParams { temperature: None, top_p: None, max_tokens: None, stream: true },
        provider_params: ProviderParams::OpenaiCompat {
            frequency_penalty: None, presence_penalty: None,
            reasoning_effort: None, seed: None, max_completion_tokens: None,
        },
    };

    // 6. 发送 Started 事件
    let _ = channel.send(ChatEvent::Started { message_id: asst_msg_id.clone() });

    // 7. 重置 cancel 信号
    let _ = state.cancel_sender.send(false);
    let cancel = state.cancel_receiver.clone();

    // 8. 流式请求
    let mut full_content = String::new();
    let mut full_reasoning = String::new();
    let mut prompt_tokens = 0u32;
    let mut completion_tokens = 0u32;

    // 用一个内部 channel 收集内容
    let (inner_tx, mut inner_rx) = tokio::sync::mpsc::unbounded_channel::<ChatEvent>();
    let channel_clone = channel.clone();

    // 转发事件到前端并收集内容
    let collector = tokio::spawn(async move {
        let mut content = String::new();
        let mut reasoning = String::new();
        let mut pt = 0u32;
        let mut ct = 0u32;
        while let Some(event) = inner_rx.recv().await {
            match &event {
                ChatEvent::Delta { content: c } => content.push_str(c),
                ChatEvent::Reasoning { content: r } => reasoning.push_str(r),
                ChatEvent::Usage { prompt_tokens: p, completion_tokens: c } => { pt = *p; ct = *c; }
                _ => {}
            }
            let _ = channel_clone.send(event);
        }
        (content, reasoning, pt, ct)
    });

    // 创建一个 Channel 适配器给 provider
    // 注意: 实际实现中直接传 channel 给 provider，这里简化处理
    let result = provider.stream_chat(request, channel.clone(), cancel).await;

    match result {
        Ok(()) => {
            let _ = channel.send(ChatEvent::Finished { message_id: asst_msg_id.clone() });
        }
        Err(crate::error::AppError::Cancelled) => {
            let _ = channel.send(ChatEvent::Finished { message_id: asst_msg_id.clone() });
        }
        Err(e) => {
            let _ = channel.send(ChatEvent::Error { message: e.to_string() });
            state.db.with_conn(|conn| {
                crate::db::messages::set_error(conn, &asst_msg_id, &e.to_string())
            })?;
        }
    }

    // 9. 更新 conversation
    state.db.with_conn(|conn| crate::db::conversations::touch(conn, &conversation_id))?;

    Ok(asst_msg)
}

#[tauri::command]
pub async fn stop_generation(state: State<'_, AppState>) -> AppResult<()> {
    let _ = state.cancel_sender.send(true);
    Ok(())
}
```

**Step 3: 在 main.rs 注册命令**

```rust
tauri::Builder::default()
    .manage(AppState::new(&db_path))
    .plugin(tauri_plugin_store::Builder::new().build())
    .invoke_handler(tauri::generate_handler![
        crate::commands::chat::send_message,
        crate::commands::chat::stop_generation,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

**Step 4: 验证编译 + Commit**

```bash
cd /home/cc/Github/orion-chat-rs/src-tauri && cargo check
git add src-tauri/src/commands/ src-tauri/src/main.rs
git commit -m "feat: implement send_message and stop_generation Tauri commands"
```

### Task 14: 实现对话、助手、供应商、搜索、导出 Commands

**Files:**
- Create: `src-tauri/src/commands/conversation.rs`
- Create: `src-tauri/src/commands/assistant.rs`
- Create: `src-tauri/src/commands/provider.rs`
- Create: `src-tauri/src/commands/search.rs`
- Create: `src-tauri/src/commands/export.rs`

每个文件都是对 db 层的薄封装，模式一致：

**Step 1: commands/conversation.rs**

```rust
use tauri::State;
use uuid::Uuid;
use crate::error::AppResult;
use crate::models::Conversation;
use crate::state::AppState;

#[tauri::command]
pub async fn create_conversation(state: State<'_, AppState>, title: String, model_id: Option<String>) -> AppResult<Conversation> {
    let conv = Conversation {
        id: Uuid::new_v4().to_string(), title, assistant_id: None, model_id,
        is_pinned: false, sort_order: 0, created_at: String::new(), updated_at: String::new(),
    };
    state.db.with_conn(|conn| crate::db::conversations::create(conn, &conv))?;
    state.db.with_conn(|conn| crate::db::conversations::get(conn, &conv.id))
}

#[tauri::command]
pub async fn list_conversations(state: State<'_, AppState>) -> AppResult<Vec<Conversation>> {
    state.db.with_conn(|conn| crate::db::conversations::list(conn))
}

#[tauri::command]
pub async fn update_conversation_title(state: State<'_, AppState>, id: String, title: String) -> AppResult<()> {
    state.db.with_conn(|conn| crate::db::conversations::update_title(conn, &id, &title))
}

#[tauri::command]
pub async fn delete_conversation(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.db.with_conn(|conn| crate::db::conversations::delete(conn, &id))
}

#[tauri::command]
pub async fn get_messages(state: State<'_, AppState>, conversation_id: String) -> AppResult<Vec<crate::models::Message>> {
    state.db.with_conn(|conn| crate::db::messages::list_by_conversation(conn, &conversation_id))
}
```

**Step 2: commands/provider.rs**

```rust
use tauri::State;
use uuid::Uuid;
use crate::error::AppResult;
use crate::models::{ProviderConfig, ProviderType, ModelInfo};
use crate::state::AppState;

#[tauri::command]
pub async fn add_provider(state: State<'_, AppState>, name: String, provider_type: ProviderType, api_key: Option<String>, base_url: String) -> AppResult<ProviderConfig> {
    let config = ProviderConfig {
        id: Uuid::new_v4().to_string(), name, r#type: provider_type.clone(),
        api_key: api_key.clone(), base_url: base_url.clone(), proxy: None, is_enabled: true,
    };
    state.db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO providers (id, name, type, api_key, base_url) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![config.id, config.name, serde_json::to_string(&config.r#type).unwrap().trim_matches('"'), config.api_key, config.base_url],
        )?;
        Ok(())
    })?;
    state.register_provider(&config.id, &provider_type, api_key.as_deref().unwrap_or(""), &base_url);
    Ok(config)
}

#[tauri::command]
pub async fn list_providers(state: State<'_, AppState>) -> AppResult<Vec<ProviderConfig>> {
    state.db.with_conn(|conn| {
        let mut stmt = conn.prepare("SELECT id, name, type, api_key, base_url, proxy, is_enabled FROM providers")?;
        let rows = stmt.query_map([], |row| {
            let type_str: String = row.get(2)?;
            Ok(ProviderConfig {
                id: row.get(0)?, name: row.get(1)?,
                r#type: serde_json::from_str(&format!("\"{type_str}\"")).unwrap_or(ProviderType::OpenaiCompat),
                api_key: row.get(3)?, base_url: row.get(4)?, proxy: row.get(5)?, is_enabled: row.get(6)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    })
}

#[tauri::command]
pub async fn fetch_models(state: State<'_, AppState>, provider_id: String) -> AppResult<Vec<ModelInfo>> {
    let provider = state.get_provider(&provider_id)
        .ok_or_else(|| crate::error::AppError::NotFound("Provider not found".into()))?;
    let mut models = provider.list_models().await?;
    for m in &mut models { m.provider_id = provider_id.clone(); }
    // 保存到 DB
    state.db.with_conn(|conn| {
        for m in &models {
            conn.execute(
                "INSERT OR REPLACE INTO models (id, provider_id, name, display_name, max_tokens, is_vision, supports_thinking)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![m.id, m.provider_id, m.name, m.display_name, m.max_tokens, m.is_vision, m.supports_thinking],
            )?;
        }
        Ok(())
    })?;
    Ok(models)
}
```

**Step 3: commands/search.rs**

```rust
use tauri::State;
use crate::error::AppResult;
use crate::models::Message;
use crate::state::AppState;

#[tauri::command]
pub async fn search_messages(state: State<'_, AppState>, query: String) -> AppResult<Vec<Message>> {
    state.db.with_conn(|conn| crate::db::messages::search(conn, &query))
}
```

**Step 4: commands/export.rs**

```rust
use tauri::State;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub async fn export_conversation_markdown(state: State<'_, AppState>, conversation_id: String) -> AppResult<String> {
    let conv = state.db.with_conn(|conn| crate::db::conversations::get(conn, &conversation_id))?;
    let messages = state.db.with_conn(|conn| crate::db::messages::list_by_conversation(conn, &conversation_id))?;

    let mut md = format!("# {}\n\n", conv.title);
    for msg in &messages {
        let role_label = match msg.role {
            crate::models::Role::User => "**User**",
            crate::models::Role::Assistant => "**Assistant**",
            crate::models::Role::System => "**System**",
        };
        md.push_str(&format!("### {}\n\n{}\n\n---\n\n", role_label, msg.content));
    }
    Ok(md)
}

#[tauri::command]
pub async fn export_conversation_json(state: State<'_, AppState>, conversation_id: String) -> AppResult<String> {
    let conv = state.db.with_conn(|conn| crate::db::conversations::get(conn, &conversation_id))?;
    let messages = state.db.with_conn(|conn| crate::db::messages::list_by_conversation(conn, &conversation_id))?;
    let export = serde_json::json!({ "conversation": conv, "messages": messages });
    Ok(serde_json::to_string_pretty(&export)?)
}
```

**Step 5: commands/assistant.rs**

```rust
use tauri::State;
use uuid::Uuid;
use crate::error::AppResult;
use crate::models::Assistant;
use crate::state::AppState;

#[tauri::command]
pub async fn create_assistant(state: State<'_, AppState>, name: String, system_prompt: Option<String>, model_id: Option<String>) -> AppResult<Assistant> {
    let asst = Assistant {
        id: Uuid::new_v4().to_string(), name, icon: None, system_prompt, model_id,
        temperature: None, top_p: None, max_tokens: None,
        extra_params: serde_json::json!({}), sort_order: 0, created_at: String::new(),
    };
    state.db.with_conn(|conn| crate::db::assistants::create(conn, &asst))?;
    state.db.with_conn(|conn| crate::db::assistants::get(conn, &asst.id))
}

#[tauri::command]
pub async fn list_assistants(state: State<'_, AppState>) -> AppResult<Vec<Assistant>> {
    state.db.with_conn(|conn| crate::db::assistants::list(conn))
}

#[tauri::command]
pub async fn update_assistant(state: State<'_, AppState>, assistant: Assistant) -> AppResult<()> {
    state.db.with_conn(|conn| crate::db::assistants::update(conn, &assistant))
}

#[tauri::command]
pub async fn delete_assistant(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.db.with_conn(|conn| crate::db::assistants::delete(conn, &id))
}
```

**Step 6: 在 main.rs 注册所有命令**

```rust
.invoke_handler(tauri::generate_handler![
    commands::chat::send_message,
    commands::chat::stop_generation,
    commands::conversation::create_conversation,
    commands::conversation::list_conversations,
    commands::conversation::update_conversation_title,
    commands::conversation::delete_conversation,
    commands::conversation::get_messages,
    commands::provider::add_provider,
    commands::provider::list_providers,
    commands::provider::fetch_models,
    commands::assistant::create_assistant,
    commands::assistant::list_assistants,
    commands::assistant::update_assistant,
    commands::assistant::delete_assistant,
    commands::search::search_messages,
    commands::export::export_conversation_markdown,
    commands::export::export_conversation_json,
])
```

**Step 7: 验证编译 + Commit**

```bash
cd /home/cc/Github/orion-chat-rs/src-tauri && cargo check
git add src-tauri/src/commands/ src-tauri/src/main.rs
git commit -m "feat: implement all Tauri commands for conversations, providers, assistants, search, export"
```

---

## Phase 6: Svelte 前端 — 基础布局 + 聊天 UI

### Task 15: 全局样式 + 主题系统 + 布局

**Files:**
- Create: `src/app.css`
- Modify: `src/routes/+layout.svelte`
- Create: `src/lib/stores/ui.svelte.ts`

**Step 1: 创建 app.css — TailwindCSS + CSS 变量主题**

```css
/* src/app.css */
@import 'tailwindcss';

:root {
  --bg-primary: #ffffff;
  --bg-secondary: #f8f9fa;
  --bg-sidebar: rgba(248, 249, 250, 0.8);
  --text-primary: #1a1a2e;
  --text-secondary: #6b7280;
  --border: #e5e7eb;
  --accent: #6366f1;
  --accent-hover: #4f46e5;
  --msg-user: #f0f0ff;
  --msg-assistant: #ffffff;
  --radius: 12px;
}

[data-theme="dark"] {
  --bg-primary: #0f0f1a;
  --bg-secondary: #1a1a2e;
  --bg-sidebar: rgba(26, 26, 46, 0.8);
  --text-primary: #e2e8f0;
  --text-secondary: #94a3b8;
  --border: #2d2d44;
  --accent: #818cf8;
  --accent-hover: #6366f1;
  --msg-user: #1e1e3a;
  --msg-assistant: #1a1a2e;
}

body {
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
  margin: 0;
  overflow: hidden;
}
```

**Step 2: 创建 ui store**

```typescript
// src/lib/stores/ui.svelte.ts
let theme = $state<'light' | 'dark'>('dark');
let sidebarOpen = $state(true);

export function getTheme() { return theme; }
export function toggleTheme() {
  theme = theme === 'dark' ? 'light' : 'dark';
  document.documentElement.setAttribute('data-theme', theme);
}
export function getSidebarOpen() { return sidebarOpen; }
export function toggleSidebar() { sidebarOpen = !sidebarOpen; }
```

**Step 3: 创建主布局**

```svelte
<!-- src/routes/+layout.svelte -->
<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';

  let { children } = $props();

  onMount(() => {
    document.documentElement.setAttribute('data-theme', 'dark');
  });
</script>

<div class="flex h-screen w-screen overflow-hidden">
  {@render children()}
</div>
```

**Step 4: 验证前端编译**

```bash
pnpm dev
```

Expected: 前端编译通过，空白页面显示。

**Step 5: Commit**

```bash
git add src/app.css src/lib/stores/ src/routes/+layout.svelte
git commit -m "feat: add theme system, UI store, and main layout"
```

### Task 16: 侧边栏 — 对话列表

**Files:**
- Create: `src/lib/components/sidebar/ConversationList.svelte`
- Create: `src/lib/utils/invoke.ts`
- Create: `src/lib/types/index.ts`
- Modify: `src/routes/+page.svelte`

**Step 1: 创建 TypeScript 类型（与 Rust 对应）**

```typescript
// src/lib/types/index.ts
export interface Conversation {
  id: string;
  title: string;
  assistantId: string | null;
  modelId: string | null;
  isPinned: boolean;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface Message {
  id: string;
  conversationId: string;
  role: 'system' | 'user' | 'assistant';
  content: string;
  modelId: string | null;
  reasoning: string | null;
  tokenPrompt: number | null;
  tokenCompletion: number | null;
  status: 'streaming' | 'done' | 'error';
  createdAt: string;
}

export interface ChatEvent {
  event: 'started' | 'delta' | 'reasoning' | 'usage' | 'finished' | 'error';
  data: any;
}

export interface ProviderConfig {
  id: string;
  name: string;
  type: 'openaiCompat' | 'anthropic' | 'gemini' | 'ollama';
  apiKey: string | null;
  baseUrl: string;
  proxy: string | null;
  isEnabled: boolean;
}

export interface ModelInfo {
  id: string;
  providerId: string;
  name: string;
  displayName: string | null;
  maxTokens: number | null;
  isVision: boolean;
  supportsThinking: boolean;
  isEnabled: boolean;
}

export interface Assistant {
  id: string;
  name: string;
  icon: string | null;
  systemPrompt: string | null;
  modelId: string | null;
  temperature: number | null;
  topP: number | null;
  maxTokens: number | null;
  extraParams: Record<string, any>;
  sortOrder: number;
  createdAt: string;
}
```

**Step 2: 创建 invoke 封装**

```typescript
// src/lib/utils/invoke.ts
import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import type { Conversation, Message, ProviderConfig, ModelInfo, Assistant } from '$lib/types';

export const api = {
  // Conversations
  createConversation: (title: string, modelId?: string) =>
    tauriInvoke<Conversation>('create_conversation', { title, modelId }),
  listConversations: () =>
    tauriInvoke<Conversation[]>('list_conversations'),
  updateConversationTitle: (id: string, title: string) =>
    tauriInvoke<void>('update_conversation_title', { id, title }),
  deleteConversation: (id: string) =>
    tauriInvoke<void>('delete_conversation', { id }),
  getMessages: (conversationId: string) =>
    tauriInvoke<Message[]>('get_messages', { conversationId }),

  // Chat
  sendMessage: (conversationId: string, content: string, modelId: string, onEvent: (e: any) => void) =>
    tauriInvoke<Message>('send_message', { conversationId, content, modelId, channel: { onEvent } }),
  stopGeneration: () =>
    tauriInvoke<void>('stop_generation'),

  // Providers
  addProvider: (name: string, providerType: string, apiKey: string | null, baseUrl: string) =>
    tauriInvoke<ProviderConfig>('add_provider', { name, providerType, apiKey, baseUrl }),
  listProviders: () =>
    tauriInvoke<ProviderConfig[]>('list_providers'),
  fetchModels: (providerId: string) =>
    tauriInvoke<ModelInfo[]>('fetch_models', { providerId }),

  // Assistants
  createAssistant: (name: string, systemPrompt?: string, modelId?: string) =>
    tauriInvoke<Assistant>('create_assistant', { name, systemPrompt, modelId }),
  listAssistants: () =>
    tauriInvoke<Assistant[]>('list_assistants'),
  updateAssistant: (assistant: Assistant) =>
    tauriInvoke<void>('update_assistant', { assistant }),
  deleteAssistant: (id: string) =>
    tauriInvoke<void>('delete_assistant', { id }),

  // Search
  searchMessages: (query: string) =>
    tauriInvoke<Message[]>('search_messages', { query }),

  // Export
  exportMarkdown: (conversationId: string) =>
    tauriInvoke<string>('export_conversation_markdown', { conversationId }),
  exportJson: (conversationId: string) =>
    tauriInvoke<string>('export_conversation_json', { conversationId }),
};
```

**Step 3: 创建 ConversationList 组件**

```svelte
<!-- src/lib/components/sidebar/ConversationList.svelte -->
<script lang="ts">
  import type { Conversation } from '$lib/types';
  import { api } from '$lib/utils/invoke';

  let { activeId = $bindable(''), onSelect }: {
    activeId: string;
    onSelect: (id: string) => void;
  } = $props();

  let conversations = $state<Conversation[]>([]);

  async function load() {
    conversations = await api.listConversations();
  }

  async function create() {
    const conv = await api.createConversation('New Chat');
    conversations = [conv, ...conversations];
    onSelect(conv.id);
  }

  async function remove(id: string) {
    await api.deleteConversation(id);
    conversations = conversations.filter(c => c.id !== id);
    if (activeId === id && conversations.length > 0) {
      onSelect(conversations[0].id);
    }
  }

  $effect(() => { load(); });
</script>

<div class="flex flex-col h-full">
  <button onclick={create}
    class="mx-3 my-2 px-4 py-2 rounded-lg bg-[var(--accent)] text-white text-sm hover:bg-[var(--accent-hover)] transition-colors">
    + New Chat
  </button>

  <div class="flex-1 overflow-y-auto px-2">
    {#each conversations as conv (conv.id)}
      <button
        onclick={() => onSelect(conv.id)}
        class="w-full text-left px-3 py-2.5 rounded-lg mb-0.5 text-sm truncate transition-colors
          {activeId === conv.id ? 'bg-[var(--accent)]/10 text-[var(--accent)]' : 'hover:bg-[var(--bg-secondary)] text-[var(--text-secondary)]'}"
      >
        {conv.title}
      </button>
    {/each}
  </div>
</div>
```

**Step 4: 创建主页面**

```svelte
<!-- src/routes/+page.svelte -->
<script lang="ts">
  import ConversationList from '$lib/components/sidebar/ConversationList.svelte';

  let activeConversationId = $state('');

  function handleSelect(id: string) {
    activeConversationId = id;
  }
</script>

<aside class="w-64 h-full border-r border-[var(--border)] bg-[var(--bg-sidebar)] backdrop-blur-xl flex flex-col">
  <div class="h-12 flex items-center px-4 font-semibold text-sm text-[var(--text-primary)]">
    Orion Chat
  </div>
  <ConversationList bind:activeId={activeConversationId} onSelect={handleSelect} />
</aside>

<main class="flex-1 flex flex-col bg-[var(--bg-primary)]">
  <div class="flex-1 flex items-center justify-center text-[var(--text-secondary)]">
    {#if !activeConversationId}
      Select or create a conversation
    {:else}
      <!-- Chat area will go here -->
      Chat: {activeConversationId}
    {/if}
  </div>
</main>
```

**Step 5: Commit**

```bash
git add src/lib/ src/routes/
git commit -m "feat: add sidebar with conversation list, types, and invoke wrapper"
```

### Task 17: 聊天区域 — 消息列表 + 输入框

**Files:**
- Create: `src/lib/components/chat/MessageList.svelte`
- Create: `src/lib/components/chat/MessageBubble.svelte`
- Create: `src/lib/components/chat/InputArea.svelte`
- Create: `src/lib/utils/markdown.ts`
- Modify: `src/routes/+page.svelte`

**Step 1: 创建 markdown 渲染工具**

```typescript
// src/lib/utils/markdown.ts
import { marked } from 'marked';
import hljs from 'highlight.js';
import 'highlight.js/styles/github-dark.css';

marked.setOptions({
  highlight(code: string, lang: string) {
    if (lang && hljs.getLanguage(lang)) {
      return hljs.highlight(code, { language: lang }).value;
    }
    return hljs.highlightAuto(code).value;
  },
});

export function renderMarkdown(content: string): string {
  return marked.parse(content) as string;
}
```

**Step 2: 创建 MessageBubble**

```svelte
<!-- src/lib/components/chat/MessageBubble.svelte -->
<script lang="ts">
  import type { Message } from '$lib/types';
  import { renderMarkdown } from '$lib/utils/markdown';

  let { message }: { message: Message } = $props();

  let renderedContent = $derived(renderMarkdown(message.content));
  let isUser = $derived(message.role === 'user');
</script>

<div class="flex {isUser ? 'justify-end' : 'justify-start'} mb-4 px-4">
  <div class="max-w-[80%] rounded-2xl px-4 py-3 {isUser ? 'bg-[var(--msg-user)]' : 'bg-[var(--msg-assistant)] border border-[var(--border)]'}">
    {#if message.reasoning}
      <details class="mb-2 text-xs text-[var(--text-secondary)]">
        <summary class="cursor-pointer">Thinking...</summary>
        <pre class="mt-1 whitespace-pre-wrap">{message.reasoning}</pre>
      </details>
    {/if}
    <div class="prose prose-sm dark:prose-invert max-w-none">
      {@html renderedContent}
    </div>
    {#if message.tokenPrompt || message.tokenCompletion}
      <div class="mt-2 text-xs text-[var(--text-secondary)]">
        {message.tokenPrompt ?? 0} in / {message.tokenCompletion ?? 0} out
      </div>
    {/if}
  </div>
</div>
```

**Step 3: 创建 MessageList**

```svelte
<!-- src/lib/components/chat/MessageList.svelte -->
<script lang="ts">
  import type { Message } from '$lib/types';
  import MessageBubble from './MessageBubble.svelte';

  let { messages }: { messages: Message[] } = $props();
  let container: HTMLDivElement;

  $effect(() => {
    if (messages.length && container) {
      container.scrollTop = container.scrollHeight;
    }
  });
</script>

<div bind:this={container} class="flex-1 overflow-y-auto py-4">
  {#each messages as msg (msg.id)}
    <MessageBubble message={msg} />
  {/each}
</div>
```

**Step 4: 创建 InputArea**

```svelte
<!-- src/lib/components/chat/InputArea.svelte -->
<script lang="ts">
  let { onSend, disabled = false }: {
    onSend: (content: string) => void;
    disabled: boolean;
  } = $props();

  let input = $state('');

  function handleSubmit() {
    const content = input.trim();
    if (!content || disabled) return;
    onSend(content);
    input = '';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }
</script>

<div class="border-t border-[var(--border)] p-4">
  <div class="flex items-end gap-2 max-w-3xl mx-auto">
    <textarea
      bind:value={input}
      onkeydown={handleKeydown}
      placeholder="Type a message..."
      rows="1"
      class="flex-1 resize-none rounded-xl border border-[var(--border)] bg-[var(--bg-secondary)] px-4 py-3 text-sm
        text-[var(--text-primary)] placeholder:text-[var(--text-secondary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50"
    ></textarea>
    <button
      onclick={handleSubmit}
      disabled={disabled || !input.trim()}
      class="px-4 py-3 rounded-xl bg-[var(--accent)] text-white text-sm font-medium
        hover:bg-[var(--accent-hover)] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
    >
      Send
    </button>
  </div>
</div>
```

**Step 5: 更新 +page.svelte 集成聊天区域**

```svelte
<!-- src/routes/+page.svelte -->
<script lang="ts">
  import ConversationList from '$lib/components/sidebar/ConversationList.svelte';
  import MessageList from '$lib/components/chat/MessageList.svelte';
  import InputArea from '$lib/components/chat/InputArea.svelte';
  import { api } from '$lib/utils/invoke';
  import type { Message } from '$lib/types';

  let activeConversationId = $state('');
  let messages = $state<Message[]>([]);
  let isStreaming = $state(false);

  async function handleSelect(id: string) {
    activeConversationId = id;
    messages = await api.getMessages(id);
  }

  async function handleSend(content: string) {
    if (!activeConversationId) return;
    isStreaming = true;

    // Optimistic: add user message to UI
    const userMsg: Message = {
      id: crypto.randomUUID(),
      conversationId: activeConversationId,
      role: 'user',
      content,
      modelId: null, reasoning: null,
      tokenPrompt: null, tokenCompletion: null,
      status: 'done',
      createdAt: new Date().toISOString(),
    };
    messages = [...messages, userMsg];

    // Streaming assistant message
    let assistantMsg: Message = {
      id: '',
      conversationId: activeConversationId,
      role: 'assistant',
      content: '',
      modelId: null, reasoning: null,
      tokenPrompt: null, tokenCompletion: null,
      status: 'streaming',
      createdAt: new Date().toISOString(),
    };
    messages = [...messages, assistantMsg];

    try {
      await api.sendMessage(activeConversationId, content, 'default-model', (event: any) => {
        switch (event.event) {
          case 'started':
            assistantMsg.id = event.data.messageId;
            break;
          case 'delta':
            assistantMsg.content += event.data.content;
            messages = [...messages.slice(0, -1), { ...assistantMsg }];
            break;
          case 'reasoning':
            assistantMsg.reasoning = (assistantMsg.reasoning ?? '') + event.data.content;
            messages = [...messages.slice(0, -1), { ...assistantMsg }];
            break;
          case 'usage':
            assistantMsg.tokenPrompt = event.data.promptTokens;
            assistantMsg.tokenCompletion = event.data.completionTokens;
            break;
          case 'finished':
            assistantMsg.status = 'done';
            messages = [...messages.slice(0, -1), { ...assistantMsg }];
            break;
          case 'error':
            assistantMsg.status = 'error';
            assistantMsg.content = event.data.message;
            messages = [...messages.slice(0, -1), { ...assistantMsg }];
            break;
        }
      });
    } finally {
      isStreaming = false;
    }
  }
</script>

<aside class="w-64 h-full border-r border-[var(--border)] bg-[var(--bg-sidebar)] backdrop-blur-xl flex flex-col">
  <div class="h-12 flex items-center px-4 font-semibold text-sm">Orion Chat</div>
  <ConversationList bind:activeId={activeConversationId} onSelect={handleSelect} />
</aside>

<main class="flex-1 flex flex-col">
  {#if activeConversationId}
    <MessageList {messages} />
    <InputArea onSend={handleSend} disabled={isStreaming} />
  {:else}
    <div class="flex-1 flex items-center justify-center text-[var(--text-secondary)]">
      Select or create a conversation
    </div>
  {/if}
</main>
```

**Step 6: Commit**

```bash
git add src/lib/components/chat/ src/lib/utils/markdown.ts src/routes/+page.svelte
git commit -m "feat: add chat UI with message list, markdown rendering, and streaming input"
```

---

## Phase 7: 设置页面 + 多模型对比 + 收尾

### Task 18: 供应商设置页面

**Files:**
- Create: `src/lib/components/settings/ProviderSettings.svelte`
- Create: `src/lib/components/settings/ModelSettings.svelte`

**Step 1: 创建 ProviderSettings.svelte**

供应商配置面板：添加/编辑供应商，输入 API Key 和 Base URL，拉取模型列表。

```svelte
<!-- src/lib/components/settings/ProviderSettings.svelte -->
<script lang="ts">
  import { api } from '$lib/utils/invoke';
  import type { ProviderConfig, ModelInfo } from '$lib/types';

  let providers = $state<ProviderConfig[]>([]);
  let models = $state<Record<string, ModelInfo[]>>({});
  let newProvider = $state({ name: '', type: 'openaiCompat' as const, apiKey: '', baseUrl: '' });

  async function load() {
    providers = await api.listProviders();
    for (const p of providers) {
      models[p.id] = await api.fetchModels(p.id);
    }
  }

  async function addProvider() {
    const p = await api.addProvider(newProvider.name, newProvider.type, newProvider.apiKey || null, newProvider.baseUrl);
    providers = [...providers, p];
    newProvider = { name: '', type: 'openaiCompat', apiKey: '', baseUrl: '' };
  }

  $effect(() => { load(); });
</script>

<div class="p-6 max-w-2xl">
  <h2 class="text-lg font-semibold mb-4">Providers</h2>

  {#each providers as provider (provider.id)}
    <div class="border border-[var(--border)] rounded-xl p-4 mb-3">
      <div class="flex justify-between items-center">
        <span class="font-medium">{provider.name}</span>
        <span class="text-xs px-2 py-1 rounded bg-[var(--bg-secondary)]">{provider.type}</span>
      </div>
      <div class="text-xs text-[var(--text-secondary)] mt-1">{provider.baseUrl}</div>
      {#if models[provider.id]?.length}
        <div class="mt-2 text-xs text-[var(--text-secondary)]">
          {models[provider.id].length} models available
        </div>
      {/if}
    </div>
  {/each}

  <div class="border border-dashed border-[var(--border)] rounded-xl p-4 mt-4">
    <h3 class="text-sm font-medium mb-3">Add Provider</h3>
    <div class="grid grid-cols-2 gap-3">
      <input bind:value={newProvider.name} placeholder="Name" class="input-field" />
      <select bind:value={newProvider.type} class="input-field">
        <option value="openaiCompat">OpenAI Compatible</option>
        <option value="anthropic">Anthropic</option>
        <option value="gemini">Google Gemini</option>
        <option value="ollama">Ollama</option>
      </select>
      <input bind:value={newProvider.apiKey} placeholder="API Key" type="password" class="input-field" />
      <input bind:value={newProvider.baseUrl} placeholder="Base URL" class="input-field" />
    </div>
    <button onclick={addProvider} class="mt-3 px-4 py-2 rounded-lg bg-[var(--accent)] text-white text-sm">
      Add
    </button>
  </div>
</div>

<style>
  .input-field {
    padding: 0.5rem 0.75rem;
    border-radius: 0.5rem;
    border: 1px solid var(--border);
    background: var(--bg-secondary);
    color: var(--text-primary);
    font-size: 0.875rem;
  }
</style>
```

**Step 2: Commit**

```bash
git add src/lib/components/settings/
git commit -m "feat: add provider settings UI"
```

### Task 19: 多模型对比视图

**Files:**
- Create: `src/lib/components/chat/CompareView.svelte`
- Create: `src/lib/components/chat/ModelSelector.svelte`

**Step 1: 创建 ModelSelector.svelte**

```svelte
<!-- src/lib/components/chat/ModelSelector.svelte -->
<script lang="ts">
  import type { ModelInfo } from '$lib/types';

  let { models, selected = $bindable(''), onSelect }: {
    models: ModelInfo[];
    selected: string;
    onSelect: (id: string) => void;
  } = $props();
</script>

<select
  bind:value={selected}
  onchange={() => onSelect(selected)}
  class="px-3 py-1.5 rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] text-sm text-[var(--text-primary)]"
>
  {#each models as model (model.id)}
    <option value={model.id}>{model.displayName ?? model.name}</option>
  {/each}
</select>
```

**Step 2: 创建 CompareView.svelte**

多模型对比：同一 prompt 发给多个模型，并排显示流式响应。

```svelte
<!-- src/lib/components/chat/CompareView.svelte -->
<script lang="ts">
  import MessageBubble from './MessageBubble.svelte';
  import type { Message } from '$lib/types';

  let { responses }: {
    responses: { modelId: string; modelName: string; message: Message }[];
  } = $props();
</script>

<div class="flex gap-4 overflow-x-auto p-4">
  {#each responses as resp (resp.modelId)}
    <div class="flex-1 min-w-[300px] border border-[var(--border)] rounded-xl p-3">
      <div class="text-xs font-medium text-[var(--accent)] mb-2">{resp.modelName}</div>
      <MessageBubble message={resp.message} />
    </div>
  {/each}
</div>
```

**Step 3: Commit**

```bash
git add src/lib/components/chat/CompareView.svelte src/lib/components/chat/ModelSelector.svelte
git commit -m "feat: add multi-model comparison view and model selector"
```

### Task 20: 搜索面板 + 助手列表

**Files:**
- Create: `src/lib/components/sidebar/SearchPanel.svelte`
- Create: `src/lib/components/sidebar/AssistantList.svelte`

**Step 1: 创建 SearchPanel.svelte**

```svelte
<!-- src/lib/components/sidebar/SearchPanel.svelte -->
<script lang="ts">
  import { api } from '$lib/utils/invoke';
  import type { Message } from '$lib/types';

  let query = $state('');
  let results = $state<Message[]>([]);

  async function handleSearch() {
    if (!query.trim()) { results = []; return; }
    results = await api.searchMessages(query);
  }
</script>

<div class="p-3">
  <input
    bind:value={query}
    oninput={handleSearch}
    placeholder="Search messages..."
    class="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] text-sm"
  />
  <div class="mt-2 max-h-64 overflow-y-auto">
    {#each results as msg (msg.id)}
      <div class="px-3 py-2 text-xs text-[var(--text-secondary)] border-b border-[var(--border)]">
        <span class="font-medium">{msg.role}</span>: {msg.content.slice(0, 100)}...
      </div>
    {/each}
  </div>
</div>
```

**Step 2: 创建 AssistantList.svelte**

```svelte
<!-- src/lib/components/sidebar/AssistantList.svelte -->
<script lang="ts">
  import { api } from '$lib/utils/invoke';
  import type { Assistant } from '$lib/types';

  let { onSelect }: { onSelect: (assistant: Assistant) => void } = $props();
  let assistants = $state<Assistant[]>([]);

  async function load() { assistants = await api.listAssistants(); }

  async function create() {
    const asst = await api.createAssistant('New Assistant');
    assistants = [...assistants, asst];
  }

  $effect(() => { load(); });
</script>

<div class="p-2">
  <button onclick={create} class="w-full text-left px-3 py-2 text-xs text-[var(--accent)] hover:bg-[var(--bg-secondary)] rounded-lg">
    + New Assistant
  </button>
  {#each assistants as asst (asst.id)}
    <button onclick={() => onSelect(asst)} class="w-full text-left px-3 py-2 text-sm truncate hover:bg-[var(--bg-secondary)] rounded-lg">
      {asst.icon ?? '🤖'} {asst.name}
    </button>
  {/each}
</div>
```

**Step 3: Commit**

```bash
git add src/lib/components/sidebar/
git commit -m "feat: add search panel and assistant list"
```

### Task 21: 最终集成 + 端到端验证

**Step 1: 确保所有 Tauri commands 在 main.rs 注册**

检查 `src-tauri/src/main.rs` 的 `invoke_handler` 包含所有命令。

**Step 2: 运行完整编译**

```bash
cd /home/cc/Github/orion-chat-rs
pnpm tauri build --debug
```

Expected: 编译成功，生成 debug 版本。

**Step 3: 手动端到端测试**

1. 启动应用 `pnpm tauri dev`
2. 进入设置，添加一个 OpenAI 兼容供应商（如 DeepSeek）
3. 拉取模型列表
4. 创建新对话
5. 发送消息，验证流式响应
6. 验证 Markdown 渲染、代码高亮
7. 测试停止生成
8. 测试全文搜索
9. 测试对话导出

**Step 4: 最终 Commit**

```bash
git add -A
git commit -m "feat: complete v1 integration — multi-provider AI chat with streaming"
```

---

## 任务总览

| Phase | Tasks | 描述 |
|-------|-------|------|
| 1 | Task 1 | 项目脚手架 (Tauri + SvelteKit) |
| 2 | Task 2-3 | 错误类型 + 数据模型 |
| 3 | Task 4-6 | SQLite 数据库层 (Schema + CRUD + FTS5) |
| 4 | Task 7-11 | Provider Trait + 4 个供应商实现 |
| 5 | Task 12-14 | AppState + Tauri Commands |
| 6 | Task 15-17 | 前端布局 + 聊天 UI + Markdown |
| 7 | Task 18-21 | 设置页面 + 多模型对比 + 集成验证 |

总计 21 个 Task，每个 Task 包含 3-7 个 Step。
