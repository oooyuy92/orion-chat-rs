# Streaming Plaintext Preview Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Stop parsing Markdown and syntax highlighting during assistant streaming, render streaming assistant content as plain text with preserved line breaks, and switch back to full Markdown rendering only after the message finishes.

**Architecture:** Keep the change entirely in the frontend rendering layer. Detect streaming assistant messages in `MessageBubble.svelte`, branch to a plain-text rendering path for both `content` and `reasoning`, and preserve the existing `renderMarkdown(...)` path for completed/error assistant messages. Do not change backend event payloads or message persistence.

**Tech Stack:** Svelte 5, TypeScript, `marked`, `highlight.js`.

---

### Task 1: Add a rendering branch for streaming assistant messages

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/src/lib/components/chat/MessageBubble.svelte`

**Step 1: Write the failing behavior target**

Before editing logic, identify the exact rendering conditions to enforce:
- assistant + `streaming` => plain text preview
- assistant + `done`/`error` => Markdown rendering
- user messages stay unchanged
- reasoning follows the same streaming vs final split

**Step 2: Run frontend check to establish the current baseline**

Run: `pnpm run check`
Expected: PASS before changes, confirming a clean baseline.

**Step 3: Write minimal implementation**

In `MessageBubble.svelte`:
- add a derived flag such as `isStreamingAssistant`
- replace unconditional assistant Markdown derivations with conditional logic
- for streaming assistant messages, return raw text instead of HTML
- keep the existing Markdown path for completed assistant messages

**Step 4: Run frontend check to verify it passes**

Run: `pnpm run check`
Expected: PASS.

**Step 5: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src/lib/components/chat/MessageBubble.svelte
git commit -m "perf: skip markdown during assistant streaming"
```

### Task 2: Style the streaming preview to preserve readability

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/src/lib/components/chat/MessageBubble.svelte`

**Step 1: Write the failing UI checklist**

Define the styling expectations:
- line breaks are preserved during streaming
- long lines wrap instead of overflowing
- visual tone stays close to the final message style
- reasoning preview remains readable when expanded during streaming

**Step 2: Run frontend check to keep the baseline clean**

Run: `pnpm run check`
Expected: PASS.

**Step 3: Write minimal implementation**

Add lightweight classes such as:
- `.message-plain`
- `.reasoning-plain`

Ensure they use:
- `white-space: pre-wrap`
- safe word wrapping
- compatible font/color sizing with existing message body styles

**Step 4: Run frontend check to verify it passes**

Run: `pnpm run check`
Expected: PASS.

**Step 5: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src/lib/components/chat/MessageBubble.svelte
git commit -m "style: add plain-text streaming preview styles"
```

### Task 3: Regress reasoning and final-render transitions

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/src/lib/components/chat/MessageBubble.svelte`
- Modify: `docs/plans/2026-03-08-streaming-markdown-design.md` (only if implementation deviates)

**Step 1: Write the manual regression checklist**

Validate these interactions manually after implementation:
- streaming assistant response appears as plain text
- streaming reasoning appears as plain text when expanded
- final completed response switches to Markdown + syntax highlighting
- copy, edit, regenerate, and version actions still work

**Step 2: Run frontend static verification**

Run: `pnpm run check`
Expected: `svelte-check found 0 errors and 0 warnings`.

**Step 3: Perform manual verification**

Use at least one long code-oriented prompt and confirm:
- no Markdown formatting during streaming
- code fences render only after completion
- scrolling stays responsive while chunks arrive

**Step 4: Commit**

```bash
git add .worktrees/assistants-conversation-binding/src/lib/components/chat/MessageBubble.svelte docs/plans/2026-03-08-streaming-markdown-design.md
git commit -m "docs: capture streaming markdown optimization behavior"
```

### Task 4: Final verification

**Files:**
- Modify: `docs/plans/2026-03-08-streaming-markdown-plan.md` (only if implementation deviates)

**Step 1: Run frontend static checks**

Run: `pnpm run check`
Expected: PASS with 0 errors and 0 warnings.

**Step 2: Re-run Rust tests only if touched indirectly**

Run: `cargo test --manifest-path .worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
Expected: PASS if any shared types or contracts changed; otherwise optional.

**Step 3: Final manual regression pass**

Confirm:
- assistant streaming is plain text
- assistant completed response is Markdown
- reasoning follows the same rule
- existing assistant prompt / auto-compress / pagination behavior still feels normal

**Step 4: Commit**

```bash
git add docs/plans/2026-03-08-streaming-markdown-plan.md
git commit -m "docs: add streaming markdown optimization plan"
```
