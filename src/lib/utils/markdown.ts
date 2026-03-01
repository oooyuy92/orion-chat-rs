import { Marked } from 'marked';
import hljs from 'highlight.js';

const marked = new Marked({
  renderer: {
    code({ text, lang }: { text: string; lang?: string }) {
      const language = lang && hljs.getLanguage(lang) ? lang : 'plaintext';
      const highlighted = hljs.highlight(text, { language }).value;
      return `<pre><code class="hljs language-${language}">${highlighted}</code></pre>`;
    },
  },
});

export function renderMarkdown(content: string): string {
  if (!content) return '';
  const result = marked.parse(content);
  if (typeof result === 'string') return result;
  // marked.parse can return Promise in async mode, but we use sync
  return '';
}
