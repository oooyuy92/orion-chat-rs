<script lang="ts">
  let {
    disabled = false,
    onSend,
  }: {
    disabled?: boolean;
    onSend: (content: string) => void;
  } = $props();

  let text = $state('');
  let textarea: HTMLTextAreaElement | undefined = $state();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  function submit() {
    const trimmed = text.trim();
    if (!trimmed || disabled) return;
    onSend(trimmed);
    text = '';
    if (textarea) {
      textarea.style.height = 'auto';
    }
  }

  function autoResize() {
    if (!textarea) return;
    textarea.style.height = 'auto';
    textarea.style.height = Math.min(textarea.scrollHeight, 200) + 'px';
  }
</script>

<div
  class="px-4 py-3 border-t"
  style="border-color: var(--border); background-color: var(--bg-secondary);"
>
  <div
    class="flex items-end gap-2 rounded-xl px-3 py-2"
    style="background-color: var(--bg-primary); border: 1px solid var(--border);"
  >
    <textarea
      bind:this={textarea}
      bind:value={text}
      onkeydown={handleKeydown}
      oninput={autoResize}
      {disabled}
      placeholder="Type a message..."
      rows="1"
      class="flex-1 resize-none text-sm bg-transparent outline-none"
      style="color: var(--text-primary); max-height: 200px; border: none;"
    ></textarea>
    <button
      onclick={submit}
      disabled={disabled || !text.trim()}
      class="px-3 py-1.5 rounded-lg text-sm font-medium cursor-pointer transition-colors"
      style="background-color: {disabled || !text.trim() ? 'var(--border)' : 'var(--accent)'}; color: #fff; border: none;"
    >
      Send
    </button>
  </div>
</div>
