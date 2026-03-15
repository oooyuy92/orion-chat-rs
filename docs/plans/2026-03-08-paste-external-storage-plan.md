# Paste External Storage Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Move oversized pasted text out of message bodies into filesystem-backed blob storage while preserving searchability, export fidelity, editability, and model-context expansion.

**Architecture:** Add a dedicated paste-blob persistence layer in Rust, store large pasted text in app data files plus a `paste_blobs` table, and convert message content from embedded paste blocks to lightweight reference markers. Keep the frontend API ergonomic by sending structured paste attachments on create/edit flows, and keep runtime behavior unchanged by expanding references only when needed for view, search, export, or model requests.

**Tech Stack:** Rust, Tauri commands, SQLite/FTS via `rusqlite`, filesystem APIs, Svelte 5, TypeScript.

**Implementation Note:** The first implementation batch keeps the existing frontend inline-paste payload format and externalizes legacy `<<paste:...>>...<</paste>>` markers on the Rust side before persistence. This avoids widening the Tauri send/update payloads in the same batch while still moving stored oversized paste bodies out of the messages table.

---

### Task 1: Add DB schema and CRUD helpers for paste blobs

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/db/mod.rs`
- Create: `.worktrees/assistants-conversation-binding/src-tauri/src/db/paste_blobs.rs`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/db/messages.rs` (only if helper reuse is needed)
- Test: `.worktrees/assistants-conversation-binding/src-tauri/src/db/paste_blobs.rs`

**Step 1: Write the failing test**

Add DB-level tests covering:
- creating a paste blob record
- listing blobs by message
- deleting blobs for a message or conversation
- FTS search over paste blob content (or the chosen equivalent search path)

**Step 2: Run test to verify it fails**

Run: `cargo test paste_blobs --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: FAIL because the table/module/helpers do not exist yet.

**Step 3: Write minimal implementation**

- add the `paste_blobs` table migration/init SQL
- add a Rust model/row-mapping helper if needed
- add CRUD/query helpers in `db/paste_blobs.rs`
- add FTS support for paste content if that is the chosen search strategy

**Step 4: Run test to verify it passes**

Run: `cargo test paste_blobs --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: PASS.

**Step 5: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src-tauri/src/db/mod.rs .worktrees/assistants-conversation-binding/src-tauri/src/db/paste_blobs.rs
git commit -m "feat: add paste blob storage schema"
```

### Task 2: Add filesystem blob storage and marker conversion helpers

**Files:**
- Create: `.worktrees/assistants-conversation-binding/src-tauri/src/paste_storage.rs`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/lib.rs` (if wiring or exports are needed)
- Test: `.worktrees/assistants-conversation-binding/src-tauri/src/paste_storage.rs`

**Step 1: Write the failing test**

Add focused tests for:
- writing a blob text file
- reading it back by paste id / path
- replacing upload placeholders with persisted `paste-ref` markers
- expanding `paste-ref` markers back to full inline text
- preserving compatibility with the legacy inline `<<paste:...>>...<</paste>>` format

**Step 2: Run test to verify it fails**

Run: `cargo test paste_storage --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: FAIL because the storage module and helper functions do not exist yet.

**Step 3: Write minimal implementation**

Implement helpers for:
- generating stable paste ids
- writing UTF-8 files under the app data paste directory
- reading blob text by id/path
- converting `<<paste-upload:CLIENT_TOKEN:LENGTH>>` into `<<paste-ref:PASTE_ID:LENGTH>>`
- expanding both new and legacy paste markers into plain text content for downstream consumers

**Step 4: Run test to verify it passes**

Run: `cargo test paste_storage --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: PASS.

**Step 5: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src-tauri/src/paste_storage.rs .worktrees/assistants-conversation-binding/src-tauri/src/lib.rs
git commit -m "feat: add paste blob filesystem helpers"
```

### Task 3: Update send/edit message write paths to persist paste blobs

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/commands/chat.rs`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/commands/conversation.rs`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/models/message.rs` or another shared model file for request payloads
- Modify: `.worktrees/assistants-conversation-binding/src/lib/utils/invoke.ts`
- Modify: `.worktrees/assistants-conversation-binding/src/lib/types/index.ts`
- Modify: `.worktrees/assistants-conversation-binding/src/lib/components/chat/InputArea.svelte`
- Modify: `.worktrees/assistants-conversation-binding/src/lib/components/chat/MessageBubble.svelte`

**Step 1: Write the failing test**

Add backend tests for:
- `send_message` persists paste blobs and stores only `paste-ref` markers in the message body
- edit/resend flows preserve blob semantics instead of inlining large text again
- legacy inline messages remain accepted where applicable

**Step 2: Run test to verify it fails**

Run: `cargo test send_message --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: FAIL because commands do not yet accept structured paste attachments.

**Step 3: Write minimal implementation**

- define a request type for paste attachments, e.g. `PasteUpload`
- update frontend send/edit callers to submit attachment arrays plus upload markers in message content
- update Rust commands to persist blobs before message creation/update
- make message bodies store only `paste-ref` markers for new oversized pastes

**Step 4: Run test to verify it passes**

Run: `cargo test send_message --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: PASS.

**Step 5: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src-tauri/src/commands/chat.rs .worktrees/assistants-conversation-binding/src-tauri/src/commands/conversation.rs .worktrees/assistants-conversation-binding/src-tauri/src/models/message.rs .worktrees/assistants-conversation-binding/src/lib/utils/invoke.ts .worktrees/assistants-conversation-binding/src/lib/types/index.ts .worktrees/assistants-conversation-binding/src/lib/components/chat/InputArea.svelte .worktrees/assistants-conversation-binding/src/lib/components/chat/MessageBubble.svelte
git commit -m "feat: persist oversized paste content externally"
```

### Task 4: Add on-demand paste viewing and edit-time expansion

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/commands/conversation.rs`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/lib.rs`
- Modify: `.worktrees/assistants-conversation-binding/src/lib/utils/invoke.ts`
- Modify: `.worktrees/assistants-conversation-binding/src/lib/components/chat/MessageBubble.svelte`
- Modify: `.worktrees/assistants-conversation-binding/src/lib/components/chat/InputArea.svelte` (if shared parsing helpers are extracted)

**Step 1: Write the failing test**

Add command/helper tests for:
- fetching paste blob content by `paste_id`
- expanding `paste-ref` markers back to editable inline content
- ensuring the UI-facing content reconstruction preserves marker order and length labels

**Step 2: Run test to verify it fails**

Run: `cargo test conversation --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: FAIL for the new blob-fetch or expansion behavior.

**Step 3: Write minimal implementation**

- add a command such as `get_paste_blob_content(paste_id)` if needed
- update `MessageBubble` click-to-view flow so `paste-ref` markers load file content lazily
- update edit flows to expand `paste-ref` markers back into editable paste blocks only when entering edit mode

**Step 4: Run test to verify it passes**

Run: `cargo test conversation --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: PASS.

**Step 5: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src-tauri/src/commands/conversation.rs .worktrees/assistants-conversation-binding/src-tauri/src/lib.rs .worktrees/assistants-conversation-binding/src/lib/utils/invoke.ts .worktrees/assistants-conversation-binding/src/lib/components/chat/MessageBubble.svelte .worktrees/assistants-conversation-binding/src/lib/components/chat/InputArea.svelte
git commit -m "feat: load external paste content on demand"
```

### Task 5: Extend search, export, and context expansion to cover external pastes

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/commands/search.rs`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/commands/export.rs`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/commands/chat.rs`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/db/paste_blobs.rs`
- Test: `.worktrees/assistants-conversation-binding/src-tauri/src/commands/search.rs`
- Test: `.worktrees/assistants-conversation-binding/src-tauri/src/commands/export.rs`
- Test: `.worktrees/assistants-conversation-binding/src-tauri/src/commands/chat.rs`

**Step 1: Write the failing tests**

Add tests for:
- searching a term that exists only inside external paste content returns the parent message
- Markdown/JSON export expands external paste content back into full text
- chat request building / auto-compress expands `paste-ref` markers before sending content to the model
- legacy inline paste markers still work in all three paths

**Step 2: Run test to verify it fails**

Run: `cargo test search --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Run: `cargo test export --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Run: `cargo test chat --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: FAIL because paste refs are not yet expanded/indexed in these flows.

**Step 3: Write minimal implementation**

- add paste-aware search merging and deduplication
- make export reconstruct full paste text for both legacy and external formats
- refactor request building so model-facing content always uses expanded paste text

**Step 4: Run test to verify it passes**

Run: `cargo test search --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Run: `cargo test export --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Run: `cargo test chat --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: PASS.

**Step 5: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src-tauri/src/commands/search.rs .worktrees/assistants-conversation-binding/src-tauri/src/commands/export.rs .worktrees/assistants-conversation-binding/src-tauri/src/commands/chat.rs .worktrees/assistants-conversation-binding/src-tauri/src/db/paste_blobs.rs
git commit -m "feat: support external pastes in search export and model context"
```

### Task 6: Add cleanup hooks for conversation deletion and final verification

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/db/conversations.rs`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/commands/conversation.rs`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/commands/settings.rs` (only if existing cache cleanup helpers are reused)
- Modify: `docs/plans/2026-03-08-paste-external-storage-design.md` (only if implementation deviates)
- Modify: `docs/plans/2026-03-08-paste-external-storage-plan.md` (only if implementation deviates)

**Step 1: Write the failing test**

Add tests covering:
- deleting a conversation removes associated paste blob records
- corresponding paste files are removed or marked for cleanup according to the chosen policy
- soft-delete / restore behavior remains coherent with blob retention rules

**Step 2: Run test to verify it fails**

Run: `cargo test conversations --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: FAIL until cleanup logic is wired.

**Step 3: Write minimal implementation**

- hook conversation deletion into paste blob cleanup
- preserve blob files where necessary for restore scenarios, per the chosen policy
- keep the implementation conservative; do not add a full orphan GC feature in this batch unless required by tests

**Step 4: Run full verification**

Run: `cargo test --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: PASS.

Run: `pnpm run check`
Expected: `svelte-check found 0 errors and 0 warnings`.

**Step 5: Manual regression checklist**

Verify manually:
- sending oversized paste creates a tag-like message segment instead of embedding raw text in the stored message
- clicking a paste tag loads and shows full content
- editing a message with external paste still works
- searching a term inside paste content finds the parent message
- exported Markdown/JSON contains the full paste text
- auto-compress and normal chat continue to include paste content in model context
- legacy inline paste messages still render and behave correctly

**Step 6: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src-tauri/src/db/conversations.rs .worktrees/assistants-conversation-binding/src-tauri/src/commands/conversation.rs docs/plans/2026-03-08-paste-external-storage-design.md docs/plans/2026-03-08-paste-external-storage-plan.md
git commit -m "docs: add paste external storage plan"
```
