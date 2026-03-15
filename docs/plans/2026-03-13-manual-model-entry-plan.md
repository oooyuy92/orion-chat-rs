# Manual Model Entry Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add manual model creation and management in the provider settings UI so users can send requests with a user-defined request model name and display name without depending on model sync.

**Architecture:** Keep a single unified model source in the existing `models` table. Split internal model identity from request model name by keeping `models.id` as the stable local identifier, `models.name` as the provider request model name, `models.display_name` as the UI label, and `models.source` as the ownership marker (`synced` or `manual`). Update the backend send path to resolve request metadata from the model row before calling a provider, then update the Svelte settings UI and all name-rendering sites to use the display-label fallback consistently.

**Tech Stack:** Rust + Tauri commands + SQLite (`rusqlite`), Svelte 5, TypeScript, `node:test`, `svelte-check`

---

### Task 1: Add Backend Model Metadata and Migration Coverage

**Files:**
- Modify: `src-tauri/src/models/provider.rs`
- Modify: `src-tauri/src/db/migrations.rs`
- Modify: `src-tauri/src/commands/provider.rs`
- Test: `src-tauri/src/commands/provider.rs`
- Test: `src-tauri/src/db/mod.rs` or existing migration-related Rust tests if needed

**Step 1: Write the failing Rust tests**

Add tests covering:

- `models.source` is added by migration with default `synced`
- `load_models_for_provider` returns `display_name` when present and falls back to `name` when absent
- manual-model rows preserve `source = 'manual'`

Suggested assertions to add in `src-tauri/src/commands/provider.rs`:

```rust
assert_eq!(model.name, "Friendly GPT");
assert_eq!(model.request_name, "gpt-4.1");
assert_eq!(model.source, "manual");
```

If you introduce an enum instead of a raw string, assert against the enum value instead.

**Step 2: Run test to verify it fails**

Run:

```bash
cargo test commands::provider::tests::load_models_for_provider_prefers_display_name -- --exact
```

Expected: FAIL because `ModelInfo` does not yet carry enough fields and the SQL loader does not read `display_name` or `source`.

**Step 3: Write minimal implementation**

Implement the smallest backend surface needed:

- extend `ModelInfo` with:
  - `display_name: Option<String>` or `String`
  - `request_name: String` if you want to stop overloading `name`
  - `source: String` or enum
- update migration logic in `src-tauri/src/db/migrations.rs` to add:
  - `display_name` backfill safety if missing data exists
  - `source TEXT NOT NULL DEFAULT 'synced'`
- update `load_models_for_provider` SQL to read the new fields and expose a display fallback

Prefer a clear semantic split even if that means renaming the frontend-facing TypeScript field later.

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test commands::provider::tests::load_models_for_provider_prefers_display_name -- --exact
```

Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/src/models/provider.rs src-tauri/src/db/migrations.rs src-tauri/src/commands/provider.rs
git commit -m "refactor: split model metadata from request name"
```

### Task 2: Add Manual Model CRUD on the Backend

**Files:**
- Modify: `src-tauri/src/commands/provider.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/utils/invoke.ts`
- Modify: `src/lib/api/web/impl.ts`
- Modify: `src/lib/types/index.ts`
- Test: `src-tauri/src/commands/provider.rs`

**Step 1: Write the failing Rust tests**

Add provider-command tests for manual model create, update, and delete using an in-memory SQLite database.

Cover:

- create inserts `source = manual`
- create stores request model name separately from display name
- update only succeeds for manual rows
- delete only removes the targeted manual row

Minimal test shape:

```rust
let created = create_manual_model_in_db(&conn, "p1", "gpt-4.1", Some("生产模型"), true)?;
assert_eq!(created.request_name, "gpt-4.1");
assert_eq!(created.display_name.as_deref(), Some("生产模型"));
assert_eq!(created.source, "manual");
```

**Step 2: Run test to verify it fails**

Run:

```bash
cargo test commands::provider::tests::create_manual_model_persists_request_name_and_source -- --exact
```

Expected: FAIL because the helper/command does not exist yet.

**Step 3: Write minimal implementation**

Add Tauri commands and supporting helpers for:

- `create_manual_model(provider_id, request_name, display_name, enabled)`
- `update_manual_model(model_id, request_name, display_name, enabled)`
- `delete_manual_model(model_id)`

Update TypeScript invoke/web shims and shared types to expose the new command contracts.

For v1, reject update/delete when `source != manual`.

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test commands::provider::tests::create_manual_model_persists_request_name_and_source -- --exact
```

Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/src/commands/provider.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src/lib/utils/invoke.ts src/lib/api/web/impl.ts src/lib/types/index.ts
git commit -m "feat: add manual model CRUD commands"
```

### Task 3: Fix the Send Path to Use the Stored Request Model Name

**Files:**
- Modify: `src-tauri/src/commands/chat.rs`
- Test: `src-tauri/src/commands/chat.rs`

**Step 1: Write the failing Rust tests**

Add a focused unit test around a new helper that resolves outbound model request metadata from the database.

Test behavior:

- internal model ID `local-uuid-1` resolves to provider `p1`
- outbound request model name becomes stored request name such as `gpt-4.1`
- provider request must not use the internal ID

Suggested test target:

```rust
let resolved = resolve_model_request(&state, "local-uuid-1").await?;
assert_eq!(resolved.request_model, "gpt-4.1");
assert_ne!(resolved.request_model, "local-uuid-1");
```

**Step 2: Run test to verify it fails**

Run:

```bash
cargo test commands::chat::tests::resolve_model_request_uses_stored_request_name -- --exact
```

Expected: FAIL because `run_stream` and group-send still send `model_id` directly.

**Step 3: Write minimal implementation**

Refactor `src-tauri/src/commands/chat.rs` so that:

- provider resolution returns both provider data and request model name
- `ChatRequest.model` uses the resolved request model name
- message rows continue storing the internal `model_id`
- single-send, resend, regenerate, generate-version, compress, and group-send all use the same resolution path

Keep the UI contract unchanged: the frontend still passes internal `model_id`.

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test commands::chat::tests::resolve_model_request_uses_stored_request_name -- --exact
```

Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/src/commands/chat.rs
git commit -m "fix: send provider requests with stored model name"
```

### Task 4: Add a Shared Frontend Model Label Helper

**Files:**
- Create: `src/lib/utils/modelDisplay.ts`
- Create: `src/lib/utils/modelDisplay.test.js`
- Modify: `src/lib/types/index.ts`

**Step 1: Write the failing frontend test**

Create a `node:test` file for the display fallback rules:

- prefer display name / remark
- fall back to request name
- fall back to internal ID

Example:

```js
assert.equal(resolveModelLabel({ id: 'm1', requestName: 'gpt-4.1', displayName: '生产模型' }), '生产模型');
assert.equal(resolveModelLabel({ id: 'm1', requestName: 'gpt-4.1', displayName: '' }), 'gpt-4.1');
assert.equal(resolveModelLabel({ id: 'm1', requestName: '', displayName: '' }), 'm1');
```

**Step 2: Run test to verify it fails**

Run:

```bash
node --test src/lib/utils/modelDisplay.test.js
```

Expected: FAIL because the helper file does not exist yet.

**Step 3: Write minimal implementation**

Create a small shared helper with functions such as:

- `resolveModelLabel(model)`
- `resolveModelSecondaryLabel(model)`
- `isManualModel(model)`

Update `src/lib/types/index.ts` to align the TypeScript `ModelInfo` shape with the backend metadata added earlier.

**Step 4: Run test to verify it passes**

Run:

```bash
node --test src/lib/utils/modelDisplay.test.js
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/lib/utils/modelDisplay.ts src/lib/utils/modelDisplay.test.js src/lib/types/index.ts
git commit -m "test: add shared model display helpers"
```

### Task 5: Add Manual Model Modal and Provider Settings Management

**Files:**
- Modify: `src/lib/components/settings/ProviderSettings.svelte`
- Possibly modify: `src/lib/components/ui/dialog/index.ts` only if import ergonomics require it
- Test: `src/lib/components/settings/providerSettingsManualModelContract.test.js`

**Step 1: Write the failing frontend contract test**

Create a `node:test` source-contract test that asserts the settings page now includes:

- an add-model action in the models toolbar
- dialog state for manual model creation
- request-model and display-name fields
- manual-model badge or source-aware rendering

Pattern-based tests are acceptable in this codebase if they focus on behavior-critical structure.

**Step 2: Run test to verify it fails**

Run:

```bash
node --test src/lib/components/settings/providerSettingsManualModelContract.test.js
```

Expected: FAIL because the dialog and add-model wiring do not exist yet.

**Step 3: Write minimal implementation**

In `ProviderSettings.svelte`:

- add dialog state for create/edit manual model
- add `Add Model` button beside sync/select-all actions
- render a modal form with:
  - request model name
  - display name / remark
  - enabled switch
- call the new backend commands
- optimistically update the selected provider's model list
- render source badge and dual-line labels for manual models
- keep synchronized models read-only except enable/disable and sync

Do not expand scope into capability editing in v1.

**Step 4: Run test to verify it passes**

Run:

```bash
node --test src/lib/components/settings/providerSettingsManualModelContract.test.js
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/lib/components/settings/ProviderSettings.svelte src/lib/components/settings/providerSettingsManualModelContract.test.js
git commit -m "feat: add manual model management to provider settings"
```

### Task 6: Update All Model Pickers and Name Renderers to Use the Shared Label Rules

**Files:**
- Modify: `src/lib/components/chat/ModelSelector.svelte`
- Modify: `src/lib/components/settings/AssistantSettings.svelte`
- Modify: `src/lib/components/chat/ComboSelector.svelte`
- Modify: `src/lib/components/chat/VersionCompareView.svelte`
- Modify: `src/routes/+page.svelte` if model-group normalization needs metadata passthrough
- Test: `src/lib/components/chat/modelDisplayContract.test.js`

**Step 1: Write the failing frontend contract test**

Add a small `node:test` file that verifies the relevant components import or use the shared model-display helper, or at minimum render `display_name`-aware fallback logic instead of raw `model.name`.

Also cover that manual models can flow through enabled model groups.

**Step 2: Run test to verify it fails**

Run:

```bash
node --test src/lib/components/chat/modelDisplayContract.test.js
```

Expected: FAIL because current components still use raw `model.name` or `model.id`.

**Step 3: Write minimal implementation**

Update all model-label render sites so they use the shared helper consistently:

- chat selector trigger and items
- assistant default model options
- combo picker resolution
- version compare header labels
- any `resolveModelName` helper in route or component code

Make sure enabled manual models appear anywhere enabled synced models already appear.

**Step 4: Run test to verify it passes**

Run:

```bash
node --test src/lib/components/chat/modelDisplayContract.test.js
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/lib/components/chat/ModelSelector.svelte src/lib/components/settings/AssistantSettings.svelte src/lib/components/chat/ComboSelector.svelte src/lib/components/chat/VersionCompareView.svelte src/routes/+page.svelte src/lib/components/chat/modelDisplayContract.test.js
git commit -m "refactor: unify model display labels across pickers"
```

### Task 7: Full Verification and Cleanup

**Files:**
- Modify: any touched files from earlier tasks only if verification reveals defects

**Step 1: Run targeted Rust tests**

Run:

```bash
cargo test commands::provider::tests:: -- --nocapture
cargo test commands::chat::tests:: -- --nocapture
```

Expected: PASS for the new provider and chat coverage.

**Step 2: Run targeted frontend contract tests**

Run:

```bash
node --test src/lib/utils/modelDisplay.test.js
node --test src/lib/components/settings/providerSettingsManualModelContract.test.js
node --test src/lib/components/chat/modelDisplayContract.test.js
```

Expected: PASS

**Step 3: Run whole-project validation**

Run:

```bash
cargo test -- --nocapture
pnpm check
```

Expected:

- Rust tests pass
- `pnpm check` completes without TypeScript or Svelte errors

**Step 4: Fix any failures and re-run**

Only change files implicated by the failed verification.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: support manual model entry and display aliases"
```
