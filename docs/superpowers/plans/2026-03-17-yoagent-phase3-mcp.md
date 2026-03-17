# yoagent Phase 3 — MCP 集成 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 MCP 服务器的添加/删除/连接管理，MCP 工具动态注册到 Agent 的工具列表中，并在设置页面的工具授权列表中展示。

**Architecture:** 后端新增 `agent/mcp.rs` 管理 MCP 服务器生命周期（连接/断开/重连），使用 yoagent 内置的 MCP client（stdio + HTTP 传输）连接外部服务器并发现工具。前端在设置页面新增 MCP 服务器管理面板。

**Tech Stack:** Rust / yoagent MCP client, Svelte 5, shadcn-svelte, Tauri v2

**前置依赖:** Phase 0 + Phase 1 + Phase 2 已完成。

---

## 文件结构

### 新建文件
- `src-tauri/src/agent/mcp.rs` — MCP 服务器连接管理、工具发现
- `src/lib/components/settings/McpServerManager.svelte` — MCP 服务器管理面板
- `src/lib/components/settings/McpServerForm.svelte` — 添加/编辑 MCP 服务器表单

### 修改文件
- `src-tauri/src/agent/mod.rs` — 添加 `pub mod mcp;`
- `src-tauri/src/agent/commands.rs` — 添加 MCP 管理 commands（已在 Phase 0 声明签名）
- `src-tauri/src/agent/commands.rs` — 修改 `agent_chat` 以注册 MCP 工具
- `src-tauri/src/state.rs` — AppState 添加 MCP 连接池
- `src-tauri/src/lib.rs` — 注册新 commands
- `src/lib/components/settings/AgentSettings.svelte` — 添加 MCP 分区

---

## Chunk 0: MCP 后端

### Task 0: 查看 yoagent MCP client API

- [ ] **Step 1: 查看 yoagent MCP client 公开接口**

```bash
grep -rn "pub fn\|pub async fn\|pub struct\|pub trait" \
  src-tauri/crates/yoagent/src/mcp/ | head -30
```

记录：`McpClient::new()`, `initialize()`, `list_tools()`, `call_tool()`, `close()` 的实际签名。

- [ ] **Step 2: 查看 MCP tool adapter**

```bash
cat src-tauri/crates/yoagent/src/mcp/tool_adapter.rs | head -60
```

了解如何将 MCP 工具包装为 `AgentTool`。

### Task 1: agent/mcp.rs — MCP 连接管理

**Files:**
- Create: `src-tauri/src/agent/mcp.rs`
- Modify: `src-tauri/src/state.rs`

- [ ] **Step 1: 在 AppState 添加 MCP 连接池**

```rust
// state.rs 新增
use std::collections::HashMap;

// AppState 中添加
pub mcp_clients: Mutex<HashMap<String, yoagent::mcp::McpClient>>,
```

初始化：`mcp_clients: Mutex::new(HashMap::new()),`

- [ ] **Step 2: 创建 mcp.rs**

```rust
// src-tauri/src/agent/mcp.rs
use std::sync::Arc;
use yoagent::mcp::{McpClient, McpTransport as YoMcpTransport};
use yoagent::AgentTool;
use crate::models::agent::{McpServerConfig, McpServerStatus, McpTransport};
use crate::state::AppState;
use crate::errors::{AppResult, AppError};

/// 连接一个 MCP 服务器，返回工具列表
pub async fn connect_server(
    state: &Arc<AppState>,
    config: &McpServerConfig,
) -> AppResult<Vec<String>> {
    let transport = match config.transport {
        McpTransport::Stdio => {
            // 按 yoagent McpClient API 调整
            YoMcpTransport::stdio(&config.command_or_url, &config.args)
                .map_err(|e| AppError::Mcp(e.to_string()))?
        }
        McpTransport::Http => {
            YoMcpTransport::http(&config.command_or_url)
                .map_err(|e| AppError::Mcp(e.to_string()))?
        }
    };

    let mut client = McpClient::new(transport);
    client.initialize().await.map_err(|e| AppError::Mcp(e.to_string()))?;

    let tools = client.list_tools().await.map_err(|e| AppError::Mcp(e.to_string()))?;
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();

    state.mcp_clients.lock().await.insert(config.name.clone(), client);

    Ok(tool_names)
}

/// 断开 MCP 服务器
pub async fn disconnect_server(
    state: &Arc<AppState>,
    name: &str,
) -> AppResult<()> {
    if let Some(mut client) = state.mcp_clients.lock().await.remove(name) {
        let _ = client.close().await;
    }
    Ok(())
}

/// 获取所有已连接 MCP 服务器的工具列表，包装为 AgentTool
pub async fn get_mcp_tools(
    state: &Arc<AppState>,
) -> Vec<Box<dyn AgentTool>> {
    let clients = state.mcp_clients.lock().await;
    let mut tools: Vec<Box<dyn AgentTool>> = Vec::new();

    for (_name, client) in clients.iter() {
        // yoagent 的 tool_adapter 将 MCP 工具包装为 AgentTool
        // 按实际 API 调整
        if let Ok(mcp_tools) = client.list_tools().await {
            for tool in mcp_tools {
                // 使用 yoagent 的 McpToolAdapter
                tools.push(Box::new(yoagent::mcp::McpToolAdapter::new(
                    client.clone(),
                    tool,
                )));
            }
        }
    }

    tools
}

/// 检查服务器连接状态
pub async fn get_server_statuses(
    state: &Arc<AppState>,
    configs: &[McpServerConfig],
) -> Vec<McpServerStatus> {
    let clients = state.mcp_clients.lock().await;
    configs
        .iter()
        .map(|config| McpServerStatus {
            config: config.clone(),
            connected: clients.contains_key(&config.name),
        })
        .collect()
}
```

**注意**：yoagent MCP client API 可能与上述代码不完全一致，需按 Step 0 的实际查询结果调整。

- [ ] **Step 3: 在 mod.rs 中添加模块**

```rust
pub mod mcp;
```

- [ ] **Step 4: 编译验证**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/agent/mcp.rs src-tauri/src/agent/mod.rs src-tauri/src/state.rs
git commit -m "feat: agent/mcp - MCP server connection management and tool discovery"
```

### Task 2: MCP Tauri Commands

**Files:**
- Modify: `src-tauri/src/agent/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 实现 MCP 管理 commands**

```rust
// commands.rs 中添加

use crate::agent::mcp;

#[tauri::command]
pub async fn add_mcp_server(
    state: State<'_, Arc<AppState>>,
    config: McpServerConfig,
) -> AppResult<Vec<String>> {
    // 1. 保存配置到 DB
    let mut servers: Vec<McpServerConfig> = {
        let json: String = state.db.get_conn()?
            .query_row(
                "SELECT value FROM agent_settings WHERE key = 'mcp_servers'",
                [],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str(&json).unwrap_or_default()
    };
    servers.push(config.clone());
    let json = serde_json::to_string(&servers)
        .map_err(|e| AppError::Serialization(e.to_string()))?;
    state.db.get_conn()?.execute(
        "INSERT OR REPLACE INTO agent_settings (key, value) VALUES ('mcp_servers', ?1)",
        rusqlite::params![json],
    ).map_err(|e| AppError::Database(e.to_string()))?;

    // 2. 连接服务器
    let tool_names = mcp::connect_server(&state, &config).await?;
    Ok(tool_names)
}

#[tauri::command]
pub async fn remove_mcp_server(
    state: State<'_, Arc<AppState>>,
    name: String,
) -> AppResult<()> {
    // 1. 断开连接
    mcp::disconnect_server(&state, &name).await?;

    // 2. 从 DB 移除配置
    let mut servers: Vec<McpServerConfig> = {
        let json: String = state.db.get_conn()?
            .query_row(
                "SELECT value FROM agent_settings WHERE key = 'mcp_servers'",
                [],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str(&json).unwrap_or_default()
    };
    servers.retain(|s| s.name != name);
    let json = serde_json::to_string(&servers)
        .map_err(|e| AppError::Serialization(e.to_string()))?;
    state.db.get_conn()?.execute(
        "INSERT OR REPLACE INTO agent_settings (key, value) VALUES ('mcp_servers', ?1)",
        rusqlite::params![json],
    ).map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

#[tauri::command]
pub async fn list_mcp_servers(
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<McpServerStatus>> {
    let configs: Vec<McpServerConfig> = {
        let json: String = state.db.get_conn()?
            .query_row(
                "SELECT value FROM agent_settings WHERE key = 'mcp_servers'",
                [],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str(&json).unwrap_or_default()
    };
    let statuses = mcp::get_server_statuses(&state, &configs).await;
    Ok(statuses)
}
```

- [ ] **Step 2: 修改 agent_chat 以包含 MCP 工具**

在 `agent_chat` 命令中，构建工具列表的位置添加：

```rust
// 在 agent_chat 的工具列表构建后
let mut tools: Vec<Box<dyn yoagent::AgentTool>> = vec![
    // ... 内置工具 ...
];

// 追加 MCP 工具
let mcp_tools = mcp::get_mcp_tools(&state).await;
tools.extend(mcp_tools);
```

- [ ] **Step 3: 注册 commands**

```rust
agent::commands::add_mcp_server,
agent::commands::remove_mcp_server,
agent::commands::list_mcp_servers,
```

- [ ] **Step 4: 编译验证**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/agent/commands.rs src-tauri/src/lib.rs
git commit -m "feat: MCP Tauri commands - add/remove/list servers, inject MCP tools into agent_chat"
```

---

## Chunk 1: MCP 前端

### Task 3: McpServerForm 组件

**Files:**
- Create: `src/lib/components/settings/McpServerForm.svelte`

- [ ] **Step 1: 创建组件**

```svelte
<!-- src/lib/components/settings/McpServerForm.svelte -->
<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import * as Select from '$lib/components/ui/select';

  let {
    onSubmit,
    onCancel,
  }: {
    onSubmit: (config: { name: string; transport: 'stdio' | 'http'; commandOrUrl: string; args: string[] }) => void;
    onCancel: () => void;
  } = $props();

  let name = $state('');
  let transport = $state<'stdio' | 'http'>('stdio');
  let commandOrUrl = $state('');
  let argsStr = $state('');

  function submit() {
    if (!name || !commandOrUrl) return;
    onSubmit({
      name,
      transport,
      commandOrUrl,
      args: argsStr ? argsStr.split(' ').filter(Boolean) : [],
    });
  }
</script>

<div class="space-y-3 rounded-md border p-4">
  <div class="grid grid-cols-2 gap-3">
    <div>
      <label class="text-xs text-muted-foreground">名称</label>
      <Input class="h-8 text-xs mt-1" bind:value={name} placeholder="my-server" />
    </div>
    <div>
      <label class="text-xs text-muted-foreground">传输方式</label>
      <Select.Root type="single" bind:value={transport}>
        <Select.Trigger class="h-8 text-xs mt-1">{transport}</Select.Trigger>
        <Select.Content>
          <Select.Item value="stdio">stdio</Select.Item>
          <Select.Item value="http">HTTP</Select.Item>
        </Select.Content>
      </Select.Root>
    </div>
  </div>
  <div>
    <label class="text-xs text-muted-foreground">
      {transport === 'stdio' ? '命令' : 'URL'}
    </label>
    <Input
      class="h-8 text-xs mt-1"
      bind:value={commandOrUrl}
      placeholder={transport === 'stdio' ? 'npx -y @modelcontextprotocol/server-filesystem' : 'http://localhost:3000'}
    />
  </div>
  {#if transport === 'stdio'}
    <div>
      <label class="text-xs text-muted-foreground">参数（空格分隔）</label>
      <Input class="h-8 text-xs mt-1" bind:value={argsStr} placeholder="/path/to/dir" />
    </div>
  {/if}
  <div class="flex justify-end gap-2">
    <Button size="sm" variant="ghost" onclick={onCancel}>取消</Button>
    <Button size="sm" onclick={submit}>添加</Button>
  </div>
</div>
```

### Task 4: McpServerManager 组件

**Files:**
- Create: `src/lib/components/settings/McpServerManager.svelte`

- [ ] **Step 1: 创建组件**

```svelte
<!-- src/lib/components/settings/McpServerManager.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Button } from '$lib/components/ui/button';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import TrashIcon from '@lucide/svelte/icons/trash-2';
  import RadioTowerIcon from '@lucide/svelte/icons/radio-tower';
  import McpServerForm from './McpServerForm.svelte';

  interface McpServerStatus {
    config: { name: string; transport: string; commandOrUrl: string; args: string[] };
    connected: boolean;
  }

  let servers = $state<McpServerStatus[]>([]);
  let showForm = $state(false);
  let loading = $state(true);

  onMount(async () => {
    await refresh();
    loading = false;
  });

  async function refresh() {
    servers = await invoke<McpServerStatus[]>('list_mcp_servers').catch(() => []);
  }

  async function addServer(config: any) {
    await invoke('add_mcp_server', { config });
    showForm = false;
    await refresh();
  }

  async function removeServer(name: string) {
    await invoke('remove_mcp_server', { name });
    await refresh();
  }
</script>

<div class="space-y-3">
  <div class="flex items-center justify-between">
    <h4 class="text-xs font-medium text-muted-foreground">MCP 服务器</h4>
    <Button size="sm" variant="ghost" class="h-7" onclick={() => (showForm = true)}>
      <PlusIcon class="h-3.5 w-3.5 mr-1" />
      添加
    </Button>
  </div>

  {#if showForm}
    <McpServerForm onSubmit={addServer} onCancel={() => (showForm = false)} />
  {/if}

  {#if servers.length > 0}
    <div class="divide-y">
      {#each servers as server}
        <div class="flex items-center gap-3 py-2 text-xs">
          <RadioTowerIcon class="h-3.5 w-3.5 text-muted-foreground" />
          <span class="font-medium">{server.config.name}</span>
          <span class="text-muted-foreground">{server.config.transport}</span>
          <span
            class="ml-auto rounded-full px-2 py-0.5 text-[10px]"
            class:bg-green-100={server.connected}
            class:text-green-700={server.connected}
            class:bg-muted={!server.connected}
            class:text-muted-foreground={!server.connected}
          >
            {server.connected ? '已连接' : '未连接'}
          </span>
          <Button
            size="icon"
            variant="ghost"
            class="h-6 w-6"
            onclick={() => removeServer(server.config.name)}
          >
            <TrashIcon class="h-3 w-3" />
          </Button>
        </div>
      {/each}
    </div>
  {:else if !showForm}
    <p class="text-xs text-muted-foreground">未配置 MCP 服务器</p>
  {/if}
</div>
```

- [ ] **Step 2: 集成到 AgentSettings**

修改 `AgentSettings.svelte`，在 Skills 分区后添加：

```svelte
<script>
  import McpServerManager from './McpServerManager.svelte';
</script>

<!-- 在 Skills section 后添加 -->
<section>
  <h3 class="text-sm font-semibold mb-3">MCP 服务器</h3>
  <McpServerManager />
</section>
```

- [ ] **Step 3: 编译验证**

```bash
pnpm check 2>&1 | tail -10
```

- [ ] **Step 4: 提交**

```bash
git add src/lib/components/settings/McpServerForm.svelte src/lib/components/settings/McpServerManager.svelte src/lib/components/settings/AgentSettings.svelte
git commit -m "feat: MCP server management UI - add/remove/status display"
```

---

## Chunk 2: 端到端验收

### Task 5: MCP 端到端验收

- [ ] **Step 1: 启动应用**

```bash
pnpm tauri dev
```

- [ ] **Step 2: 验证 MCP 服务器添加**

1. 打开设置 → Agent → MCP 服务器
2. 点击 "添加"，填写：名称=filesystem，传输=stdio，命令=npx，参数=-y @modelcontextprotocol/server-filesystem /tmp
3. 点击添加，等待连接
4. 状态显示 "已连接"

- [ ] **Step 3: 验证 MCP 工具可用**

1. 返回对话页面
2. Agent ON 模式下发送 "使用 filesystem 工具列出 /tmp 目录"
3. 时间线中应显示 MCP 提供的工具调用
4. 授权弹窗（MCP 工具默认 ask）正确弹出

- [ ] **Step 4: 验证删除**

1. 设置 → Agent → MCP 服务器，删除 filesystem
2. 状态列表中不再显示该服务器

- [ ] **Step 5: 最终提交**

```bash
git add -A
git commit -m "feat: Phase 3 complete - MCP server integration"
```

---

## 验收标准

Phase 3 完成的标志：
- [ ] 设置页面可添加 MCP 服务器（stdio + HTTP）
- [ ] 连接成功后显示 "已连接" 状态
- [ ] MCP 工具在 Agent 模式下可被调用
- [ ] MCP 工具在工具授权列表中显示（默认 ask 等级）
- [ ] 删除服务器后正确断开连接并移除配置
- [ ] `cargo check` 无 error
- [ ] `pnpm check` 无 TypeScript 报错
