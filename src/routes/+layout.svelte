<script lang="ts">
  import '../app.css';
  import { setTheme } from '$lib/stores/ui.svelte';
  import { onMount } from 'svelte';

  let { children } = $props();

  onMount(() => {
    setTheme('light');
  });
</script>

<div class="flex h-screen" style="background-color: var(--background); color: var(--foreground);">
  {@render children()}
</div>
