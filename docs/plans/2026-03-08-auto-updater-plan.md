# Auto Updater Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 Orion Chat 接入基于 GitHub Releases 的 Tauri 自动更新能力，支持真实检查更新、后台下载、下载完成后提示重启安装。

**Architecture:** 使用 `tauri-plugin-updater` 作为更新引擎；Rust 侧提供轻薄命令封装，前端设置页驱动更新状态机；GitHub Actions release workflow 负责生成和上传 updater 元数据与安装包；版本号统一由应用配置和 tag 对齐。

**Tech Stack:** Tauri v2、Rust、Svelte 5、TypeScript、GitHub Actions、GitHub Releases。

---

### Task 1: 固化更新配置契约并统一版本号

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`
- Create: `src/lib/components/settings/autoUpdaterContract.test.js`

**Step 1: Write the failing test**

写最小契约测试，覆盖：
- `Cargo.toml` 包版本为 `0.2.0`
- `tauri.conf.json` 版本为 `0.2.0`
- 后续若更新版本，这个测试能阻止 tag/release 与 app version 继续脱节

**Step 2: Run test to verify it fails**

Run: `node --test src/lib/components/settings/autoUpdaterContract.test.js`
Expected: FAIL，当前版本仍为 `0.1.0`。

**Step 3: Write minimal implementation**

把：
- `src-tauri/Cargo.toml` 的 `version`
- `src-tauri/tauri.conf.json` 的 `version`
统一改为 `0.2.0`

**Step 4: Run test to verify it passes**

Run: `node --test src/lib/components/settings/autoUpdaterContract.test.js`
Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/tauri.conf.json src/lib/components/settings/autoUpdaterContract.test.js
git commit -m "chore: align app version with release"
```

### Task 2: 接入 Tauri updater 后端能力

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/commands/updater.rs`
- Modify: `src-tauri/src/commands/mod.rs`

**Step 1: Write the failing test**

先写一个最小 Rust 测试或契约测试，确认：
- updater 命令模块已暴露
- 至少存在可供前端调用的检查更新接口

如果插件本身难以在单元测试中完整 mock，可改成契约层测试：验证命令注册与返回结构。

**Step 2: Run test to verify it fails**

Run: `cargo test updater --manifest-path src-tauri/Cargo.toml`
Expected: FAIL，命令或模块尚不存在。

**Step 3: Write minimal implementation**

- 添加 `tauri-plugin-updater`
- 在 `lib.rs` 中注册 plugin
- 新增 `commands/updater.rs`，提供：
  - `check_for_update`
  - `download_update`
  - `install_update`
- 保持返回结构简洁，至少包含：
  - 是否存在更新
  - 目标版本
  - 下载/安装阶段错误信息

**Step 4: Run test to verify it passes**

Run: `cargo test updater --manifest-path src-tauri/Cargo.toml`
Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs src-tauri/src/commands/mod.rs src-tauri/src/commands/updater.rs
git commit -m "feat: add tauri updater backend commands"
```

### Task 3: 接入前端更新状态机与设置页行为

**Files:**
- Modify: `src/lib/utils/invoke.ts`
- Modify: `src/lib/components/settings/ProviderSettings.svelte`
- Create: `src/lib/components/settings/autoUpdaterState.test.js`

**Step 1: Write the failing test**

为前端状态逻辑写最小测试，覆盖：
- 手动检查无更新 → `up-to-date`
- 检查到更新且 `autoUpdate=true` → 自动进入 `downloading`
- 下载完成 → `downloaded`
- 失败 → `error`

如有必要，可把状态转换逻辑抽到单独 helper 中再测。

**Step 2: Run test to verify it fails**

Run: `node --test src/lib/components/settings/autoUpdaterState.test.js`
Expected: FAIL，状态逻辑或 helper 尚不存在。

**Step 3: Write minimal implementation**

- 在 `invoke.ts` 暴露 updater API
- 将设置页中的 `handleCheckUpdate()` 从占位实现替换为真实调用
- 发现更新时在 `autoUpdate=true` 下自动后台下载
- 下载完成后显示“重启安装”提示和按钮
- 保留错误提示与 GitHub release 页面兜底

**Step 4: Run test to verify it passes**

Run: `node --test src/lib/components/settings/autoUpdaterState.test.js`
Expected: PASS

**Step 5: Commit**

```bash
git add src/lib/utils/invoke.ts src/lib/components/settings/ProviderSettings.svelte src/lib/components/settings/autoUpdaterState.test.js
git commit -m "feat: wire real auto updater UI flow"
```

### Task 4: 调整 release workflow 生成 updater 资产

**Files:**
- Modify: `.github/workflows/release.yml`
- Create: `docs/plans/2026-03-08-auto-updater-ops.md` (optional if needed for secrets/instructions)
- Update: `src/lib/components/settings/autoUpdaterContract.test.js`

**Step 1: Write the failing test**

扩展契约测试，要求：
- `release.yml` 中 updater 元数据生成已开启
- workflow 中预留 updater 签名所需环境变量/secret 说明

**Step 2: Run test to verify it fails**

Run: `node --test src/lib/components/settings/autoUpdaterContract.test.js`
Expected: FAIL，当前 `includeUpdaterJson` 为 `false`。

**Step 3: Write minimal implementation**

- 修改 `release.yml`，开启 updater 元数据生成
- 调整 release body 说明，避免继续写“首个公开版本”
- 如有必要，补一份 ops 说明，写清：
  - 需要的 GitHub Secrets
  - 发版前版本号同步要求
  - updater 密钥管理方式

**Step 4: Run test to verify it passes**

Run: `node --test src/lib/components/settings/autoUpdaterContract.test.js`
Expected: PASS

**Step 5: Commit**

```bash
git add .github/workflows/release.yml docs/plans/2026-03-08-auto-updater-ops.md src/lib/components/settings/autoUpdaterContract.test.js
git commit -m "ci: enable updater assets in release workflow"
```

### Task 5: 全量验证并收口

**Files:**
- Modify: any files above only if verification exposes gaps

**Step 1: Write the failing test**

如果前几步暴露新的边界行为，先补最小回归测试。

**Step 2: Run test to verify it fails**

Run the specific new failing test.
Expected: FAIL with expected reason.

**Step 3: Write minimal implementation**

仅修补验证暴露的缺口，不做额外范围扩展。

**Step 4: Run test to verify it passes**

Run:
- `node --test src/lib/components/settings/autoUpdaterContract.test.js src/lib/components/settings/autoUpdaterState.test.js`
- `pnpm run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`

Expected:
- Node tests PASS
- `svelte-check` 0 errors / 0 warnings
- Rust tests PASS

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: complete github-based auto updater flow"
```
