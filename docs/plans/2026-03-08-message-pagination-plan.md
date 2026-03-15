# Message Pagination Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add conversation message pagination so Orion Chat initially loads only the latest 100 messages, auto-loads older messages when the user scrolls to the top, and preserves viewport position while prepending older pages.

**Architecture:** Keep the change incremental. Add a paginated message query in the Rust DB/command layer, expose it through the Tauri invoke client, and update the Svelte page plus `MessageList` to manage page state and scroll anchoring. Do not add virtualization in this batch; keep message order and existing business rules unchanged.

**Tech Stack:** Rust, Tauri commands, SQLite via `rusqlite`, Svelte 5, TypeScript.

---

### Task 1: Add paginated message query in the DB layer

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/db/messages.rs`
- Test: `.worktrees/assistants-conversation-binding/src-tauri/src/db/messages.rs`

**Step 1: Write the failing test**

Add DB-level tests covering:
- latest-page query returns only the newest `limit` visible messages
- before-anchor query returns messages older than `before_message_id`
- `has_more` is `true` only when older visible messages still exist
- deleted or inactive-version messages are excluded from both the page and `has_more`

**Step 2: Run test to verify it fails**

Run: `cargo test messages::tests::test_list_by_conversation_page --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: FAIL because the paginated query function and/or assertions do not exist yet.

**Step 3: Write minimal implementation**

In `db/messages.rs`:
- keep existing `list_by_conversation(...)` unchanged for compatibility until later tasks switch callers
- add a new paginated query helper, e.g. `list_page_by_conversation(conn, conversation_id, limit, before_message_id)`
- return a small result struct containing `messages` and `has_more`
- use `created_at ASC, rowid ASC` ordering in the final returned page
- compute `has_more` from actual older visible rows, not just page size

**Step 4: Run test to verify it passes**

Run: `cargo test messages::tests::test_list_by_conversation_page --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: PASS.

**Step 5: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src-tauri/src/db/messages.rs
git commit -m "test: add paginated message query coverage"
```

### Task 2: Expose paginated messages through the command layer

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/commands/conversation.rs`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/models/message.rs` or another shared response-model file if a new response struct is needed
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/src/lib.rs`
- Test: `.worktrees/assistants-conversation-binding/src-tauri/src/commands/conversation.rs`

**Step 1: Write the failing test**

Add a command-level test that creates a conversation with more than one page of messages and asserts:
- `get_messages` with no anchor returns the latest page and `has_more = true`
- `get_messages` with `before_message_id` returns the older page
- returned message order remains oldest-to-newest inside each page

**Step 2: Run test to verify it fails**

Run: `cargo test commands::conversation::tests::test_get_messages_page --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: FAIL because command signature / response shape does not match the new behavior yet.

**Step 3: Write minimal implementation**

- introduce a serializable response type such as `PagedMessages { messages: Vec<Message>, has_more: bool }`
- extend `get_messages` to accept `limit: Option<u32>` and `before_message_id: Option<String>`
- wire the command to the new DB pagination helper
- register any new types or imports needed so Tauri serialization remains intact

**Step 4: Run test to verify it passes**

Run: `cargo test commands::conversation::tests::test_get_messages_page --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: PASS.

**Step 5: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src-tauri/src/commands/conversation.rs .worktrees/assistants-conversation-binding/src-tauri/src/models/message.rs .worktrees/assistants-conversation-binding/src-tauri/src/lib.rs
git commit -m "feat: expose paginated message command"
```

### Task 3: Update the frontend invoke client and types

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/src/lib/types/index.ts`
- Modify: `.worktrees/assistants-conversation-binding/src/lib/utils/invoke.ts`

**Step 1: Write the failing type-level usage**

Change the frontend API surface so `api.getMessages(...)` is expected to return a paged result rather than a bare `Message[]`.

**Step 2: Run frontend check to verify it fails**

Run: `pnpm run check`
Expected: FAIL with TypeScript/Svelte errors at old call sites that still assume `Message[]`.

**Step 3: Write minimal implementation**

- add a shared TS type such as `PagedMessages`
- update `api.getMessages(conversationId, options?)` to pass `limit` and `beforeMessageId`
- keep argument names aligned with Tauri command parameters

**Step 4: Run frontend check to verify it still fails only at remaining callers**

Run: `pnpm run check`
Expected: FAIL only in page/components that still need migration.

**Step 5: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src/lib/types/index.ts .worktrees/assistants-conversation-binding/src/lib/utils/invoke.ts
git commit -m "refactor: add paged messages API types"
```

### Task 4: Migrate page-level message loading to latest-page semantics

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/src/routes/+page.svelte`

**Step 1: Write the failing behavior-oriented checklist**

Document the expected page-level behavior in code comments or a temporary checklist while implementing:
- switch conversation loads latest 100 messages only
- message state stores `hasMoreMessages`, `isLoadingMoreMessages`, and `oldestMessageId`
- fallback refreshes reload only the latest page, not the entire conversation
- auto-compress resets message pagination state using the compression result

**Step 2: Run frontend check to verify current callers fail after API changes**

Run: `pnpm run check`
Expected: FAIL at `+page.svelte` until all `api.getMessages(...)` call sites are migrated.

**Step 3: Write minimal implementation**

In `src/routes/+page.svelte`:
- add `pageSize = 100`, `hasMoreMessages`, `isLoadingMoreMessages`
- create a single helper like `loadLatestMessages(conversationId)` that sets `messages` and `hasMoreMessages`
- replace direct `api.getMessages(activeConversationId)` calls with that helper or a paged equivalent
- ensure auto-compress success path resets pagination state without a full reload
- block top-load behavior while streaming or compressing

**Step 4: Run frontend check to verify it passes page-level typing**

Run: `pnpm run check`
Expected: PASS for `+page.svelte`, or remaining failures only in `MessageList` integration before Task 5.

**Step 5: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src/routes/+page.svelte
git commit -m "feat: load latest message page in chat view"
```

### Task 5: Add auto-load-older behavior and scroll anchoring in MessageList

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/src/lib/components/chat/MessageList.svelte`
- Modify: `.worktrees/assistants-conversation-binding/src/lib/components/chat/ChatArea.svelte`
- Modify: `.worktrees/assistants-conversation-binding/src/routes/+page.svelte`

**Step 1: Write the failing UI integration**

Plumb a new callback prop such as `onLoadOlder` from `+page.svelte` through `ChatArea.svelte` into `MessageList.svelte`, and add the state/props needed for:
- `hasMoreMessages`
- `isLoadingMoreMessages`
- current streaming/compressing disable conditions

**Step 2: Run frontend check to verify it fails**

Run: `pnpm run check`
Expected: FAIL until the new props and handlers are wired consistently.

**Step 3: Write minimal implementation**

In `MessageList.svelte`:
- listen to the scroll container
- when `scrollTop <= threshold` and loading is allowed, call `onLoadOlder()` once
- before requesting older messages, record `scrollHeight` and `scrollTop`
- after DOM updates with prepended messages, restore `scrollTop` using the scroll-height delta
- preserve existing “empty state” and bottom-scroll behavior for initial load / new outgoing messages

In `+page.svelte`:
- implement `loadOlderMessages()` using `beforeMessageId = messages[0]?.id`
- prepend returned messages to the existing array
- update `hasMoreMessages`
- prevent duplicate loads

**Step 4: Run frontend check to verify it passes**

Run: `pnpm run check`
Expected: PASS.

**Step 5: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src/lib/components/chat/MessageList.svelte .worktrees/assistants-conversation-binding/src/lib/components/chat/ChatArea.svelte .worktrees/assistants-conversation-binding/src/routes/+page.svelte
git commit -m "feat: auto-load older messages on scroll top"
```

### Task 6: Re-run focused and full verification

**Files:**
- Modify: `docs/plans/2026-03-08-message-pagination-design.md` (only if implementation deviates)
- Modify: `docs/plans/2026-03-08-message-pagination-plan.md` (only if implementation deviates)

**Step 1: Run focused Rust pagination tests**

Run: `cargo test test_list_by_conversation_page --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: PASS.

**Step 2: Run full Rust test suite**

Run: `cargo test --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: PASS with no new failures.

**Step 3: Run frontend static checks**

Run: `pnpm run check`
Expected: `svelte-check found 0 errors and 0 warnings`.

**Step 4: Manual regression checklist**

Verify manually:
- opening a long conversation loads only the latest page
- scrolling to the top loads older messages automatically
- viewport position remains stable after prepending
- sending a new message still keeps the chat at the bottom
- streaming and auto-compress do not trigger top-loading
- auto-compress resets the page state cleanly
- assistant-bound conversations still preserve request order as `assistant prompt → summary → follow-up`

**Step 5: Commit**

```bash
git add docs/plans/2026-03-08-message-pagination-design.md docs/plans/2026-03-08-message-pagination-plan.md
git commit -m "docs: add message pagination design and plan"
```
