<script lang="ts">
  import { onMount } from 'svelte';
  import { loadStore } from '$lib/stores/kvStore';
  import '../app.css';
  import { i18n } from '$lib/stores/i18n.svelte';
  import { applyTheme } from '$lib/utils/theme.js';

  let { children } = $props();

  onMount(() => {
    void i18n.init();
    void (async () => {
      try {
        const store = await loadStore('settings.json');
        applyTheme(await store.get<string>('theme'));
      } catch {
        applyTheme(undefined);
      }
    })();
  });
</script>

<div class="flex h-screen" style="background-color: var(--background); color: var(--foreground);">
  {@render children()}
</div>
