# Orion Chat — 设计文档

> 日期: 2026-03-01
> 目标: 用 Tauri v2 + Rust 重写 Cherry Studio 的核心聊天功能，砍掉冗余模块，实现极致内存占用的 AI Chat 集成工具。

## 1. 项目定位

Cherry Studio 的轻量替代品。保留核心聊天能力，砍掉 AI 绘图、知识库/RAG、OCR、TTS、翻译面板、内置 API Server、可视化 Flow 编辑器、系统级选中工具栏、WebDAV、300+ 预置 Assistant 等臃肿功能。

### 预估性能对比

| 指标 | Cherry Studio (Electron) | Orion Chat (Tauri) |
|------|-------------------------|-------------------|
| 安装包 | ~150MB | ~5-8MB |
| 空闲内存 | ~800MB-1GB | ~40-60MB |
| 依赖数 | 327 devDeps | ~15 Rust + ~10 npm |
| 启动时间 | 3-5s | <1s |

## 2. 功能范围

### 保留（v1）

- 多供应商多模型统一接入（OpenAI 兼容、Anthropic Claude、Google Gemini、Ollama）
- 流式响应（SSE/NDJSON 逐 token 推送）
- 对话管理（分组、拖拽排序、CRUD）
- Markdown 渲染（代码高亮、Mermaid 图表、LaTeX 公式）
- 模型参数调节（通用 + 供应商特有参数）
- 多模型对比回答（同一 prompt 并排对比）
- 自定义 Assistant（系统提示词 + 模型绑定 + 参数预设）
- 对话导入导出（Markdown/JSON）
- 全文搜索（SQLite FTS5）
- 暗色/亮色主题

### 砍掉

- AI 绘图面板
- 知识库/RAG
- OCR
- TTS
- 翻译面板
- 内置 API Server + Swagger
- 可视化 Flow 编辑器
- 系统级选中工具栏
- WebDAV 文件管理
- MCP 支持
- 300+ 预置 Assistant

## 3. 技术架构

### 架构方案：Rust 重后端

所有业务逻辑在 Rust 侧，Svelte 5 前端只做纯渲染层。

```
┌─────────────────────────────────────────┐
│  Svelte 5 Frontend (thin render layer)  │
│  - 消息列表渲染 (虚拟滚动)              │
│  - Markdown/代码高亮                    │
│  - UI 状态 (主题、布局)                 │
└──────────────┬──────────────────────────┘
               │ Tauri Commands + Channel<T>
┌──────────────▼──────────────────────────┐
│  Rust Backend (all business logic)      │
│  ├── providers/  (trait Provider)       │
│  │   ├── openai_compat.rs              │
│  │   ├── anthropic.rs                  │
│  │   ├── gemini.rs                     │
│  │   └── ollama.rs                     │
│  ├── db/         (rusqlite + FTS5)     │
│  ├── commands/   (tauri commands)      │
│  ├── streaming/  (SSE → Channel<T>)    │
│  └── models/     (Message, Conv, Asst) │
└─────────────────────────────────────────┘
```

## 4. 项目目录结构

```
orion-chat-rs/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   ├── icons/
│   └── src/
│       ├── main.rs                # Tauri 启动 + state 注册
│       ├── lib.rs                 # 模块声明
│       ├── error.rs               # 统一错误类型
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── chat.rs            # send_message, stop_generation
│       │   ├── conversation.rs    # CRUD 对话
│       │   ├── assistant.rs       # CRUD 自定义助手
│       │   ├── provider.rs        # 供应商配置管理
│       │   ├── search.rs          # 全文搜索
│       │   └── export.rs          # 导入导出
│       ├── providers/
│       │   ├── mod.rs             # trait Provider 定义
│       │   ├── openai_compat.rs   # OpenAI 兼容 (含 DeepSeek 等)
│       │   ├── anthropic.rs       # Claude
│       │   ├── gemini.rs          # Google Gemini
│       │   └── ollama.rs          # 本地模型
│       ├── db/
│       │   ├── mod.rs             # 连接池 + 初始化
│       │   ├── migrations.rs      # Schema 迁移
│       │   ├── conversations.rs   # 对话表操作
│       │   ├── messages.rs        # 消息表操作
│       │   └── assistants.rs      # 助手表操作
│       └── models/
│           ├── mod.rs
│           ├── message.rs         # Message, Role, MessageBlock
│           ├── conversation.rs    # Conversation, ConvGroup
│           ├── assistant.rs       # Assistant, AssistantConfig
│           └── provider.rs        # ProviderConfig, ModelInfo
├── src/                           # Svelte 5 前端
│   ├── app.html
│   ├── app.css                    # 全局样式 + CSS 变量主题
│   ├── lib/
│   │   ├── components/
│   │   │   ├── chat/
│   │   │   │   ├── MessageList.svelte    # 虚拟滚动消息列表
│   │   │   │   ├── MessageBubble.svelte  # 单条消息渲染
│   │   │   │   ├── InputArea.svelte      # 输入框 + 发送
│   │   │   │   ├── ModelSelector.svelte  # 模型选择器
│   │   │   │   └── CompareView.svelte    # 多模型对比视图
│   │   │   ├── sidebar/
│   │   │   │   ├── ConversationList.svelte
│   │   │   │   ├── AssistantList.svelte
│   │   │   │   └── SearchPanel.svelte
│   │   │   ├── settings/
│   │   │   │   ├── ProviderSettings.svelte
│   │   │   │   ├── ModelSettings.svelte
│   │   │   │   └── AppSettings.svelte
│   │   │   └── ui/               # 通用 UI 原子组件
│   │   │       ├── Button.svelte
│   │   │       ├── Dialog.svelte
│   │   │       ├── Toast.svelte
│   │   │       └── ...
│   │   ├── stores/
│   │   │   └── ui.svelte.ts      # 仅 UI 状态 (主题、侧边栏)
│   │   ├── utils/
│   │   │   ├── markdown.ts       # Markdown 渲染配置
│   │   │   └── invoke.ts         # Tauri command 类型封装
│   │   └── types/
│   │       └── index.ts          # 与 Rust 对应的 TS 类型
│   └── routes/
│       ├── +layout.svelte        # 主布局 (侧边栏 + 内容区)
│       └── +page.svelte          # 聊天主页面
├── static/
├── package.json
├── svelte.config.js
├── vite.config.ts
└── tsconfig.json
```

## 5. Provider Trait 抽象

```rust
use async_trait::async_trait;
use tauri::ipc::Channel;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum ChatEvent {
    Started { message_id: String },
    Delta { content: String },
    Reasoning { content: String },
    Usage { prompt_tokens: u32, completion_tokens: u32 },
    Finished { message_id: String },
    Error { message: String },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub common: CommonParams,
    pub provider_params: ProviderParams,
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        channel: Channel<ChatEvent>,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), ProviderError>;

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError>;

    async fn validate(&self) -> Result<bool, ProviderError>;
}
```

### Provider 实现差异

| Provider | SSE 格式 | 认证方式 | Thinking 控制 |
|----------|---------|---------|--------------|
| OpenAI 兼容 | `data: {json}\n\n` | `Bearer` header | `reasoning_effort`: low/medium/high (o 系列) |
| Anthropic | `event: content_block_delta` | `x-api-key` header | `thinking.type`: adaptive/enabled/disabled + `effort` |
| Gemini | `data: {json}\n\n` | API key in URL | `thinkingBudget`: 0-32768 或 `thinkingLevel` |
| Ollama | `{json}\n` (NDJSON) | 无认证 | `think`: bool 或 "low"/"medium"/"high" |

## 6. 模型参数设计

### 通用参数

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct CommonParams {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}
```

### 供应商特有参数（枚举，编译期类型安全）

```rust
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "provider_type")]
pub enum ProviderParams {
    OpenAICompat {
        frequency_penalty: Option<f32>,
        presence_penalty: Option<f32>,
        reasoning_effort: Option<ReasoningEffort>,  // o 系列
        seed: Option<u64>,
        max_completion_tokens: Option<u32>,          // o 系列替代 max_tokens
    },
    Anthropic {
        top_k: Option<u32>,
        thinking: Option<AnthropicThinking>,
        effort: Option<AnthropicEffort>,             // low/medium/high/max
    },
    Gemini {
        thinking_budget: Option<u32>,                // 2.5 系列: 0-32768
        thinking_level: Option<GeminiThinkingLevel>, // 3.x 系列
        safety_settings: Option<Vec<GeminiSafety>>,
    },
    Ollama {
        think: Option<OllamaThink>,
        num_ctx: Option<u32>,
        repeat_penalty: Option<f32>,
        min_p: Option<f32>,
        keep_alive: Option<String>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort { Low, Medium, High }

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicThinking {
    Adaptive,
    Enabled { budget_tokens: u32 },
    Disabled,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnthropicEffort { Low, Medium, High, Max }

#[derive(Clone, Serialize, Deserialize)]
pub enum GeminiThinkingLevel { Minimal, Low, Medium, High }

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OllamaThink {
    Bool(bool),
    Level(String),
}
```

## 7. 数据库 Schema

```sql
-- 供应商配置
CREATE TABLE providers (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    type        TEXT NOT NULL,  -- 'openai_compat' | 'anthropic' | 'gemini' | 'ollama'
    api_key     TEXT,           -- 加密存储，Ollama 为空
    base_url    TEXT NOT NULL,
    proxy       TEXT,
    is_enabled  INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 模型信息
CREATE TABLE models (
    id                TEXT PRIMARY KEY,
    provider_id       TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    display_name      TEXT,
    max_tokens        INTEGER,
    is_vision         INTEGER NOT NULL DEFAULT 0,
    supports_thinking INTEGER NOT NULL DEFAULT 0,
    is_enabled        INTEGER NOT NULL DEFAULT 1
);

-- 自定义助手
CREATE TABLE assistants (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    icon          TEXT,
    system_prompt TEXT,
    model_id      TEXT REFERENCES models(id),
    temperature   REAL,
    top_p         REAL,
    max_tokens    INTEGER,
    extra_params  TEXT DEFAULT '{}',  -- JSON, 对应 ProviderParams 枚举
    sort_order    INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 对话
CREATE TABLE conversations (
    id           TEXT PRIMARY KEY,
    title        TEXT NOT NULL,
    assistant_id TEXT REFERENCES assistants(id),
    model_id     TEXT REFERENCES models(id),
    is_pinned    INTEGER NOT NULL DEFAULT 0,
    sort_order   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 消息
CREATE TABLE messages (
    id               TEXT PRIMARY KEY,
    conversation_id  TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role             TEXT NOT NULL,  -- 'system' | 'user' | 'assistant'
    content          TEXT NOT NULL,
    model_id         TEXT,           -- 多模型对比时标记来源
    reasoning        TEXT,           -- thinking/reasoning 内容
    token_prompt     INTEGER,
    token_completion INTEGER,
    status           TEXT NOT NULL DEFAULT 'done',  -- 'streaming' | 'done' | 'error'
    created_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

-- FTS5 全文搜索
CREATE VIRTUAL TABLE messages_fts USING fts5(
    content,
    content=messages,
    content_rowid=rowid
);

CREATE TRIGGER messages_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TRIGGER messages_ad AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content)
        VALUES('delete', old.rowid, old.content);
END;
```

## 8. 流式消息数据流

```
用户输入 → Svelte InputArea
    │
    ▼ invoke("send_message", { conv_id, content, model_id })
    │
    ▼ Rust: commands/chat.rs
    ├── 1. 写入 user message 到 SQLite (status: "done")
    ├── 2. 创建 assistant message 占位 (status: "streaming")
    ├── 3. 从 DB 加载对话历史 → 构建 ChatRequest
    ├── 4. 根据 model.provider_id 路由到对应 Provider
    │
    ▼ Provider::stream_chat(request, channel, cancel)
    │   ├── reqwest 发起 HTTP 请求
    │   ├── 逐行解析 SSE/NDJSON
    │   │   ├── OpenAI: "data: {json}\n\n"
    │   │   ├── Anthropic: "event: content_block_delta\ndata: {json}"
    │   │   ├── Gemini: "data: {json}\n\n"
    │   │   └── Ollama: "{json}\n" (NDJSON)
    │   ├── 每个 token → channel.send(ChatEvent::Delta { content })
    │   ├── thinking → channel.send(ChatEvent::Reasoning { content })
    │   └── 检查 cancel receiver，收到信号则中断
    │
    ▼ Tauri Channel<ChatEvent> → Svelte onEvent callback
    │   ├── Delta → 追加到当前消息的 content
    │   ├── Reasoning → 追加到折叠的 thinking 区域
    │   ├── Usage → 显示 token 用量
    │   ├── Finished → 标记完成
    │   └── Error → 显示错误提示
    │
    ▼ Rust: stream 结束后
    ├── 更新 assistant message (content, reasoning, tokens, status: "done")
    └── 更新 conversation.updated_at
```

### 多模型对比流程

```
用户点击"对比模式" → 选择 N 个模型
    │
    ▼ invoke("send_message_compare", { conv_id, content, model_ids: [...] })
    │
    ▼ Rust: 为每个模型创建独立的 assistant message 占位
    ├── tokio::spawn(provider_a.stream_chat(..., channel_a, cancel))
    ├── tokio::spawn(provider_b.stream_chat(..., channel_b, cancel))
    └── tokio::spawn(provider_c.stream_chat(..., channel_c, cancel))
    │
    ▼ Svelte: 每个 channel 独立回调，并排渲染 CompareView
```

### 停止生成

```rust
#[tauri::command]
async fn stop_generation(state: State<'_, AppState>) -> Result<()> {
    state.cancel_sender.send(true)?;
    Ok(())
}
```

## 9. 依赖选型

### Rust (src-tauri/Cargo.toml)

| Crate | 用途 |
|-------|------|
| `tauri` v2 | 桌面框架 |
| `tauri-plugin-store` | KV 配置存储 |
| `reqwest` + `eventsource-stream` | HTTP + SSE |
| `rusqlite` (bundled) | SQLite + FTS5 |
| `serde` / `serde_json` | 序列化 |
| `tokio` | 异步运行时 |
| `uuid` | ID 生成 |
| `async-trait` | Provider trait |
| `thiserror` | 错误类型 |
| `keyring` | API Key 安全存储 |

### Svelte 前端 (package.json)

| 包 | 用途 |
|----|------|
| `@tauri-apps/api` | Tauri IPC |
| `marked` + `highlight.js` | Markdown + 代码高亮 |
| `katex` | LaTeX 公式 |
| `mermaid` | 图表渲染 |
| `@tanstack/svelte-virtual` | 虚拟滚动 |
| `bits-ui` | 无样式组件原语 |
| `tailwindcss` v4 | 样式 |

## 10. UI 设计方向

极简现代风，参考 Arc Browser / Linear 设计语言：

- 毛玻璃效果侧边栏
- 圆角卡片式消息
- 微动画过渡
- CSS 变量驱动的暗色/亮色主题切换
- 信息密度适中，留白充足
