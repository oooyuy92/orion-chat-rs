# Model Selector Zoom Clipping Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Keep the model selector popup fully visible after page zoom by making the composer dropdown open upward with explicit viewport padding.

**Architecture:** `ModelSelector` already uses the shared dropdown-menu portal wrapper, so the fix belongs at the floating-layer configuration point. A source-level contract test will lock the placement props in place and prevent regressions.

**Tech Stack:** Svelte 5, Bits UI dropdown menu primitives, `node:test`

---

### Task 1: Lock the intended placement in a failing test

**Files:**
- Modify: `src/lib/components/chat/modelDisplayContract.test.js`
- Test: `src/lib/components/chat/modelDisplayContract.test.js`

**Step 1: Write the failing test**

Add a test that asserts `ModelSelector.svelte` passes explicit placement props on `DropdownMenuContent`, specifically upward placement and viewport collision padding.

**Step 2: Run test to verify it fails**

Run: `node --test src/lib/components/chat/modelDisplayContract.test.js`
Expected: FAIL because the dropdown content does not yet declare the placement props.

### Task 2: Implement the minimal dropdown placement fix

**Files:**
- Modify: `src/lib/components/chat/ModelSelector.svelte`
- Test: `src/lib/components/chat/modelDisplayContract.test.js`

**Step 1: Write minimal implementation**

Update `DropdownMenuContent` in `ModelSelector.svelte` to pass `side="top"` and a small `collisionPadding`, keeping the rest of the selector behavior unchanged.

**Step 2: Run test to verify it passes**

Run: `node --test src/lib/components/chat/modelDisplayContract.test.js`
Expected: PASS

### Task 3: Verify no broader regressions

**Files:**
- Modify: none
- Test: project checks

**Step 1: Run broader validation**

Run: `pnpm run check`
Expected: existing checks remain green.
