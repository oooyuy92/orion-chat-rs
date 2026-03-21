// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const viteConfigSource = readFileSync(new URL('../vite.config.ts', import.meta.url), 'utf8');
const packageJson = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));

test('vite config includes macOS 12 Safari CSS nesting compatibility transform', () => {
  assert.match(viteConfigSource, /import \{ transform, Features \} from 'lightningcss';/);
  assert.match(viteConfigSource, /function cssNestingCompat\(\): Plugin/);
  assert.match(viteConfigSource, /name: 'css-nesting-compat'/);
  assert.match(viteConfigSource, /include: Features\.Nesting \| Features\.MediaQueries/);
  assert.match(viteConfigSource, /targets: \{ safari: \(15 << 16\) \| \(6 << 8\) \}/);
  assert.match(viteConfigSource, /plugins: \[tailwindcss\(\), cssNestingCompat\(\), sveltekit\(\)\]/);
});

test('package manifest pins lightningcss for explicit compatibility transform usage', () => {
  assert.equal(packageJson.devDependencies?.lightningcss, '1.31.1');
});
