<script lang="ts">
  import '../app.css';
  import { setTheme } from '$lib/stores/ui.svelte';
  import { onMount } from 'svelte';

  let { children } = $props();

  onMount(() => {
    setTheme('dark');
  });
</script>

<div class="flex h-screen" style="background-color: var(--bg-primary); color: var(--text-primary);">
  {@render children()}
</div>
