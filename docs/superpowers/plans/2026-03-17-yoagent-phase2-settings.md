# yoagent Phase 2 — Agent 设置页面 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Orion Chat 设置页面中新增 Agent 分区，支持工具授权等级配置持久化和 Skills 目录管理。

**Architecture:** 在现有 ProviderSettings.svelte 的设置导航中新增 "Agent" 分区，包含工具授权配置（下拉选择 auto/ask/deny）和 Skills 目录扫描。后端新增 Skills 管理的 Tauri commands。

**Tech Stack:** Svelte 5, shadcn-svelte, Tauri v2 IPC, TypeScript

**前置依赖:** Phase 0 + Phase 1 已完成。

---

## 文件结构

### 新建文件
- `src/lib/components/settings/AgentSettings.svelte` — Agent 设置面板主组件
- `src/lib/components/settings/ToolPermissionRow.svelte` — 单个工具权限配置行
- `src/lib/components/settings/SkillsManager.svelte` — Skills 目录扫描和管理
- `src-tauri/src/agent/skills.rs` — Skills 扫描和管理 Tauri commands

### 修改文件
- `src/lib/components/settings/ProviderSettings.svelte` — 添加 Agent 设置导航项
- `src-tauri/src/agent/mod.rs` — 添加 `pub mod skills;`
- `src-tauri/src/agent/commands.rs` — 添加 skills 相关 commands
- `src-tauri/src/lib.rs` — 注册新 commands

---

## Chunk 0: 后端 Skills 管理

### Task 0: agent/skills.rs — Skills 扫描

**Files:**
- Create: `src-tauri/src/agent/skills.rs`
- Modify: `src-tauri/src/agent/mod.rs`
- Modify: `src-tauri/src/agent/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建 skills.rs**

```rust
// src-tauri/src/agent/skills.rs
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::errors::{AppResult, AppError};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInfo {
    pub name: String,
    pub path: String,
    pub description: String,
    pub enabled: bool,
}

/// 扫描指定目录下的 .md 文件作为 Skills
pub fn scan_skills_dir(dir: &str) -> AppResult<Vec<SkillInfo>> {
    let path = Path::new(dir);
    if !path.exists() || !path.is_dir() {
        return Ok(vec![]);
    }

    let mut skills = Vec::new();
    for entry in std::fs::read_dir(path)
        .map_err(|e| AppError::Io(e.to_string()))?
    {
        let entry = entry.map_err(|e| AppError::Io(e.to_string()))?;
        let file_path = entry.path();
        if file_path.extension().map_or(false, |ext| ext == "md") {
            let name = file_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let content = std::fs::read_to_string(&file_path)
                .unwrap_or_default();
            let description = content.lines().next().unwrap_or("").to_string();
            skills.push(SkillInfo {
                name,
                path: file_path.to_string_lossy().to_string(),
                description,
                enabled: true,
            });
        }
    }

    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_scan_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let skills = scan_skills_dir(dir.path().to_str().unwrap()).unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_scan_dir_with_skills() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("code-review.md"), "# Code Review Skill\nReview code").unwrap();
        fs::write(dir.path().join("not-a-skill.txt"), "ignored").unwrap();

        let skills = scan_skills_dir(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "code-review");
    }

    #[test]
    fn test_scan_nonexistent_dir() {
        let skills = scan_skills_dir("/nonexistent/path").unwrap();
        assert!(skills.is_empty());
    }
}
```

- [ ] **Step 2: 在 mod.rs 中添加模块**

```rust
pub mod skills;
```

- [ ] **Step 3: 在 commands.rs 中添加 Skills 相关 commands**

```rust
use crate::agent::skills::{self, SkillInfo};

#[tauri::command]
pub async fn get_skills_dir(
    state: State<'_, Arc<AppState>>,
) -> AppResult<String> {
    let dir: String = state.db.get_conn()?
        .query_row(
            "SELECT value FROM agent_settings WHERE key = 'skills_dir'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_default();
    Ok(dir)
}

#[tauri::command]
pub async fn set_skills_dir(
    state: State<'_, Arc<AppState>>,
    dir: String,
) -> AppResult<()> {
    state.db.get_conn()?
        .execute(
            "INSERT OR REPLACE INTO agent_settings (key, value) VALUES ('skills_dir', ?1)",
            rusqlite::params![dir],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn scan_skills(
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<SkillInfo>> {
    let dir = get_skills_dir(state.clone()).await?;
    if dir.is_empty() {
        return Ok(vec![]);
    }
    skills::scan_skills_dir(&dir)
}
```

- [ ] **Step 4: 注册 commands 到 lib.rs**

```rust
agent::commands::get_skills_dir,
agent::commands::set_skills_dir,
agent::commands::scan_skills,
```

- [ ] **Step 5: 运行测试**

```bash
cd src-tauri && cargo test agent::skills::tests 2>&1 | tail -10
```

预期：`test result: ok. 3 passed`

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/agent/skills.rs src-tauri/src/agent/mod.rs src-tauri/src/agent/commands.rs src-tauri/src/lib.rs
git commit -m "feat: Skills scanning and management backend"
```

---

## Chunk 1: Agent 设置前端组件

### Task 1: ToolPermissionRow 组件

**Files:**
- Create: `src/lib/components/settings/ToolPermissionRow.svelte`

- [ ] **Step 1: 创建组件**

```svelte
<!-- src/lib/components/settings/ToolPermissionRow.svelte -->
<script lang="ts">
  import * as Select from '$lib/components/ui/select';
  import type { PermissionLevel } from '$lib/types';

  let {
    toolName,
    level,
    onLevelChange,
  }: {
    toolName: string;
    level: PermissionLevel;
    onLevelChange: (level: PermissionLevel) => void;
  } = $props();

  const options: { value: PermissionLevel; label: string }[] = [
    { value: 'auto', label: '自动执行' },
    { value: 'ask', label: '需要确认' },
    { value: 'deny', label: '禁用' },
  ];
</script>

<div class="flex items-center justify-between py-2">
  <span class="font-mono text-sm">{toolName}</span>
  <Select.Root
    type="single"
    value={level}
    onValueChange={(v) => { if (v) onLevelChange(v as PermissionLevel); }}
  >
    <Select.Trigger class="w-32 h-8 text-xs">
      {options.find(o => o.value === level)?.label ?? level}
    </Select.Trigger>
    <Select.Content>
      {#each options as opt}
        <Select.Item value={opt.value}>{opt.label}</Select.Item>
      {/each}
    </Select.Content>
  </Select.Root>
</div>
```

**注意**：Select 组件的导入路径和 API 需按项目已有 shadcn-svelte 用法调整。

- [ ] **Step 2: 提交**

```bash
git add src/lib/components/settings/ToolPermissionRow.svelte
git commit -m "feat: ToolPermissionRow component for per-tool permission dropdown"
```

### Task 2: AgentSettings 主组件

**Files:**
- Create: `src/lib/components/settings/AgentSettings.svelte`

- [ ] **Step 1: 创建组件**

```svelte
<!-- src/lib/components/settings/AgentSettings.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import ToolPermissionRow from './ToolPermissionRow.svelte';
  import SkillsManager from './SkillsManager.svelte';
  import type { ToolPermissions, PermissionLevel } from '$lib/types';
  import { getToolPermissions, setToolPermissions } from '$lib/api/agent';

  let permissions = $state<ToolPermissions>({});
  let workingDir = $state('');
  let loading = $state(true);

  onMount(async () => {
    permissions = await getToolPermissions();
    workingDir = await invoke<string>('get_skills_dir').catch(() => '');
    loading = false;
  });

  async function updatePermission(toolName: string, level: PermissionLevel) {
    permissions[toolName] = level;
    await setToolPermissions(permissions);
  }

  async function updateWorkingDir() {
    await invoke('set_skills_dir', { dir: workingDir });
  }

  const builtinTools = ['read_file', 'list_files', 'search', 'edit_file', 'write_file', 'bash'];
</script>

{#if loading}
  <p class="text-sm text-muted-foreground">加载中...</p>
{:else}
  <div class="space-y-6">
    <!-- 工具授权 -->
    <section>
      <h3 class="text-sm font-semibold mb-3">工具授权</h3>
      <div class="divide-y">
        {#each builtinTools as tool}
          <ToolPermissionRow
            toolName={tool}
            level={permissions[tool] ?? 'ask'}
            onLevelChange={(level) => updatePermission(tool, level)}
          />
        {/each}
      </div>
    </section>

    <!-- Skills -->
    <section>
      <h3 class="text-sm font-semibold mb-3">Skills</h3>
      <SkillsManager />
    </section>
  </div>
{/if}
```

### Task 3: SkillsManager 组件

**Files:**
- Create: `src/lib/components/settings/SkillsManager.svelte`

- [ ] **Step 1: 创建组件**

```svelte
<!-- src/lib/components/settings/SkillsManager.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import FolderOpenIcon from '@lucide/svelte/icons/folder-open';

  interface SkillInfo {
    name: string;
    path: string;
    description: string;
    enabled: boolean;
  }

  let skillsDir = $state('');
  let skills = $state<SkillInfo[]>([]);
  let editing = $state(false);

  onMount(async () => {
    skillsDir = await invoke<string>('get_skills_dir').catch(() => '');
    if (skillsDir) {
      skills = await invoke<SkillInfo[]>('scan_skills').catch(() => []);
    }
  });

  async function saveDir() {
    await invoke('set_skills_dir', { dir: skillsDir });
    skills = await invoke<SkillInfo[]>('scan_skills').catch(() => []);
    editing = false;
  }
</script>

<div class="space-y-3">
  <div class="flex items-center gap-2">
    <label class="text-xs text-muted-foreground">Skills 目录</label>
    {#if editing}
      <Input
        class="h-7 text-xs flex-1"
        bind:value={skillsDir}
        placeholder="~/.orion/skills/"
        onkeydown={(e) => { if (e.key === 'Enter') saveDir(); }}
      />
      <Button size="sm" variant="outline" class="h-7 text-xs" onclick={saveDir}>保存</Button>
    {:else}
      <span class="text-xs font-mono flex-1 truncate">{skillsDir || '未设置'}</span>
      <Button size="sm" variant="ghost" class="h-7" onclick={() => (editing = true)}>
        <FolderOpenIcon class="h-3.5 w-3.5" />
      </Button>
    {/if}
  </div>

  {#if skills.length > 0}
    <div class="divide-y text-xs">
      {#each skills as skill}
        <div class="flex items-center justify-between py-2">
          <div>
            <span class="font-medium">{skill.name}</span>
            <span class="text-muted-foreground ml-2">{skill.description}</span>
          </div>
        </div>
      {/each}
    </div>
  {:else if skillsDir}
    <p class="text-xs text-muted-foreground">未找到 Skills 文件</p>
  {/if}
</div>
```

- [ ] **Step 2: 编译验证**

```bash
pnpm check 2>&1 | tail -10
```

- [ ] **Step 3: 提交**

```bash
git add src/lib/components/settings/AgentSettings.svelte src/lib/components/settings/SkillsManager.svelte
git commit -m "feat: AgentSettings and SkillsManager settings components"
```

### Task 4: 集成到设置页面

**Files:**
- Modify: `src/lib/components/settings/ProviderSettings.svelte`

- [ ] **Step 1: 读取 ProviderSettings.svelte**

```bash
cat src/lib/components/settings/ProviderSettings.svelte
```

- [ ] **Step 2: 添加 Agent 导航项和面板**

按现有设置页面的导航模式，添加 "Agent" 导航项（在合适位置），点击后渲染 AgentSettings 组件。

```svelte
<script>
  import AgentSettings from './AgentSettings.svelte';
</script>

<!-- 在导航列表中添加 -->
<!-- { id: 'agent', label: 'Agent', icon: BotIcon } -->

<!-- 在内容区域添加 -->
<!-- {#if activeSection === 'agent'}
  <AgentSettings />
{/if} -->
```

具体位置和代码需按现有导航结构调整。

- [ ] **Step 3: 编译并在浏览器中验证**

```bash
pnpm tauri dev
```

1. 打开设置页面
2. 看到 "Agent" 分区
3. 工具授权列表正确显示 6 个内置工具
4. 修改权限等级后刷新仍然保持

- [ ] **Step 4: 提交**

```bash
git add -u
git commit -m "feat: Phase 2 complete - Agent settings page with tool permissions and skills management"
```

---

## 验收标准

Phase 2 完成的标志：
- [ ] 设置页面显示 "Agent" 分区
- [ ] 6 个内置工具各有下拉选择器（自动执行/需要确认/禁用）
- [ ] 修改权限后持久化到 DB，刷新保持
- [ ] Skills 目录可配置，扫描并展示 .md 文件列表
- [ ] `cargo test agent::skills::tests` 通过
- [ ] `pnpm check` 无 TypeScript 报错
