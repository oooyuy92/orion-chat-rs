# Manual Model Entry Design

**Date:** 2026-03-13

## Background

The current "Model Service" tab relies entirely on model synchronization. A provider must fetch models from the remote API before any model can appear in chat, assistant settings, model combos, or other model pickers.

This breaks down for OpenAI-compatible providers and custom gateways where:

- the upstream model list endpoint is incomplete or unsupported
- the user already knows the request model name they want to send
- the user wants a localized or descriptive label in the UI that differs from the raw API model name

The requested behavior is to support manual model creation from the provider settings UI through a modal dialog with separate fields for:

- request model name
- display name / remark
- enabled state

The actual chat request must use the request model name, while the UI should display the user-defined display name when available.

## Goals

- Allow users to add models without relying on "Sync Models".
- Keep one unified model source so chat, assistants, combos, auto-rename, auto-compress, and version views all continue to work.
- Separate internal model identity from API request model name.
- Preserve synchronized models while introducing manually managed models.

## Non-Goals

- Editing synchronized models in place.
- Letting users manually configure advanced capability metadata such as context length or vision support in v1.
- Redesigning the model picker UX outside the minimal changes needed to surface manual models.

## Current State

The current implementation uses one `models` table and exposes models through `api.listProviders()`.

- Settings UI reads provider models from `ProviderConfig.models`.
- Chat model selection is built from enabled providers and enabled models returned by `api.listProviders()`.
- Assistant settings and combo selection use the same provider-backed model list.
- The backend currently treats `model_id` as both:
  - the internal identifier stored in the database
  - the request model name sent to the provider API

That last point is the core abstraction problem. Manual entry requires these to be split.

## Options Considered

### Option 1: Reuse `models` table with clarified field semantics

Keep one model table, but redefine the fields clearly:

- `id`: internal stable identifier
- `name`: request model name sent to provider API
- `display_name`: UI display name / remark
- `source`: synchronized or manual

Pros:

- Minimal disruption to the rest of the app because all existing selectors already depend on one model source.
- Assistant bindings, conversation bindings, version metadata, and combos continue to reference one `model.id`.
- Sync and manual creation can coexist in one list.

Cons:

- Requires a database migration and backend send-path fix.

### Option 2: Separate user-defined model table

Create a second table for manual models and merge it with synced models in the frontend.

Pros:

- Strong conceptual separation between remote and manual sources.

Cons:

- Every consumer would need dual-source merging logic.
- More complicated for assistant bindings, combo storage, name resolution, and regressions.

### Option 3: Keep sync-only behavior and add local aliases

Continue requiring sync, but let users attach aliases after sync.

Pros:

- Smaller UI change.

Cons:

- Does not solve the primary problem because users still cannot send to unsynced models.

## Decision

Choose Option 1.

We will keep a unified `models` table and make model records first-class entities whether they come from synchronization or manual creation.

## Data Model

### Model identity

`models.id` becomes an internal identifier only. It must no longer be assumed to equal the provider request model name.

### Request model name

`models.name` stores the exact string sent to the provider API as the `model` field.

Examples:

- `gpt-4.1`
- `openai/gpt-4.1`
- `deepseek-chat`

### Display name / remark

`models.display_name` stores the user-facing display name.

Rules:

- For synchronized models, default to the provider-returned model name.
- For manual models, use the user-entered remark if provided.
- If empty, UI falls back to `name`, then `id`.

### Source

Add `models.source TEXT NOT NULL DEFAULT 'synced'`.

Allowed values:

- `synced`
- `manual`

This is used to:

- distinguish sync-managed vs user-managed records
- prevent sync from overwriting or deleting manual models
- show a "manual" badge in the settings UI
- enable edit/delete actions only for manual models in v1

## Backend Behavior

### Model lookup

All send flows continue to accept a frontend-selected internal `model_id`.

Before issuing a provider request, the backend must resolve:

- provider instance
- provider type
- request model name from `models.name`

The provider request payload must use `models.name`, not the internal `models.id`.

### Sync behavior

`fetch_models` remains responsible only for synchronized models.

For synced rows:

- upsert by a deterministic internal ID scheme derived from provider and raw model name, or by preserving existing IDs for existing synced rows
- set `name` to the real provider model name
- set `display_name` to the provider model name
- set `source` to `synced`

Manual rows must not be deleted or overwritten by sync.

### Manual model creation

Add backend commands to create, update, and delete manual model records under a provider.

Create:

- generates a new UUID internal ID
- stores provider ID
- stores request model name in `name`
- stores optional remark in `display_name`
- stores `source = 'manual'`
- stores enabled state

Update and delete should be limited to manual rows in v1.

## Frontend Behavior

### Settings UI

The provider detail panel gets a new `Add Model` action in the models toolbar.

Clicking it opens a modal dialog scoped to the currently selected provider with fields:

- request model name, required
- display name / remark, optional
- enabled, default true

After save:

- the new model appears immediately in the current provider's model list
- no sync is required

### Model list rendering

Model cards should render:

- primary label: `display_name || name || id`
- secondary label: request model name when different from the primary label
- source badge for manual models

### Other model consumers

All existing model selection UIs should continue reading from the unified provider model list:

- chat model selector
- assistant default model selector
- model combo picker
- auto-rename model selector
- auto-compress model selector
- version compare model name resolution

All display logic should use the same fallback rule:

`display_name || name || id`

## Migration Plan

The database migration must be idempotent for existing installations.

Add:

- `display_name` backfill if missing or empty
- `source TEXT NOT NULL DEFAULT 'synced'`

Existing synced rows should remain valid after migration. Their current `name` values continue to act as request model names, and `display_name` should be backfilled from `name` where necessary.

## Error Handling

- Creating a manual model requires a non-empty request model name.
- Provider settings should reject manual creation if no provider is selected.
- Backend should return a clear `NotFound` or validation error if a manual-model edit/delete targets a missing row.
- Send-path resolution should fail clearly if the selected model row exists but its provider does not.

## Testing Strategy

### Backend

- migration tests for the new `source` column and `display_name` backfill
- provider command tests for manual model CRUD
- sync tests proving manual rows survive `fetch_models`
- chat command tests proving provider requests use `models.name` rather than internal `models.id`

### Frontend

- settings UI tests for opening the add-model modal and rendering a new manual model
- display fallback tests for `display_name -> name -> id`
- regression coverage for assistant settings, model selector, combo picker, and version compare name resolution

## Open Constraint Chosen for V1

Synchronized models remain sync-managed and only support enable/disable selection. Manual models support add/edit/delete. This keeps ownership boundaries simple and avoids mixing user-managed labels into sync refresh behavior.
