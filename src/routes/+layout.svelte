<script lang="ts">
  import '../app.css';

  let { children } = $props();
</script>

<div class="flex h-screen" style="background-color: var(--background); color: var(--foreground);">
  {@render children()}
</div>
