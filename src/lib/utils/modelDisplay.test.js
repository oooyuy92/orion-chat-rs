// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';

const { isManualModel, resolveModelLabel, resolveModelSecondaryLabel } = await import('./modelDisplay.ts');

test('resolveModelLabel prefers display name over request name', () => {
  assert.equal(
    resolveModelLabel({
      id: 'm1',
      name: 'Friendly GPT',
      requestName: 'gpt-4.1',
      displayName: 'Friendly GPT',
      source: 'manual',
    }),
    'Friendly GPT',
  );
});

test('resolveModelLabel falls back to request name when display name is missing', () => {
  assert.equal(
    resolveModelLabel({
      id: 'm2',
      name: 'gpt-4.1-mini',
      requestName: 'gpt-4.1-mini',
      displayName: null,
      source: 'synced',
    }),
    'gpt-4.1-mini',
  );
});

test('resolveModelLabel falls back to internal id when display and request names are empty', () => {
  assert.equal(
    resolveModelLabel({
      id: 'm3',
      name: '',
      requestName: '',
      displayName: '',
      source: 'synced',
    }),
    'm3',
  );
});

test('resolveModelSecondaryLabel hides duplicate request name and exposes non-empty fallback', () => {
  assert.equal(
    resolveModelSecondaryLabel({
      id: 'm1',
      name: 'Friendly GPT',
      requestName: 'gpt-4.1',
      displayName: 'Friendly GPT',
      source: 'manual',
    }),
    'gpt-4.1',
  );
  assert.equal(
    resolveModelSecondaryLabel({
      id: 'm2',
      name: 'gpt-4.1-mini',
      requestName: 'gpt-4.1-mini',
      displayName: null,
      source: 'synced',
    }),
    '',
  );
});

test('isManualModel detects manual source', () => {
  assert.equal(
    isManualModel({
      id: 'm1',
      name: 'Friendly GPT',
      requestName: 'gpt-4.1',
      displayName: 'Friendly GPT',
      source: 'manual',
    }),
    true,
  );
  assert.equal(
    isManualModel({
      id: 'm2',
      name: 'gpt-4.1-mini',
      requestName: 'gpt-4.1-mini',
      displayName: null,
      source: 'synced',
    }),
    false,
  );
});
