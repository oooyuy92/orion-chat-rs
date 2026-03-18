# Conversation Working Dirs Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the global `working_dir` setting with per-conversation `working_dirs` list, gate file tools when empty, and add a popover UI on the AgentToggle for managing directories.

**Architecture:** New `working_dirs` JSON column on `conversations` table. Backend `build_tools()` conditionally registers file tools based on the list. Frontend AgentToggle gains a hover popover with add/remove directory management.

**Tech Stack:** Rust/Tauri backend, SvelteKit frontend, SQLite, yoagent tools

---

## Chunk 1: Data Layer (Rust Backend)

### Task 1: DB Migration — Add `working_dirs` Column

**Files:**
- Modify: `src-tauri/src/db/migrations.rs:150-183`

- [ ] **Step 1: Add idempotent ALTER TABLE for working_dirs column**

In `migrations.rs`, after the `agent_mode` ALTER (line 150-152), add:

```rust
let _ = conn.execute(
    "ALTER TABLE conversations ADD COLUMN working_dirs TEXT NOT NULL DEFAULT '[]'",
    [],
);
```

- [ ] **Step 2: Remove the working_dir INSERT OR IGNORE**

Delete lines 180-183 (the `INSERT OR IGNORE INTO agent_settings ... VALUES ('working_dir', '')` block).

- [ ] **Step 3: Update test — remove working_dir assertion, add working_dirs assertion**

In `test_agent_schema_and_defaults_exist` (line 246):

Remove line 289:
```rust
assert_eq!(settings_value(&conn, "working_dir"), "");
```

After the `conversation_cols` assertions (after line 271), add:
```rust
assert!(conversation_cols.contains(&"working_dirs".to_string()));
```

- [ ] **Step 4: Run migration tests**

Run: `cd src-tauri && cargo test db::migrations::tests -- --nocapture`
Expected: All 3 tests pass (including idempotency test).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db/migrations.rs
git commit -m "feat: add working_dirs column to conversations table"
```

---

### Task 2: Conversation Model, DB Queries, and Commands — Add `working_dirs`

This task modifies model, DB layer, and commands together so the project compiles at each commit boundary.

**Files:**
- Modify: `src-tauri/src/models/conversation.rs`
- Modify: `src-tauri/src/db/conversations.rs`
- Modify: `src-tauri/src/commands/conversation.rs:118-138,506-577`

- [ ] **Step 1: Add `working_dirs` field to Conversation struct**

In `src-tauri/src/models/conversation.rs`:

```rust
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
    pub working_dirs: Vec<String>,
}
```

- [ ] **Step 2: Update `conversations.rs` — add `serde_json` usage and update `create()`**

In `src-tauri/src/db/conversations.rs`, the file already uses `rusqlite` and `crate::error`/`crate::models` imports. `serde_json` is available from the crate's `Cargo.toml` dependencies. Use it directly with `serde_json::` prefix (no new `use` needed since it's a crate-level dependency).

Replace the entire `create` function:

```rust
pub fn create(conn: &Connection, conv: &Conversation) -> AppResult<()> {
    let working_dirs_json = serde_json::to_string(&conv.working_dirs)
        .unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT INTO conversations (id, title, assistant_id, model_id, is_pinned, sort_order, created_at, updated_at, working_dirs)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            conv.id,
            conv.title,
            conv.assistant_id,
            conv.model_id,
            conv.is_pinned as i32,
            conv.sort_order,
            conv.created_at,
            conv.updated_at,
            working_dirs_json,
        ],
    )?;
    Ok(())
}
```

- [ ] **Step 3: Update `get()` — add working_dirs to SELECT**

Replace the `get` function:

```rust
pub fn get(conn: &Connection, id: &str) -> AppResult<Conversation> {
    conn.query_row(
        "SELECT id, title, assistant_id, model_id, is_pinned, sort_order, created_at, updated_at, working_dirs
         FROM conversations WHERE id = ?1",
        [id],
        |row| {
            let working_dirs_json: String = row.get(8)?;
            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                assistant_id: row.get(2)?,
                model_id: row.get(3)?,
                is_pinned: row.get::<_, i32>(4)? != 0,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                working_dirs: serde_json::from_str(&working_dirs_json).unwrap_or_default(),
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Conversation {id}"))
        }
        other => AppError::Database(other),
    })
}
```

- [ ] **Step 4: Update `list()` — add working_dirs to SELECT**

Replace the `list` function:

```rust
pub fn list(conn: &Connection) -> AppResult<Vec<Conversation>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, assistant_id, model_id, is_pinned, sort_order, created_at, updated_at, working_dirs
         FROM conversations ORDER BY is_pinned DESC, updated_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        let working_dirs_json: String = row.get(8)?;
        Ok(Conversation {
            id: row.get(0)?,
            title: row.get(1)?,
            assistant_id: row.get(2)?,
            model_id: row.get(3)?,
            is_pinned: row.get::<_, i32>(4)? != 0,
            sort_order: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
            working_dirs: serde_json::from_str(&working_dirs_json).unwrap_or_default(),
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}
```

- [ ] **Step 5: Add `set_working_dirs()` function**

After the `touch` function (line 132), add:

```rust
pub fn set_working_dirs(conn: &Connection, id: &str, dirs: &[String]) -> AppResult<()> {
    let json = serde_json::to_string(dirs).unwrap_or_else(|_| "[]".to_string());
    let changed = conn.execute(
        "UPDATE conversations SET working_dirs = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![json, id],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Conversation {id}")));
    }
    Ok(())
}
```

- [ ] **Step 6: Update `create_conversation` in commands — add working_dirs field**

In `src-tauri/src/commands/conversation.rs`, in `create_conversation` (line 126), add `working_dirs: vec![],` to the Conversation struct:

```rust
let conv = Conversation {
    id: uuid::Uuid::new_v4().to_string(),
    title,
    assistant_id,
    model_id,
    is_pinned: false,
    sort_order: 0,
    created_at: now.clone(),
    updated_at: now,
    working_dirs: vec![],
};
```

- [ ] **Step 7: Update `fork_conversation` — copy working_dirs from source**

In `fork_conversation` (line 520), add `working_dirs: source.working_dirs.clone(),` to the new_conv struct:

```rust
let new_conv = Conversation {
    id: uuid::Uuid::new_v4().to_string(),
    title: format!("{} 副本", source.title),
    assistant_id: source.assistant_id.clone(),
    model_id: source.model_id.clone(),
    is_pinned: false,
    sort_order: 0,
    created_at: now.clone(),
    updated_at: now.clone(),
    working_dirs: source.working_dirs.clone(),
};
```

- [ ] **Step 8: Fix all other Conversation struct initializations**

Search for all `Conversation {` in both `conversations.rs` tests and `conversation.rs` test helpers. Add `working_dirs: vec![],` after `updated_at` in every occurrence.

- [ ] **Step 9: Update conversations.rs tests and add new tests**

In all three existing tests (`test_conversation_crud`, `test_update_assistant_binding`, `test_update_model_binding`), add `working_dirs: vec![],` to every `Conversation { ... }`.

Add test for `set_working_dirs`:

```rust
#[test]
fn test_set_working_dirs() {
    let db = Database::new(":memory:").unwrap();

    db.with_conn(|conn| {
        let conv = Conversation {
            id: "conv-1".into(),
            title: "Hello".into(),
            assistant_id: None,
            model_id: None,
            is_pinned: false,
            sort_order: 0,
            created_at: "2025-01-01T00:00:00".into(),
            updated_at: "2025-01-01T00:00:00".into(),
            working_dirs: vec![],
        };
        create(conn, &conv)?;

        let fetched = get(conn, "conv-1")?;
        assert!(fetched.working_dirs.is_empty());

        set_working_dirs(conn, "conv-1", &["/tmp/project-a".into(), "/tmp/project-b".into()])?;
        let fetched = get(conn, "conv-1")?;
        assert_eq!(fetched.working_dirs, vec!["/tmp/project-a", "/tmp/project-b"]);

        // Verify list() also returns working_dirs correctly
        let all = list(conn)?;
        assert_eq!(all[0].working_dirs, vec!["/tmp/project-a", "/tmp/project-b"]);

        set_working_dirs(conn, "conv-1", &[])?;
        let fetched = get(conn, "conv-1")?;
        assert!(fetched.working_dirs.is_empty());

        Ok(())
    })
    .unwrap();
}
```

- [ ] **Step 10: Verify project compiles and all tests pass**

Run: `cd src-tauri && cargo test -- --nocapture`
Expected: All tests pass. Project compiles cleanly.

- [ ] **Step 11: Commit**

```bash
git add src-tauri/src/models/conversation.rs src-tauri/src/db/conversations.rs src-tauri/src/commands/conversation.rs
git commit -m "feat: add working_dirs to Conversation model, DB queries, and commands"
```

---

## Chunk 2: Agent Tool Building (Rust Backend)

### Task 3: Update `build_tools` and Agent Commands

**Files:**
- Modify: `src-tauri/src/agent/commands.rs`

- [ ] **Step 1: Remove `load_working_dir` function (lines 329-347)**

Delete the entire `load_working_dir` function.

- [ ] **Step 2: Update `agent_chat` — reuse existing conversation fetch for working_dirs**

The existing code at line 49-51 fetches the conversation and discards it:
```rust
app_state.db.with_conn(|conn| db::conversations::get(conn, &conversation_id).map(|_| ()))?;
```

Replace lines 49-55 (from the discard-fetch through `let working_dir = load_working_dir(...)`) with:

```rust
let conv = app_state
    .db
    .with_conn(|conn| db::conversations::get(conn, &conversation_id))?;
let working_dirs = conv.working_dirs;
```

This eliminates the double DB query — we fetch the conversation once and extract `working_dirs` from it.

- [ ] **Step 3: Update `agent_chat` — pass working_dirs to build_tools**

Replace lines 96-103 (the `build_tools` call):

```rust
tools: build_tools(
    app_state.clone(),
    &conversation_id,
    permissions,
    emit_event.clone(),
    &working_dirs,
)
.await?,
```

- [ ] **Step 4: Update `agent_chat` — conditionally append system prompt warning**

After the `let mut context = AgentContext { ... }` block (after line 104), add:

```rust
if working_dirs.is_empty() {
    context.system_prompt.push_str(
        "\n\n注意：当前未设置工作目录，文件操作功能不可用。请用户先设置工作目录。"
    );
}
```

- [ ] **Step 5: Rewrite `build_tools` — conditional file tool registration**

Replace the entire `build_tools` function:

```rust
async fn build_tools(
    state: Arc<AppState>,
    conversation_id: &str,
    permissions: ToolPermissions,
    emit_event: ChatEventEmitter,
    working_dirs: &[String],
) -> AppResult<Vec<Box<dyn yoagent::AgentTool>>> {
    let make_tool = |tool: Box<dyn yoagent::AgentTool>| {
        Box::new(PermissionedTool::new(
            tool,
            state.clone(),
            conversation_id.to_string(),
            permissions.clone(),
            emit_event.clone(),
        )) as Box<dyn yoagent::AgentTool>
    };

    let bash_cwd = if let Some(first) = working_dirs.first() {
        first.clone()
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/"))
            .display()
            .to_string()
    };

    let mut tools = vec![
        make_tool(Box::new(BashTool::new().with_cwd(bash_cwd))),
    ];

    if !working_dirs.is_empty() {
        let paths: Vec<String> = working_dirs.to_vec();
        tools.extend(vec![
            make_tool(Box::new(
                ReadFileTool::new().with_allowed_paths(paths.clone()),
            )),
            make_tool(Box::new(
                WriteFileTool::new().with_allowed_paths(paths.clone()),
            )),
            make_tool(Box::new(
                EditFileTool::new().with_allowed_paths(paths.clone()),
            )),
            make_tool(Box::new(
                ListFilesTool::new().with_root(working_dirs[0].clone()),
            )),
            make_tool(Box::new(
                SearchTool::new().with_root(working_dirs[0].clone()),
            )),
        ]);
    }

    let mcp_tools = mcp::get_mcp_tools(&state).await?;
    tools.extend(mcp_tools);

    Ok(tools)
}
```

- [ ] **Step 6: Add Tauri commands for get/set working dirs**

After the `set_skills_dir` function (line 227), add:

```rust
#[tauri::command]
pub async fn get_conversation_working_dirs(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
) -> AppResult<Vec<String>> {
    state
        .db
        .with_conn(|conn| {
            let conv = db::conversations::get(conn, &conversation_id)?;
            Ok(conv.working_dirs)
        })
}

#[tauri::command]
pub async fn set_conversation_working_dirs(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    dirs: Vec<String>,
) -> AppResult<()> {
    let dirs: Vec<String> = dirs
        .into_iter()
        .map(|d| d.trim_end_matches('/').to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    state
        .db
        .with_conn(|conn| db::conversations::set_working_dirs(conn, &conversation_id, &dirs))
}
```

- [ ] **Step 7: Register new commands in lib.rs**

In `src-tauri/src/lib.rs`, inside the `tauri::generate_handler![]` macro (after line 91, the `list_mcp_servers` line), add:

```rust
agent::commands::get_conversation_working_dirs,
agent::commands::set_conversation_working_dirs,
```

- [ ] **Step 8: Update existing tests for new `build_tools` signature**

In the tests section of `commands.rs`, update both `test_build_tools_scopes_*` tests. Replace the `build_tools` call pattern:

```rust
// Before:
//     &working_dir.display().to_string(),
// After:
//     &[working_dir.display().to_string()],
```

For `test_build_tools_scopes_read_file_to_working_dir` (line 672-678):
```rust
let tools = build_tools(
    state,
    "conv-1",
    ToolPermissions::with_defaults(),
    Arc::new(|_| {}),
    &[working_dir.display().to_string()],
)
.await
.unwrap();
```

Same pattern for `test_build_tools_scopes_list_files_to_working_dir` (line 707-713).

- [ ] **Step 9: Add test for empty working_dirs — no file tools**

Add a new test:

```rust
#[tokio::test]
async fn test_build_tools_empty_working_dirs_excludes_file_tools() {
    let state = Arc::new(AppState::new(":memory:", "/tmp").unwrap());

    let tools = build_tools(
        state,
        "conv-1",
        ToolPermissions::with_defaults(),
        Arc::new(|_| {}),
        &[],
    )
    .await
    .unwrap();

    let tool_names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(tool_names.contains(&"bash"), "bash should always be present");
    assert!(!tool_names.contains(&"read_file"), "read_file should not be present when working_dirs empty");
    assert!(!tool_names.contains(&"write_file"), "write_file should not be present when working_dirs empty");
    assert!(!tool_names.contains(&"edit_file"), "edit_file should not be present when working_dirs empty");
    assert!(!tool_names.contains(&"list_files"), "list_files should not be present when working_dirs empty");
    assert!(!tool_names.contains(&"search"), "search should not be present when working_dirs empty");
}
```

- [ ] **Step 10: Run all agent tests**

Run: `cd src-tauri && cargo test agent::commands::tests -- --nocapture`
Expected: All tests pass (old scoping tests + new empty test).

- [ ] **Step 11: Run full backend test suite**

Run: `cd src-tauri && cargo test -- --nocapture`
Expected: All tests pass.

- [ ] **Step 12: Commit**

```bash
git add src-tauri/src/agent/commands.rs src-tauri/src/lib.rs
git commit -m "feat: conditional file tools based on conversation working_dirs"
```

---

## Chunk 3: Frontend (Types, API, UI)

### Task 4: Frontend Types and API

**Files:**
- Modify: `src/lib/types/index.ts:85-94`
- Modify: `src/lib/utils/invoke.ts:30-58`
- Modify: `src/lib/api/web/impl.ts:66-94`

- [ ] **Step 1: Add `workingDirs` to Conversation type**

In `src/lib/types/index.ts`, update the Conversation interface (line 85-94):

```typescript
export interface Conversation {
  id: string;
  title: string;
  assistantId: string | null;
  modelId: string | null;
  isPinned: boolean;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
  workingDirs: string[];
}
```

- [ ] **Step 2: Add working dirs API to tauriApi**

In `src/lib/utils/invoke.ts`, after `forkConversation` (line 57-58), add:

```typescript
getConversationWorkingDirs(conversationId: string): Promise<string[]> {
  return invoke('get_conversation_working_dirs', { conversationId });
},
setConversationWorkingDirs(conversationId: string, dirs: string[]): Promise<void> {
  return invoke('set_conversation_working_dirs', { conversationId, dirs });
},
```

- [ ] **Step 3: Add matching stubs to webApi**

In `src/lib/api/web/impl.ts`, after `forkConversation` (line 92-94), add:

```typescript
getConversationWorkingDirs(conversationId: string): Promise<string[]> {
  return get(`/conversations/${conversationId}/working-dirs`);
},
setConversationWorkingDirs(conversationId: string, dirs: string[]): Promise<void> {
  return patch(`/conversations/${conversationId}/working-dirs`, { dirs });
},
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/types/index.ts src/lib/utils/invoke.ts src/lib/api/web/impl.ts
git commit -m "feat: add working dirs frontend types and API"
```

---

### Task 5: AgentToggle Popover UI

**Files:**
- Modify: `src/lib/components/chat/AgentToggle.svelte`

- [ ] **Step 1: Rewrite AgentToggle with popover**

Replace the entire file content:

```svelte
<script lang="ts">
  import BotIcon from '@lucide/svelte/icons/bot';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import XIcon from '@lucide/svelte/icons/x';
  import { agentMode } from '$lib/stores/agent';
  import { api } from '$lib/utils/invoke';

  let {
    disabled = false,
    conversationId = '',
  }: {
    disabled?: boolean;
    conversationId?: string;
  } = $props();

  let workingDirs = $state<string[]>([]);
  let showPopover = $state(false);
  let hoverTimeout = $state<ReturnType<typeof setTimeout> | null>(null);
  let containerEl = $state<HTMLDivElement>();

  $effect(() => {
    if (conversationId) {
      void loadDirs();
    }
  });

  async function loadDirs() {
    if (!conversationId) return;
    try {
      workingDirs = await api.getConversationWorkingDirs(conversationId);
    } catch {
      workingDirs = [];
    }
  }

  function toggle() {
    if (!disabled) {
      agentMode.update((value) => !value);
    }
  }

  async function addDir() {
    try {
      const dir = await api.pickDirectory();
      if (dir && !workingDirs.includes(dir)) {
        const updated = [...workingDirs, dir];
        await api.setConversationWorkingDirs(conversationId, updated);
        workingDirs = updated;
      }
    } catch (e) {
      console.error('Failed to add directory:', e);
    }
  }

  async function removeDir(index: number) {
    const updated = workingDirs.filter((_, i) => i !== index);
    try {
      await api.setConversationWorkingDirs(conversationId, updated);
      workingDirs = updated;
    } catch (e) {
      console.error('Failed to remove directory:', e);
    }
  }

  function handleMouseEnter() {
    if (hoverTimeout) clearTimeout(hoverTimeout);
    showPopover = true;
  }

  function handleMouseLeave() {
    hoverTimeout = setTimeout(() => {
      showPopover = false;
    }, 200);
  }

  function shortenPath(path: string): string {
    const home = path.replace(/^\/Users\/[^/]+/, '~');
    if (home.length <= 30) return home;
    const parts = home.split('/');
    if (parts.length <= 3) return home;
    return parts[0] + '/.../' + parts.slice(-2).join('/');
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="agent-wrapper"
  bind:this={containerEl}
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
>
  {#if showPopover}
    <div class="popover">
      {#if workingDirs.length === 0}
        <div class="popover-empty">未设置工作目录</div>
      {:else}
        {#each workingDirs as dir, i}
          <div class="popover-row" title={dir}>
            <span class="popover-path">{shortenPath(dir)}</span>
            <button class="popover-remove" onclick={() => void removeDir(i)}>
              <XIcon class="h-3 w-3" />
            </button>
          </div>
        {/each}
      {/if}
      <button class="popover-add" onclick={() => void addDir()}>
        <PlusIcon class="h-3 w-3" />
        <span>添加目录</span>
      </button>
    </div>
  {/if}

  <button
    type="button"
    class="agent-toggle"
    class:active={$agentMode}
    title={$agentMode ? 'Agent mode on' : 'Agent mode off'}
    onclick={toggle}
    {disabled}
  >
    <BotIcon class="h-3.5 w-3.5" />
    <span class="agent-label">
      Agent
      <span class="agent-badge">{$agentMode ? 'ON' : 'OFF'}</span>
    </span>
  </button>
</div>

<style>
  .agent-wrapper {
    position: relative;
    margin-left: auto;
  }

  .popover {
    position: absolute;
    bottom: calc(100% + 6px);
    right: 0;
    min-width: 220px;
    max-width: 320px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 4px 12px rgb(0 0 0 / 0.12);
    padding: 4px;
    z-index: 50;
  }

  .popover-empty {
    padding: 8px 10px;
    color: var(--muted-foreground);
    font-size: 11px;
    text-align: center;
  }

  .popover-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border-radius: 5px;
  }

  .popover-row:hover {
    background: var(--muted);
  }

  .popover-path {
    flex: 1;
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    color: var(--foreground);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .popover-remove {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border: none;
    background: none;
    color: var(--muted-foreground);
    cursor: pointer;
    border-radius: 3px;
    opacity: 0;
    transition: opacity 0.1s ease;
  }

  .popover-row:hover .popover-remove {
    opacity: 1;
  }

  .popover-remove:hover {
    color: var(--destructive);
    background: var(--muted);
  }

  .popover-add {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding: 6px 8px;
    border: none;
    background: none;
    color: var(--muted-foreground);
    font-size: 11px;
    cursor: pointer;
    border-radius: 5px;
    border-top: 1px solid var(--border);
    margin-top: 2px;
  }

  .popover-add:hover {
    background: var(--muted);
    color: var(--foreground);
  }

  .agent-toggle {
    display: flex;
    align-items: center;
    gap: 0;
    border-radius: 7px;
    padding: 4px 7px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid var(--border);
    background: var(--muted);
    color: var(--muted-foreground);
    white-space: nowrap;
    overflow: hidden;
    max-width: 27px;
    transition:
      max-width 0.22s ease,
      gap 0.18s ease,
      padding 0.18s ease,
      background 0.15s ease,
      color 0.15s ease,
      border-color 0.15s ease;
  }

  .agent-wrapper:hover .agent-toggle {
    max-width: 120px;
    gap: 5px;
    padding: 4px 10px;
  }

  .agent-toggle.active {
    background: var(--primary);
    color: var(--primary-foreground);
    border-color: var(--primary);
  }

  .agent-label {
    display: flex;
    align-items: center;
    gap: 4px;
    opacity: 0;
    width: 0;
    overflow: hidden;
    transition:
      opacity 0.15s ease 0.05s,
      width 0.22s ease;
  }

  .agent-wrapper:hover .agent-label {
    opacity: 1;
    width: auto;
  }

  .agent-badge {
    border-radius: 4px;
    padding: 0 5px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.04em;
  }

  .agent-toggle.active .agent-badge {
    background: rgb(255 255 255 / 0.16);
  }

  .agent-toggle:not(.active) .agent-badge {
    background: var(--border);
  }

  .agent-toggle:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/chat/AgentToggle.svelte
git commit -m "feat: AgentToggle popover for managing working directories"
```

---

### Task 6: Thread `conversationId` Through InputArea and ChatArea

**Files:**
- Modify: `src/lib/components/chat/InputArea.svelte:12-32,174`
- Modify: `src/lib/components/chat/ChatArea.svelte:176-186`

- [ ] **Step 1: Add `conversationId` prop to InputArea**

In `src/lib/components/chat/InputArea.svelte`, add `conversationId` to the props (after line 14, the `disabledReason` prop):

In the destructuring block (line 12-32), add `conversationId = '',` to the let block:

```typescript
let {
  disabled = false,
  disabledReason = '',
  conversationId = '',
  onSend,
  // ... rest unchanged
}: {
  disabled?: boolean;
  disabledReason?: string;
  conversationId?: string;
  onSend: (content: string) => void;
  // ... rest unchanged
} = $props();
```

- [ ] **Step 2: Pass `conversationId` to AgentToggle**

In `InputArea.svelte`, update line 174:

```svelte
<AgentToggle {disabled} {conversationId} />
```

- [ ] **Step 3: Pass `conversationId` from ChatArea to InputArea**

In `src/lib/components/chat/ChatArea.svelte`, update the `<InputArea>` block (lines 176-186):

```svelte
<InputArea
  {disabled}
  {disabledReason}
  {conversationId}
  {suggestions}
  {modelGroups}
  bind:selectedModelId
  {onModelSelect}
  onSend={handleSend}
  onGroupSend={handleGroupSend}
  onStop={handleStop}
/>
```

- [ ] **Step 4: Verify frontend builds**

Run: `npm run check` (or `pnpm check` / `bun check` depending on package manager)
Expected: No type errors.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/chat/InputArea.svelte src/lib/components/chat/ChatArea.svelte
git commit -m "feat: thread conversationId to AgentToggle for working dirs"
```

---

### Task 7: Final Verification

- [ ] **Step 1: Run full backend tests**

Run: `cd src-tauri && cargo test -- --nocapture`
Expected: All tests pass.

- [ ] **Step 2: Run frontend type checks**

Run: `npm run check`
Expected: No errors.

- [ ] **Step 3: Build the application**

Run: `npm run tauri build -- --debug` (or just `cargo build` in src-tauri)
Expected: Build succeeds.
