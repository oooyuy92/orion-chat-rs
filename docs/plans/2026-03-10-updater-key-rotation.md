# Updater Key Rotation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rotate the Tauri updater signing key, update repository configuration and GitHub secrets, then publish a new release with valid updater metadata.

**Architecture:** Generate a new encrypted minisign keypair locally, keep the private key out of git, replace the committed updater public key in Tauri config, update release operations docs, set GitHub repository secrets for the new private key and password, and publish a new version tag so GitHub Actions rebuilds release artifacts including `latest.json`.

**Tech Stack:** Tauri v2, GitHub Actions, GitHub Releases, `gh` CLI, `pnpm tauri signer`, JSON config.

---

### Task 1: Rotate updater key material

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/.local-secrets/orion-updater.key`
- Modify: `.worktrees/assistants-conversation-binding/.local-secrets/orion-updater.key.pub`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/tauri.conf.json`

**Step 1: Generate a fresh encrypted updater keypair**
Run: `pnpm tauri signer generate -w .local-secrets/orion-updater.key -p '<new-password>' -f`
Expected: new private and public key files written.

**Step 2: Update committed updater public key**
Run: write `.local-secrets/orion-updater.key.pub` into `src-tauri/tauri.conf.json` `plugins.updater.pubkey`.
Expected: config contains the new public key and still has `bundle.createUpdaterArtifacts = true`.

**Step 3: Verify key rotation locally**
Run: inspect new files and config.
Expected: public key changed from previous value.

### Task 2: Update release operations and GitHub secrets

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/docs/plans/2026-03-08-auto-updater-ops.md`

**Step 1: Update ops doc with rotation note**
Run: add note that the old key was rotated on 2026-03-10 and both GitHub secrets must be updated together.
Expected: docs reflect new operational state.

**Step 2: Set GitHub secrets**
Run: `gh secret set TAURI_SIGNING_PRIVATE_KEY` and `gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
Expected: repository stores the new private key and password.

### Task 3: Release a new version

**Files:**
- Modify: `.worktrees/assistants-conversation-binding/package.json`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/Cargo.toml`
- Modify: `.worktrees/assistants-conversation-binding/src-tauri/tauri.conf.json`

**Step 1: Bump version to v0.3.2**
Run: update all three version sources consistently.
Expected: app version reads `0.3.2` everywhere.

**Step 2: Commit and push**
Run: commit only relevant files and push branch.
Expected: remote branch contains key rotation and version bump.

**Step 3: Create and push tag**
Run: `git tag v0.3.2 && git push origin v0.3.2`
Expected: release workflow starts.

### Task 4: Verify release artifacts and updater metadata

**Files:**
- None

**Step 1: Monitor GitHub Actions**
Run: `gh run watch` / `gh run view`
Expected: release workflow succeeds.

**Step 2: Verify release assets**
Run: `gh release view v0.3.2 --json assets`
Expected: installers and `latest.json` are present.

**Step 3: Verify updater metadata URL**
Run: `curl -L https://github.com/oooyuy92/orion-chat-rs/releases/latest/download/latest.json`
Expected: valid JSON is returned.
