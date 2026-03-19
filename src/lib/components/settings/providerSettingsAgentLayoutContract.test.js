// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./ProviderSettings.svelte', import.meta.url), 'utf8');

test('provider settings renders agent settings inside agent panel', () => {
  assert.match(
    source,
    /{:else if activeNav === 'agent'}\s*<section class="agent-panel">\s*<AgentSettings \/>/s,
  );
});

test('provider settings gives agent panel the same full-width right-column layout as other settings panels', () => {
  assert.match(source, /\.agent-panel\s*\{/);
  assert.match(source, /\.agent-panel\s*\{[^}]*grid-column:\s*2\s*\/\s*-1;/s);
  assert.match(source, /\.agent-panel\s*\{[^}]*padding:\s*1\.2rem 1\.5rem;/s);
  assert.match(source, /\.agent-panel\s*\{[^}]*min-height:\s*0;/s);
});
