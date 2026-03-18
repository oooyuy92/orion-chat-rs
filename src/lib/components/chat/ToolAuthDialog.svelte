<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
  import { agentAuthorizeTool } from '$lib/api/agent';
  import { pendingAuth } from '$lib/stores/agent';
  import type { AuthAction } from '$lib/types';

  let open = $state(false);

  $effect(() => {
    open = $pendingAuth !== null;
  });

  async function respond(action: AuthAction) {
    if (!$pendingAuth) {
      return;
    }

    await agentAuthorizeTool($pendingAuth.toolCallId, action);
    pendingAuth.set(null);
  }

  async function handleOpenChange(nextOpen: boolean) {
    if (!nextOpen && $pendingAuth) {
      await respond('deny');
      return;
    }

    open = nextOpen;
  }

  function formatArgs(args: string): string {
    try {
      return JSON.stringify(JSON.parse(args), null, 2);
    } catch {
      return args;
    }
  }
</script>

<Dialog.Root bind:open={() => open, handleOpenChange}>
  <Dialog.Content class="sm:max-w-md">
    <Dialog.Header>
      <Dialog.Title>Tool authorization</Dialog.Title>
      <Dialog.Description>
        Agent wants to run the following tool.
      </Dialog.Description>
    </Dialog.Header>

    {#if $pendingAuth}
      <div class="auth-body">
        <div class="tool-name">{$pendingAuth.toolName}</div>
        <pre class="tool-args">{formatArgs($pendingAuth.args)}</pre>
      </div>
    {/if}

    <Dialog.Footer>
      <Button variant="destructive" size="sm" onclick={() => respond('deny')}>
        Deny
      </Button>
      <div class="auth-actions">
        <Button variant="outline" size="sm" onclick={() => respond('allowSession')}>
          Allow and remember
        </Button>
        <Button size="sm" onclick={() => respond('allow')}>
          Allow
        </Button>
      </div>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<style>
  .auth-body {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .tool-name {
    font-family: monospace;
    font-size: 0.95rem;
    font-weight: 600;
  }

  .tool-args {
    margin: 0;
    max-height: 12rem;
    overflow: auto;
    border-radius: 0.5rem;
    background: var(--muted);
    padding: 0.75rem;
    font-size: 0.75rem;
    font-family: monospace;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .auth-actions {
    display: flex;
    gap: 0.5rem;
  }
</style>
