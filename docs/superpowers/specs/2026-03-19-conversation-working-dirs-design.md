# Conversation Working Dirs Design

Per-conversation working directory list that gates agent file tool access.

## Problem

`load_working_dir()` falls back to `std::env::current_dir()` (the Tauri process cwd, often `/` on macOS) when no working_dir is configured. File tools operate in an unpredictable directory. There is no way to set working directories per conversation, and no way to set them from the chat UI.

## Solution

Add a `working_dirs` JSON array field to the `conversations` table. When the list is empty, file operation tools are not registered and the system prompt warns the agent. The AgentToggle button gains an upward popover for adding/removing directories per conversation.

## Data Layer

### DB Migration

Add column to `conversations`:

```sql
ALTER TABLE conversations ADD COLUMN working_dirs TEXT NOT NULL DEFAULT '[]';
```

Remove the `INSERT OR IGNORE` for `working_dir` in `agent_settings` (line 180-183 of migrations.rs). The old row in existing databases is left as-is (harmless dead data); no DELETE needed.

### Migration Ordering

The existing `INSERT OR IGNORE INTO agent_settings ... VALUES ('working_dir', '')` at line 180-183 must be removed entirely. The `ALTER TABLE` for `working_dirs` is added as idempotent migration (using `let _ = conn.execute(...)` pattern, same as other ALTER statements).

### Rust Model

`Conversation` struct gains:

```rust
pub working_dirs: Vec<String>,
```

### JSON Serialization in SQLite

The `working_dirs` field is stored as a JSON TEXT column. Since existing `conversations.rs` uses only direct column-to-field mapping, this field requires explicit serde at the query boundary:

- **Read**: `serde_json::from_str::<Vec<String>>(&row.get::<_, String>(idx)?).unwrap_or_default()`
- **Write**: `serde_json::to_string(&conv.working_dirs)?`

All SELECT/INSERT queries in `conversations.rs` updated to include `working_dirs` as the new column.

### New DB Function

```rust
pub fn set_working_dirs(conn: &Connection, id: &str, dirs: &[String]) -> AppResult<()>
```

Reading working dirs is done via the existing `get()` function since `working_dirs` is now on the `Conversation` struct.

### Tauri Commands

```rust
#[tauri::command]
pub async fn get_conversation_working_dirs(state, conversation_id) -> AppResult<Vec<String>>

#[tauri::command]
pub async fn set_conversation_working_dirs(state, conversation_id, dirs: Vec<String>) -> AppResult<()>
```

Both must be registered in `src-tauri/src/lib.rs` `invoke_handler` (the `tauri::generate_handler![]` macro).

### Frontend API

```typescript
// src/lib/utils/invoke.ts (tauriApi object, alongside existing conversation APIs)
getConversationWorkingDirs(conversationId: string): Promise<string[]>
setConversationWorkingDirs(conversationId: string, dirs: string[]): Promise<void>
```

Placed in `invoke.ts` (not `agent.ts`) because `working_dirs` is a conversation property. The `src/lib/api/web/impl.ts` web API needs matching stub implementations for Docker/web mode.

### Frontend Type

```typescript
// src/lib/types/index.ts
export interface Conversation {
  // ... existing fields
  workingDirs: string[];
}
```

### Path Validation on Add

When adding a directory via `set_conversation_working_dirs`:
- Deduplicate paths (no duplicates allowed)
- Normalize trailing slashes (strip trailing `/`)
- No existence validation (directory may be on a removable drive; nonexistent dirs are a known limitation)

## Backend Tool Building

### `build_tools()` Signature Change

Full current signature at `commands.rs:349`:

```rust
// Before
async fn build_tools(
    state: Arc<AppState>,
    conversation_id: &str,
    permissions: ToolPermissions,
    emit_event: ChatEventEmitter,
    working_dir: &str,
) -> AppResult<Vec<Box<dyn yoagent::AgentTool>>>

// After: working_dir: &str → working_dirs: &[String]
async fn build_tools(
    state: Arc<AppState>,
    conversation_id: &str,
    permissions: ToolPermissions,
    emit_event: ChatEventEmitter,
    working_dirs: &[String],
) -> AppResult<Vec<Box<dyn yoagent::AgentTool>>>
```

### Empty List Behavior

When `working_dirs` is empty:

- Do NOT register file tools: read_file, write_file, edit_file, list_files, search
- Bash cwd defaults to user home directory via `dirs::home_dir()` (behavioral change from current `std::env::current_dir()` fallback, which returns the Tauri process cwd — typically `/` on macOS)
- Append to system prompt: `"注意：当前未设置工作目录，文件操作功能不可用。请用户先设置工作目录。"`

### Non-empty List Behavior

When `working_dirs` has entries:

- All file tools receive the full list as allowed paths
- Bash cwd set to the first path in the list
- `ReadFileTool::new().with_allowed_paths(working_dirs.to_vec())`
- `WriteFileTool::new().with_allowed_paths(working_dirs.to_vec())`
- `EditFileTool::new().with_allowed_paths(working_dirs.to_vec())`
- `ListFilesTool::new().with_root(working_dirs[0].clone())`
- `SearchTool::new().with_root(working_dirs[0].clone())`

### `load_working_dir()` Removal

Remove `load_working_dir(state: &AppState)` function entirely. In `agent_chat()`, replace:

```rust
// Before
let working_dir = load_working_dir(&app_state)?;
// After
let working_dirs = app_state.db.with_conn(|conn| {
    let conv = db::conversations::get(conn, &conversation_id)?;
    Ok(conv.working_dirs)
})?;
```

## AgentToggle Hover UI

### Popover Layout

```
┌──────────────────────────┐
│ /Users/xxx/project-a   ✕ │
│ /Users/xxx/project-b   ✕ │
│ ＋ 添加目录              │
└──────────────────────────┘
     [Bot] Agent ON
```

### Behavior

- Hover on AgentToggle opens an upward popover
- Each row: truncated path + delete button (✕)
- Bottom row: "＋ 添加目录" button → calls `pickDirectory()` system dialog
- Empty state: shows "未设置工作目录" placeholder text
- Add/remove immediately persists via `setConversationWorkingDirs`
- Popover closes on mouse leave or click outside

### Component Changes

`AgentToggle.svelte`:

- New prop: `conversationId: string`
- Local state: `workingDirs: string[]`, `showPopover: boolean`
- `onMount`: load working dirs for current conversation
- Reactivity: reload when `conversationId` changes

`InputArea.svelte`:

- New prop: `conversationId: string`
- Pass `conversationId` to `AgentToggle`

`ChatArea.svelte`:

- Pass `conversationId` to `InputArea` (ChatArea already holds `conversationId`)

### No Global Store Needed

- `working_dirs` is local UI state in AgentToggle
- Backend reads from DB on each `agent_chat` call independently

Note: This differs from `agent_mode` which uses a global client-side store. The difference is intentional: `working_dirs` is per-conversation persistent state that must survive app restarts, while `agent_mode` is a UI toggle.

## Conversation Lifecycle

### New Conversation

New conversations start with `working_dirs: []` (empty). File tools are NOT available until the user adds a directory via the popover. This is intentional — the current fallback to process cwd is the bug being fixed.

### Fork Conversation

`fork_conversation` in `src-tauri/src/commands/conversation.rs` must copy `source.working_dirs` to the new conversation (alongside the existing copy of `assistant_id` and `model_id`).

### Delete Conversation

`working_dirs` is a column on the `conversations` row — deleted automatically with the conversation. No additional cleanup needed.

## Test Updates

The following tests need updating:

- `migrations.rs::test_agent_schema_and_defaults_exist` — remove assertion for `settings_value(&conn, "working_dir")`; add assertion that `working_dirs` column exists on conversations
- `commands.rs::test_build_tools_scopes_read_file_to_working_dir` — update to pass `&[String]` instead of `&str`
- `commands.rs::test_build_tools_scopes_list_files_to_working_dir` — same
- `conversations.rs::test_conversation_crud` — update `Conversation` struct initialization to include `working_dirs: vec![]`; add test for `set_working_dirs`

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/db/migrations.rs` | ALTER TABLE add working_dirs column; remove working_dir INSERT |
| `src-tauri/src/models/conversation.rs` | Add working_dirs field |
| `src-tauri/src/db/conversations.rs` | Update all queries for working_dirs; add set_working_dirs; JSON serde |
| `src-tauri/src/agent/commands.rs` | Remove load_working_dir; update build_tools signature and logic; add Tauri commands |
| `src-tauri/src/commands/conversation.rs` | Update create_conversation and fork_conversation for working_dirs |
| `src-tauri/src/lib.rs` | Register new Tauri commands in invoke_handler |
| `src/lib/types/index.ts` | Add workingDirs to Conversation |
| `src/lib/utils/invoke.ts` | Add get/set working dirs API functions to tauriApi |
| `src/lib/api/web/impl.ts` | Add matching stub implementations for web mode |
| `src/lib/components/chat/AgentToggle.svelte` | Add popover UI, conversationId prop |
| `src/lib/components/chat/InputArea.svelte` | Add conversationId prop, pass to AgentToggle |
| `src/lib/components/chat/ChatArea.svelte` | Pass conversationId to InputArea |
