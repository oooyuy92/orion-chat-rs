# Chatbot Shadcn Style Refresh Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rebuild the Svelte chat frontend visual and interaction structure to match shadcn chatbot style while preserving current backend behavior.

**Architecture:** Keep existing route/state/event flow in `+page.svelte`, then recompose chat UI into shadcn-like layout surfaces through targeted component rewrites (`MessageList`, `MessageBubble`, `InputArea`, sidebar controls, and shared CSS tokens). No backend or protocol changes.

**Tech Stack:** SvelteKit 2, Svelte 5 runes, Tailwind CSS v4, TypeScript.

---

### Task 1: Establish light design tokens and default light theme

**Files:**
- Modify: `src/app.css`
- Modify: `src/lib/stores/ui.svelte.ts`
- Modify: `src/routes/+layout.svelte`

**Step 1: Write failing test/check expectation**
- Define desired baseline: app should boot in light mode and expose neutral token variables.

**Step 2: Verify RED**
- Run: `pnpm check`
- Expected before edits: pass/fail independent of visual goal; manual check still shows dark default.

**Step 3: Minimal implementation**
- Replace color variables with neutral light palette.
- Set store default theme to `light`.
- Keep layout mount behavior aligned with light theme.

**Step 4: Verify GREEN**
- Run: `pnpm check`
- Confirm no type errors and light mode defaults correctly.

### Task 2: Rebuild page-level chat layout surfaces

**Files:**
- Modify: `src/routes/+page.svelte`

**Step 1: Write failing test/check expectation**
- Define desired structure: sidebar retained, top bar + conversation viewport + composer with suggestion chips.

**Step 2: Verify RED**
- Manual inspection shows old structure (legacy header/input surface).

**Step 3: Minimal implementation**
- Recompose markup and class structure.
- Add suggestion list and pass to composer.
- Keep existing state/event logic and API calls.

**Step 4: Verify GREEN**
- Run: `pnpm check`
- Manual chat flow still works (select/send/streaming).

### Task 3: Rebuild conversation viewport and message presentation

**Files:**
- Modify: `src/lib/components/chat/MessageList.svelte`
- Modify: `src/lib/components/chat/MessageBubble.svelte`

**Step 1: Write failing test/check expectation**
- Desired behavior: centered readable column, shadcn-like user/assistant presentation, reasoning panel hierarchy.

**Step 2: Verify RED**
- Existing bubbles and spacing differ from target.

**Step 3: Minimal implementation**
- Rewrite message container and bubble class structure.
- Keep markdown rendering and reasoning toggle behavior.
- Keep token metadata and streaming visibility states.

**Step 4: Verify GREEN**
- Run: `pnpm check`
- Manual verification of message appearance and reasoning toggle.

### Task 4: Rebuild prompt composer interactions

**Files:**
- Modify: `src/lib/components/chat/InputArea.svelte`
- Modify: `src/lib/components/chat/ModelSelector.svelte`

**Step 1: Write failing test/check expectation**
- Desired behavior: suggestion chips, textarea + tool row composer, compact submit action, model selector in control row.

**Step 2: Verify RED**
- Existing composer lacks target structure.

**Step 3: Minimal implementation**
- Add `suggestions` and optional `selectedModelLabel` props.
- Build chip row and control row interactions.
- Keep enter/send behavior and resize logic.

**Step 4: Verify GREEN**
- Run: `pnpm check`
- Manual verify chip click, enter/send, disabled states.

### Task 5: Refresh sidebar and settings visual consistency

**Files:**
- Modify: `src/lib/components/sidebar/ConversationList.svelte`
- Modify: `src/lib/components/settings/ProviderSettings.svelte`

**Step 1: Write failing test/check expectation**
- Sidebar/settings should visually align with new neutral light token system.

**Step 2: Verify RED**
- Existing accent-heavy style differs from target.

**Step 3: Minimal implementation**
- Update button/list/card styles to muted surfaces.
- Keep existing behavior unchanged.

**Step 4: Verify GREEN**
- Run: `pnpm check`
- Manual verify list and settings usability.

### Task 6: Final verification and cleanup

**Files:**
- Review: `src/routes/+page.svelte`
- Review: `src/lib/components/chat/*.svelte`
- Review: `src/lib/components/sidebar/*.svelte`
- Review: `src/app.css`

**Step 1: Run verification command**
- `pnpm check`

**Step 2: Manual QA pass**
- Create/select/delete conversation.
- Send message, confirm streaming updates.
- Toggle reasoning block.
- Validate suggestions and composer actions.
- Validate narrow viewport layout.

**Step 3: Document residual risks**
- Note any behavior not fully covered by automated tests.
