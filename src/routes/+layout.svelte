<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import { i18n } from '$lib/stores/i18n.svelte';

  let { children } = $props();

  onMount(() => {
    void i18n.init();
  });
</script>

<div class="flex h-screen" style="background-color: var(--background); color: var(--foreground);">
  {@render children()}
</div>
