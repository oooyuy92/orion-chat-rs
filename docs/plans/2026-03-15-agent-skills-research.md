# Agent Skills 能力调研报告

> 日期：2026-03-15
> 目标：为 Orion Chat RS 引入 AI Agent 执行 Skills 的能力，调研可行方案

---

## 一、方案总览

### 1.1 已淘汰方案

| 方案 | 淘汰原因 |
|------|---------|
| **Claude Agent SDK** | 子进程模型，每次启动 ~50K token 开销；仅支持 Anthropic；需 Node.js 18+；会绕开现有 Provider/流式架构 |
| **ACP 协议** | IBM 创建，2025-08 已归档停止开发，合并入 Google A2A 协议 |
| **LangGraph** | Python，内存 400-1000MB，依赖链重 |
| **CrewAI** | Python，需 sidecar 进程 |
| **AutoGen** | Python，~400-500MB 内存，进入维护模式 |
| **OpenHands** | 需 Docker per session，开销大 |
| **pi-agent** | TypeScript，需 Node.js sidecar |
| **Kalosm** | 仅本地模型推理，无 Agent/Tool 能力 |
| **llm-chain** | 已废弃，签名密钥过期 |
| **swarm-rs** | 仅 OpenAI，实验性质 |
| **Devon** | AGPL 许可证，限制商用 |
| **Aider** | 无官方 API，仅 CLI |
| **Cline/Continue** | IDE 扩展，不可嵌入 |

### 1.2 候选方案对比（Rust 原生）

| 框架 | Stars | 内置工具 | 多 Agent | MCP | Provider 数 | 可作 lib 嵌入 |
|------|-------|---------|---------|-----|-----------|-------------|
| **yoagent** | 80 | bash/file/edit/search/list | SubAgentTool | Client | 20+ | 是 |
| **Rig** | 6.2k | 无 | 可组合 | 有示例 | 20+ | 是 |
| **Goose** (core) | 27k+ | shell/editor/analyze/screenshot | 10 并发子 Agent | 原生 | 30+ | 部分可行 |
| **ADK-Rust** | 新 | Google Search + MCP | Sequential/Parallel/Loop/Graph | 是 | 10+ | 是(模块化) |
| **agentai** | 新 | builtin + web | 无 | Client | 15+ (genai) | 是 |

---

## 二、Rust 原生 Agent 框架详细分析

### 2.1 yoagent — "Baby Claude Code"

- **仓库**: [github.com/yologdev/yoagent](https://github.com/yologdev/yoagent)
- **版本**: v0.6.1 (2026-03-13)
- **许可证**: MIT
- **代码量**: ~8200 行 Rust
- **维护者**: 1 人 (Yuanhao Li) + Claude Opus 协作
- **架构**: 纯 library crate，3 层设计

```
Layer 1 — Agent Loop (agent_loop.rs): 无状态核心
  Prompt → LLM Stream → Tool Exec → Loop if tool_call → Done

Layer 2 — Agent (agent.rs): 有状态包装
  消息历史、工具注册、Provider 选择、队列模式、生命周期钩子、Skills

Layer 3 — 消费者自行实现
  多 Agent 编排、规划等
```

**源码结构**:

```
src/
  lib.rs              — 模块声明、re-exports
  types.rs            — Content, Message, AgentEvent, AgentTool trait, ToolContext
  agent_loop.rs       — agent_loop(), agent_loop_continue(), run_loop()
  agent.rs            — Agent struct, builder methods, 消息持久化
  context.rs          — ContextTracker, CompactionStrategy, DefaultCompaction (3级)
  retry.rs            — RetryConfig 指数退避 + jitter
  sub_agent.rs        — SubAgentTool (AgentTool 实现)
  skills.rs           — SkillSet loader (AgentSkills 兼容)
  tools/              — bash.rs, file.rs, edit.rs, list.rs, search.rs
  provider/           — anthropic, openai_compat, openai_responses, azure_openai,
                        google, google_vertex, bedrock, traits, registry, model, sse, mock
  mcp/                — client.rs, transport.rs, tool_adapter.rs, types.rs
  openapi/            — adapter.rs, types.rs (feature-gated)
```

**内置工具**:

| 工具 | 名称 | 能力 | 限制 |
|------|------|------|------|
| BashTool | `bash` | shell 执行，超时 120s，输出上限 256KB，deny patterns 拦截危险命令，可选用户确认回调 | 无沙箱，deny patterns 为字符串包含检查可绕过 |
| ReadFileTool | `read_file` | 文本文件 (1MB) 带行号，支持 offset/limit 分段读取；图片 (20MB) 返回 base64 | 不支持 PDF、notebook |
| WriteFileTool | `write_file` | 创建文件，自动创建父目录 | 全量覆写，无追加模式 |
| EditFileTool | `edit_file` | 精确搜索替换，失败时给出模糊匹配提示，校验唯一性 | 不支持正则 |
| ListFilesTool | `list_files` | 使用 `find` 命令，支持 glob、递归深度，排除 target/.git/node_modules | 依赖 shell 的 find 命令，上限 200 结果 |
| SearchTool | `search` | 优先用 ripgrep，fallback 到 grep，支持正则、大小写、glob 过滤 | 无 context lines，无 output mode，上限 50 结果 |

**Provider 支持 (7 协议 20+ Provider)**:

| 协议 | 覆盖 Provider |
|------|-------------|
| Anthropic Messages | Claude 全系列 |
| OpenAI Completions | OpenAI, xAI, Groq, Cerebras, OpenRouter, Mistral, DeepSeek, MiniMax, HuggingFace, Kimi 等 |
| OpenAI Responses | OpenAI Responses API |
| Azure OpenAI | Azure OpenAI |
| Google GenAI | Google Gemini |
| Google Vertex | Vertex AI |
| Bedrock ConverseStream | AWS Bedrock |

**子 Agent 机制**:
- `SubAgentTool` 实现 `AgentTool` trait
- 每次调用启动全新 `agent_loop()`，独立 system prompt、工具集、model、provider
- 上下文隔离：每次是全新对话
- 深度限制：子 Agent 不会被赋予 SubAgentTool（防止无限递归）
- 取消传播：父级 cancel token 转发给子级
- 事件转发：子 Agent 事件通过 `on_update` 回调流向父级

**Skills 支持**:
- `SkillSet` loader 兼容 AgentSkills 格式
- 扫描目录下的 `SKILL.md` 文件，解析 YAML frontmatter
- 将 skill 内容注入 system prompt 或作为工具描述
- 是文本级别的 prompt 注入，非可执行代码

**上下文管理 (DefaultCompaction 3 级)**:
- Level 1: 截断冗长的工具输出（保留首尾部分）
- Level 2: 压缩旧的 assistant 响应为简短摘要
- Level 3: 完全移除中间消息，仅保留首尾

**MCP Client**:
- 支持 stdio + HTTP transport
- JSON-RPC 2.0 协议
- initialize() / list_tools() / call_tool() / close()
- McpToolAdapter 将 MCP tools 包装为 AgentTool
- 缺失：resources/prompts/sampling/notifications/reconnect/SSE transport

**测试覆盖**: 107 tests + 3 ignored，0 clippy warnings

---

### 2.2 Rig — 最成熟 Rust LLM 框架

- **仓库**: [github.com/0xPlaygrounds/rig](https://github.com/0xPlaygrounds/rig)
- **版本**: v0.31
- **Stars**: ~6,186
- **许可证**: MIT

**架构**: 纯 library crate。Agent = Model + SystemPrompt + Tools + optional RAG pipeline。

**优势**:
- 20+ Provider 统一 API
- 生产级，St Jude 医院等用户
- 基准: 峰值 <1.1GB，CPU 24.3%，Python 框架的 1/5
- WASM 支持进行中
- 完善的 Tool trait + ToolEmbedding（语义搜索动态选工具）

**劣势**: **无内置工具** — 需自行实现 bash/file/edit 等全套

---

### 2.3 Goose — 最完整 Rust Agent (Block/Square)

- **仓库**: [github.com/block/goose](https://github.com/block/goose)
- **Stars**: ~27,262
- **许可证**: Apache-2.0
- **架构**: Rust workspace (goose/goose-cli/goose-server/goose-mcp/goose-acp)

**内置工具 (Developer Extension)**:
- `developer__shell` — shell 命令执行
- `developer__text_editor` — view/write/str_replace/undo
- `developer__analyze` — 代码结构分析
- `developer__screen_capture` — 截屏
- `developer__image_processor` — 图片处理

**其他扩展**: Computer Controller (web_search, automation_script)、Memory (store/search)、Auto Visualiser

**Provider 支持**: 30+，包括 API/Cloud/Local/CLI pass-through

**多 Agent**: 最多 10 个并发子 Agent，Recipe 系统，Summon 扩展

**集成挑战**: 不是设计为 library 的。集成路径：
1. Sidecar（打包 goosed 二进制，HTTP/SSE 通信）— 最稳定
2. Crate 依赖（非官方，需跟踪 API 变更）

**注意**: 正在讨论从 Electron 迁移到 Tauri v2

---

### 2.4 ADK-Rust — 25 crate 模块化

- **仓库**: [github.com/zavora-ai/adk-rust](https://github.com/zavora-ai/adk-rust)
- **许可证**: Apache-2.0

**模块**: adk-core, adk-agent, adk-model, adk-tool, adk-graph, adk-server, adk-realtime 等

**Agent 类型**: LlmAgent, SequentialAgent, ParallelAgent, LoopAgent, 自定义

**特色**: Graph workflow 编排、RAG 管道 (6 种向量库)、RBAC、实时语音 Agent、A2A 协议

**劣势**: 功能过重，25 crate 引入不必要复杂度

---

### 2.5 agentai — 最轻量

- **仓库**: [github.com/AdamStrojek/rust-agentai](https://github.com/AdamStrojek/rust-agentai)
- **许可证**: MIT
- **基于**: genai crate (15+ Provider)

**特点**: `#[toolbox]` 宏定义工具，内置 tools-buildin + tools-web，MCP client 支持

**劣势**: 无多 Agent 支持，工具生态不如 yoagent 丰富

---

### 2.6 其他 Rust Agent 框架

| 框架 | 特点 | 适用场景 |
|------|------|---------|
| **AutoAgents** (292 stars) | Actor 模型 + WASM 沙箱工具执行，4ms 冷启动 | 需要安全沙箱的场景 |
| **swarms-rs** | 企业级多 Agent 编排，MCP 原生 | 大规模 Agent 编排 |
| **fcn06/swarm** | MCP + A2A，Agent Factory 动态创建，自纠正 | 分布式 Agent 网络 |
| **rs-agent / Lattice** | UTCP bridge，CodeMode，Postgres/Qdrant memory | 需要持久化 memory 的场景 |
| **rust-agent** | MCP client/server，混合本地+远程工具 | Web3 场景 |
| **neuron** | 可组合积木，"serde of agent frameworks" | 极简定制 |

---

## 三、MCP 工具生态

### 3.1 Rust MCP 实现

| 实现 | 版本 | 说明 |
|------|------|------|
| **rmcp** (官方) | v1.2.0 | modelcontextprotocol/rust-sdk，Client + Server，可 in-process 运行 |
| rust-mcp-sdk | - | 完整实现 MCP 2025-11-25 spec |
| pmcp | - | 号称比 TypeScript 快 16x |

**In-Process 运行方式（无需子进程）**:

方式 A — `tokio::io::duplex`:
```rust
let (client_stream, server_stream) = tokio::io::duplex(1024);
// Server 端
tokio::spawn(async move { my_server.serve(server_stream).await });
// Client 端
let client = ().serve(client_stream).await?;
```

方式 B — `rmcp-in-process-transport` crate：专用 in-process transport

### 3.2 Rust 原生 MCP Server（无需 Node.js）

| Server | 工具 |
|--------|------|
| rust-mcp-filesystem | read/write/list/search(glob)/zip/unzip/metadata |
| winx-code-agent | shell(PTY)/file read-write-edit/image |

### 3.3 Tauri + MCP 集成项目

| 项目 | 说明 |
|------|------|
| tauri-plugin-mcp (moeru-ai) | Tauri v2 插件，连接外部 MCP Server，v0.7.1 |
| mcp-bouncer | Tauri v2 app，使用 rmcp，已验证可行 |

### 3.4 MCP Server 注册中心

| 注册中心 | 规模 |
|---------|------|
| registry.modelcontextprotocol.io | 官方 |
| mcp.so | 18,503 servers |
| glama.ai/mcp/servers | 19,281 servers |

### 3.5 内存开销参考

| 语言 | MCP Server 内存 | 延迟 |
|------|---------------|------|
| Go | 18 MB | 0.855ms |
| Rust (估计) | 10-20 MB | <1ms |
| Node.js | 50-80 MB | 10-30ms |
| Python | 60-100 MB | 26.45ms |

---

## 四、非 Rust 但值得参考的框架

### 4.1 Goose 集成路径（Rust，但非 library 模式）

| 路径 | 说明 | 开销 |
|------|------|------|
| Sidecar | 打包 goosed 二进制，localhost HTTP/SSE | +30-50MB 内存 |
| Crate 依赖 | `use goose::agent::Agent` | 非官方支持 |
| MCP bridge | Orion 暴露 MCP Server，Goose 连接 | 最清晰的分离 |
| ACP client | JSON-RPC over stdio | 新兴协议 |

### 4.2 工具/沙箱平台

| 平台 | 用途 | 集成方式 | 本地可用 |
|------|------|---------|---------|
| Composio | 500+ SaaS 工具集成 (GitHub/Slack/Gmail…) | REST API | 否(云) |
| Toolhouse | 40+ 工具 + RAG/memory | REST API | 否(云) |
| E2B | 沙箱代码执行 (Firecracker microVM) | Python/TS SDK | 可自托管 |
| Daytona | Docker 沙箱环境，<200ms 启动 | REST API / Go SDK | 可自托管 |

---

## 五、关键维度交叉对比

### 5.1 内存占用（纯 Agent 运行时，不含模型推理）

| 框架 | 估计内存 | 说明 |
|------|---------|------|
| **yoagent** | ~5-15 MB | 纯 library，无子进程 |
| **agentai** | ~5-15 MB | 同上 |
| **Rig** | ~10-20 MB | 含 RAG 时 ~100MB |
| **Goose sidecar** | ~30-50 MB | 独立进程 + Axum HTTP server |
| **Goose in-process** | ~15-30 MB | 仅 core crate |
| **ADK-Rust** | ~20-40 MB | 取决于引入的 crate 数量 |
| ~~Python 框架~~ | ~~400-1000 MB~~ | ~~参考值~~ |

### 5.2 工具执行延迟

| 框架 | 执行延迟 | 说明 |
|------|---------|------|
| **yoagent** | <1ms | in-process Rust 函数调用 |
| **agentai** | <1ms | 同上 |
| **Rig** | <1ms | 同上 |
| **Goose builtin** | <1ms | in-process MCP |
| **Goose stdio** | ~5-10ms | 子进程 JSON-RPC |
| **MCP stdio** | ~5-10ms | 子进程序列化开销 |
| **MCP in-process** | ~1-2ms | tokio duplex，仍有 JSON-RPC 序列化 |

### 5.3 多 Agent 编排能力

| 框架 | 能力 | 模式 |
|------|------|------|
| **Goose** | 强 | 10 并发子 Agent，Recipe 系统 |
| **ADK-Rust** | 最强 | Sequential/Parallel/Loop/Graph，A2A |
| **yoagent** | 中 | SubAgentTool 委派，单层深度 |
| **Rig** | 可组合 | 手动编排，无内置 orchestrator |
| **agentai** | 无 | 单 Agent |

### 5.4 Provider Tool Use 兼容性

| Provider | tool_use 支持 | 格式 | 流式 tool_use |
|----------|-------------|------|-------------|
| OpenAI | 完善 | `tools` + `function` | 支持 (delta chunks) |
| Anthropic | 完善 | `tools` + `tool_use` content block | 支持 (content_block_start/delta) |
| Gemini | 完善 | `tools` + `functionDeclarations` | 支持 |
| Ollama | 部分 | OpenAI 兼容格式 | 取决于模型 |

---

## 六、yoagent vs Claude Code Gap 分析

### 6.1 已覆盖的能力（~40%）

| 能力 | Claude Code | yoagent | 差距 |
|------|-----------|---------|------|
| Agent Loop | while(tool_call) 循环 | 相同模式 | 无 |
| 文件读取 | Read (text/image/PDF/notebook) | ReadFileTool (text + image) | 缺 PDF、notebook |
| 文件写入 | Write | WriteFileTool | 功能对等 |
| 文件编辑 | Edit (精确替换) | EditFileTool (精确替换 + fuzzy hint) | yoagent 更好 |
| Shell 执行 | Bash | BashTool | 功能对等 |
| 文件搜索 | Glob (专用 glob) | ListFilesTool (依赖 find) | 较弱 |
| 内容搜索 | Grep (ripgrep, context lines) | SearchTool (rg/grep fallback) | 缺 context lines |
| 子 Agent | Task (7-10 并发, 多类型) | SubAgentTool (单层) | 缺多级递归 |
| 上下文压缩 | 自动 3 阶段 + LLM 摘要 | DefaultCompaction 3 级 | 缺 LLM 摘要 |
| 会话持久化 | JSONL + session index | JSON save/restore | 缺 session ID |
| Prompt 缓存 | 多层级，90% 折扣 | CacheConfig + 3 breakpoints | 基本对等 |
| 生命周期钩子 | PreToolUse/PostToolUse/6+ 事件 | before_turn/after_turn/on_error | 缺 per-tool 钩子 |
| 多 Provider | 仅 Anthropic | 7 协议 20+ provider | **yoagent 更强** |
| 流式事件 | SSE + React Ink | AgentEvent mpsc channel | 对等 |
| 重试机制 | 指数退避 + 熔断 | 指数退避 + jitter | 缺熔断器 |
| 输入过滤 | 无公开机制 | InputFilter trait | **yoagent 更强** |
| 中途打断 | h2A 实时 steering | SteeringMessage::Abort/FollowUp | 对等 |
| OpenAPI 工具 | 无 | 从 spec 自动生成工具 | **yoagent 独有** |
| Skills | slash commands + marketplace | SkillSet (SKILL.md frontmatter) | 格式兼容 |

### 6.2 完全缺失的能力（~60%）

| 缺失能力 | Claude Code 实现 | 补齐难度 | 估计工作量 |
|---------|----------------|---------|----------|
| WebSearch | 内置 web search 工具 | 低 | ~100 行 |
| WebFetch | URL → Markdown 转换 | 低 | ~150 行 |
| LSP 集成 | goToDefinition/findReferences/hover | 高 | tower-lsp client |
| PDF 阅读 | Read 支持 PDF | 中 | pdf-extract crate |
| Notebook 编辑 | NotebookEdit | 中 | .ipynb JSON 解析 |
| 权限系统 | allow/deny/ask + 记忆 + glob 匹配 | 中 | 全新模块 |
| Per-tool 钩子 | PreToolUse/PostToolUse | 低 | agent_loop 插入回调 |
| AskUserQuestion | 结构化多选交互 | 低 | 新 AgentTool |
| Plan Mode | 只读分析 | 中 | 工具集切换 + UI |
| Task Tracking | TodoWrite 任务列表 | 低 | Vec + AgentTool |
| Git 集成 | commit/PR/diff | 中 | git2 crate |
| 文件检查点 | Edit/Write 前自动快照 | 中 | hash + 备份 |
| Worktree 隔离 | 子 Agent 在 git worktree 执行 | 中 | git worktree |
| 项目记忆 | CLAUDE.md 3 层级 + auto memory | 低 | 读 .md 注入 prompt |
| Model Routing | opusplan | 低 | 多 Provider 切换 |
| MCP Resources/Prompts | 完整 MCP spec | 中 | 扩展 MCP client |
| 多级子 Agent | Explore/Plan/General/Bash/Custom | 中 | 扩展 SubAgentTool |
| 并发子 Agent | 7-10 并发 | 低 | tokio::JoinSet |

### 6.3 不可复制的能力（模型层面）

| 能力 | 原因 |
|------|------|
| Adaptive Thinking | Claude 模型原生能力 |
| 模型-框架协调优化 | 110+ prompt 片段针对 Claude 模型专门调优 |
| Prompt 缓存 90% 折扣 | Anthropic 基础设施 |
| AskUserQuestion 结构化输出 | Claude 模型训练行为 |

---

## 七、当前架构适配分析

### 7.1 Orion Chat 已有基础设施

| 已有能力 | 如何复用 |
|---------|---------|
| Provider trait + `stream_chat()` | 扩展参数增加 tools，或替换为 yoagent 的 StreamProvider |
| ChatEvent 流式事件 | 映射 AgentEvent → ChatEvent |
| Per-conversation cancel token | 传递给 yoagent 的 CancellationToken |
| Message 版本化 | 工具调用消息可作为隐藏版本 |
| invoke.ts 双 API 层 | 新增工具管理 API |
| 消息软删除 | 工具调用失败的消息可软删除重试 |
| 多 Provider 支持 | yoagent 的 ProviderRegistry 覆盖更全 |

### 7.2 需新建的部分

- 工具 schema 存储与管理
- 工具执行引擎集成
- 安全/权限控制层
- 前端工具调用 UI 组件（ToolCall/ToolResult 渲染）
- Agent 模式切换 UI

---

## 八、推荐方案

### 首选：fork yoagent + rmcp

| 要求 | 满足度 |
|------|--------|
| 最省内存 | ~5-15MB，纯 in-process |
| 速度最快 | <1ms 工具执行 |
| 兼容现有架构 | Cargo 依赖，事件流直接映射 ChatEvent |
| 多 Agent | SubAgentTool，可扩展并发 |
| 内置工具 | bash/file/edit/search/list 五件套 |

**集成架构**:

```
Orion Chat Tauri 进程
├── 现有 Provider 层 (chat.rs)           ← 普通聊天，不变
├── yoagent Agent loop                   ← 新增，Agent 模式
│   ├── 内置工具: bash, file, edit, search, list
│   ├── SubAgentTool: 子任务委派
│   └── MCP 工具: 通过 rmcp client 连接外部 MCP Server
├── rmcp (in-process)                    ← 可选，动态工具扩展
└── 前端 ChatEvent 统一渲染
```

**许可证**: MIT，可 fork、修改、商用、闭源分发。所有依赖均为 MIT/Apache-2.0。

### 备选：自研 Agent Loop + rmcp

如果 yoagent 成熟度不足：

```
Orion Chat Tauri 进程
├── 自研 Agent Loop (~300 行 Rust)
│   ├── while !done { call_provider → parse_tool_calls → execute → loop }
│   ├── 工具: tokio::fs + tokio::process 原生实现
│   └── 工具 schema: 手动定义 JSON Schema
├── rmcp client                         ← 连接外部 MCP Server
└── 复用现有 Provider trait 的 stream_chat
```

---

## 九、二次开发路线图

### Phase 0：直接集成（1-2 天）
- Fork yoagent，作为 workspace member 加入 src-tauri/
- AgentEvent → ChatEvent 映射
- 前端新增 "Agent 模式" 开关

### Phase 1：补齐高价值低成本能力（3-5 天）
- WebSearchTool (Brave/Tavily API)
- WebFetchTool (reqwest + readability)
- AskUserQuestionTool (前端交互)
- 增强权限系统 (per-tool allow/deny/ask)
- Per-tool PreToolUse/PostToolUse 钩子

### Phase 2：补齐中等难度能力（1-2 周）
- 项目记忆 (.orion.md → system prompt)
- 文件检查点 (Edit/Write 前自动备份)
- Git 集成 (git2 crate 或 BashTool)
- 增强 MCP Client (resources + prompts + reconnect)
- 多级子 Agent + 并发执行

### Phase 3：高级能力（按需）
- LSP 集成
- PDF/Notebook 支持
- Plan Mode
- Model Routing (不同任务用不同模型)
- A2A 协议支持

> Phase 1 结束即超过 yoagent 原版能力
> Phase 2 结束可覆盖 Claude Code ~70% 核心功能，且多 Provider
