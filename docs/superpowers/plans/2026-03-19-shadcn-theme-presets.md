# Shadcn Theme Presets Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current primary-color-only theme switcher with the eight official `shadcn/ui` theme presets and apply the selected preset globally across the app.

**Architecture:** Introduce a small theme utility module for theme keys and DOM application, move full semantic token presets into `src/app.css` under `data-theme` selectors, and persist the selected theme in `settings.json` so startup and Settings use the same source of truth.

**Tech Stack:** SvelteKit, Svelte 5, Tauri plugin store, Tailwind v4 theme tokens, node:test

---

## Chunk 1: Theme Utilities and Contracts

### Task 1: Add failing frontend tests for theme utilities and CSS preset selectors

**Files:**
- Create: `src/lib/utils/theme.test.js`
- Create: `src/lib/themeCssContract.test.js`

- [ ] **Step 1: Write the failing tests**
- [ ] **Step 2: Run the tests to verify they fail because the theme module and CSS selectors do not exist yet**
- [ ] **Step 3: Add the minimal `src/lib/utils/theme.js` implementation**
- [ ] **Step 4: Add CSS selector coverage in `src/app.css` until the tests pass**
- [ ] **Step 5: Re-run the same tests and confirm green**

## Chunk 2: Startup and Settings Integration

### Task 2: Replace `colorIndex` settings flow with persisted `theme`

**Files:**
- Modify: `src/routes/+layout.svelte`
- Modify: `src/lib/components/settings/ProviderSettings.svelte`
- Reuse: `src/lib/utils/theme.js`

- [ ] **Step 1: Add a failing contract by wiring the new theme utility tests to require `theme` normalization and DOM application**
- [ ] **Step 2: Update `+layout.svelte` to load saved `theme` on mount and apply it**
- [ ] **Step 3: Replace `ProviderSettings.svelte` custom swatches and `colorIndex` persistence with official theme presets and `theme` persistence**
- [ ] **Step 4: Run targeted tests for theme utilities plus `pnpm check`**
- [ ] **Step 5: Refine the settings UI preview markup only if needed after green**

## Chunk 3: Verification

### Task 3: Verify the full frontend path

**Files:**
- Verify: `src/app.css`
- Verify: `src/routes/+layout.svelte`
- Verify: `src/lib/components/settings/ProviderSettings.svelte`
- Verify: `src/lib/utils/theme.js`
- Verify: `src/lib/utils/theme.test.js`
- Verify: `src/lib/themeCssContract.test.js`

- [ ] **Step 1: Run `node --test src/lib/utils/theme.test.js src/lib/themeCssContract.test.js`**
- [ ] **Step 2: Run `pnpm check`**
- [ ] **Step 3: Inspect `git diff --stat` to confirm scope stays limited to theme preset work**
