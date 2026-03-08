// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  calculateVirtualWindow,
  estimateTotalHeight,
  getAdjustedScrollTopAfterPrepend,
  getMeasuredHeight,
  isNearBottom,
} from './messageVirtualization.js';

test('returns full range when item count fits in viewport', () => {
  const itemKeys = ['a', 'b', 'c'];
  const heightCache = new Map([
    ['a', 80],
    ['b', 120],
    ['c', 90],
  ]);

  const result = calculateVirtualWindow({
    itemKeys,
    heightCache,
    viewportHeight: 500,
    scrollTop: 0,
    overscan: 1,
    estimatedItemHeight: 100,
  });

  assert.deepEqual(result, {
    startIndex: 0,
    endIndex: 3,
    topSpacerHeight: 0,
    bottomSpacerHeight: 0,
    totalHeight: 290,
  });
});

test('uses estimated height for unmeasured items', () => {
  const heightCache = new Map([['known', 140]]);

  assert.equal(getMeasuredHeight('known', heightCache, 96), 140);
  assert.equal(getMeasuredHeight('unknown', heightCache, 96), 96);
  assert.equal(estimateTotalHeight(['known', 'unknown', 'another'], heightCache, 96), 332);
});

test('calculates window and spacers around middle scroll position', () => {
  const itemKeys = ['0', '1', '2', '3', '4', '5', '6', '7'];
  const heightCache = new Map(itemKeys.map((key) => [key, 100]));

  const result = calculateVirtualWindow({
    itemKeys,
    heightCache,
    viewportHeight: 180,
    scrollTop: 260,
    overscan: 1,
    estimatedItemHeight: 100,
  });

  assert.deepEqual(result, {
    startIndex: 1,
    endIndex: 6,
    topSpacerHeight: 100,
    bottomSpacerHeight: 200,
    totalHeight: 800,
  });
});

test('clamps the visible window near the bottom of the list', () => {
  const itemKeys = ['0', '1', '2', '3', '4', '5', '6'];
  const heightCache = new Map(itemKeys.map((key) => [key, 100]));

  const result = calculateVirtualWindow({
    itemKeys,
    heightCache,
    viewportHeight: 180,
    scrollTop: 520,
    overscan: 1,
    estimatedItemHeight: 100,
  });

  assert.deepEqual(result, {
    startIndex: 4,
    endIndex: 7,
    topSpacerHeight: 400,
    bottomSpacerHeight: 0,
    totalHeight: 700,
  });
});

test('detects near-bottom state with a threshold', () => {
  assert.equal(
    isNearBottom({
      scrollTop: 700,
      scrollHeight: 1000,
      clientHeight: 260,
      threshold: 48,
    }),
    true,
  );

  assert.equal(
    isNearBottom({
      scrollTop: 640,
      scrollHeight: 1000,
      clientHeight: 260,
      threshold: 48,
    }),
    false,
  );
});

test('restores scrollTop after prepending older messages', () => {
  assert.equal(
    getAdjustedScrollTopAfterPrepend({
      previousScrollHeight: 1200,
      previousScrollTop: 18,
      nextScrollHeight: 1680,
    }),
    498,
  );
});
