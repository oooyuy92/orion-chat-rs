// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./+layout.svelte', import.meta.url), 'utf8');

test('layout restores persisted theme on mount', () => {
  assert.match(source, /import \{ load as loadStore \} from '@tauri-apps\/plugin-store';/);
  assert.match(source, /import \{ applyTheme \} from '\$lib\/utils\/theme\.js';/);
  assert.match(source, /const store = await loadStore\('settings\.json'\);/);
  assert.match(source, /applyTheme\(await store\.get<string>\('theme'\)\);/);
  assert.match(source, /applyTheme\(undefined\);/);
});
