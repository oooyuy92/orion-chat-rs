<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import FolderOpenIcon from '@lucide/svelte/icons/folder-open';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';

  interface SkillInfo {
    name: string;
    path: string;
    description: string;
    enabled: boolean;
  }

  let skillsDir = $state('');
  let skills = $state<SkillInfo[]>([]);
  let editing = $state(false);

  onMount(() => {
    void loadSkills();
  });

  async function loadSkills() {
    skillsDir = await invoke<string>('get_skills_dir').catch(() => '');

    if (!skillsDir) {
      skills = [];
      return;
    }

    skills = await invoke<SkillInfo[]>('scan_skills').catch(() => []);
  }

  async function saveDir() {
    await invoke('set_skills_dir', { dir: skillsDir });
    skills = await invoke<SkillInfo[]>('scan_skills').catch(() => []);
    editing = false;
  }
</script>

<div class="space-y-3">
  <div class="flex items-center gap-2">
    <span class="text-xs text-muted-foreground">Skills 目录</span>
    {#if editing}
      <Input
        class="h-7 flex-1 text-xs"
        bind:value={skillsDir}
        placeholder="~/.orion/skills/"
        onkeydown={(event) => {
          if (event.key === 'Enter') {
            void saveDir();
          }
        }}
      />
      <Button size="sm" variant="outline" class="h-7 text-xs" onclick={() => void saveDir()}>
        保存
      </Button>
    {:else}
      <span class="flex-1 truncate font-mono text-xs">{skillsDir || '未设置'}</span>
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
            <span class="ml-2 text-muted-foreground">{skill.description}</span>
          </div>
        </div>
      {/each}
    </div>
  {:else if skillsDir}
    <p class="text-xs text-muted-foreground">未找到 Skills 文件</p>
  {/if}
</div>
