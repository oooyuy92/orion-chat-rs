import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig, type Plugin } from 'vite';
import { transform, Features } from 'lightningcss';

const encoder = new TextEncoder();
const decoder = new TextDecoder();

/**
 * Downlevel CSS nesting for older WebKit (Safari < 17.2).
 * Only targets .css files so Svelte :global() is unaffected.
 */
function cssNestingCompat(): Plugin {
  return {
    name: 'css-nesting-compat',
    enforce: 'pre',
    transform(code, id) {
      if (!/\.css(?:\?|$)/.test(id)) return;
      if (id.includes('\0') || id.includes('node_modules')) return;
      const result = transform({
        filename: id.split('?')[0],
        code: encoder.encode(code),
        include: Features.Nesting | Features.MediaQueries,
        targets: { safari: (15 << 16) | (6 << 8) },
        errorRecovery: true,
      });
      return {
        code: decoder.decode(result.code),
        map: result.map ? decoder.decode(result.map) : undefined,
      };
    },
  };
}

export default defineConfig({
  plugins: [tailwindcss(), cssNestingCompat(), sveltekit()],
  clearScreen: false,
  server: {
    strictPort: true,
  },
});
