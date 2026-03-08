/**
 * @param {string} itemKey
 * @param {Map<string, number>} heightCache
 * @param {number} estimatedItemHeight
 */
export function getMeasuredHeight(itemKey, heightCache, estimatedItemHeight) {
  return heightCache.get(itemKey) ?? estimatedItemHeight;
}

/**
 * @param {string[]} itemKeys
 * @param {Map<string, number>} heightCache
 * @param {number} estimatedItemHeight
 */
export function estimateTotalHeight(itemKeys, heightCache, estimatedItemHeight) {
  return itemKeys.reduce(
    (total, itemKey) => total + getMeasuredHeight(itemKey, heightCache, estimatedItemHeight),
    0,
  );
}

/**
 * @param {string[]} itemKeys
 * @param {Map<string, number>} heightCache
 * @param {number} estimatedItemHeight
 * @param {number} endIndex
 */
function getOffsetBeforeIndex(itemKeys, heightCache, estimatedItemHeight, endIndex) {
  let total = 0;
  for (let index = 0; index < endIndex; index += 1) {
    total += getMeasuredHeight(itemKeys[index], heightCache, estimatedItemHeight);
  }
  return total;
}

/**
 * @param {{
 *   itemKeys: string[];
 *   heightCache: Map<string, number>;
 *   viewportHeight: number;
 *   scrollTop: number;
 *   overscan?: number;
 *   estimatedItemHeight?: number;
 * }} params
 */
export function calculateVirtualWindow({
  itemKeys,
  heightCache,
  viewportHeight,
  scrollTop,
  overscan = 2,
  estimatedItemHeight = 160,
}) {
  const totalCount = itemKeys.length;
  const totalHeight = estimateTotalHeight(itemKeys, heightCache, estimatedItemHeight);

  if (totalCount === 0) {
    return {
      startIndex: 0,
      endIndex: 0,
      topSpacerHeight: 0,
      bottomSpacerHeight: 0,
      totalHeight: 0,
    };
  }

  const clampedScrollTop = Math.max(0, Math.min(scrollTop, Math.max(0, totalHeight - viewportHeight)));
  const viewportBottom = clampedScrollTop + Math.max(0, viewportHeight);

  let visibleStartIndex = 0;
  let offset = 0;
  while (visibleStartIndex < totalCount) {
    const itemHeight = getMeasuredHeight(itemKeys[visibleStartIndex], heightCache, estimatedItemHeight);
    if (offset + itemHeight > clampedScrollTop) {
      break;
    }
    offset += itemHeight;
    visibleStartIndex += 1;
  }

  let visibleEndIndex = visibleStartIndex;
  let visibleOffset = offset;
  while (visibleEndIndex < totalCount && visibleOffset < viewportBottom) {
    visibleOffset += getMeasuredHeight(itemKeys[visibleEndIndex], heightCache, estimatedItemHeight);
    visibleEndIndex += 1;
  }

  const startIndex = Math.max(0, visibleStartIndex - overscan);
  const endIndex = Math.min(totalCount, visibleEndIndex + overscan);
  const topSpacerHeight = getOffsetBeforeIndex(itemKeys, heightCache, estimatedItemHeight, startIndex);
  const renderedHeight = getOffsetBeforeIndex(itemKeys, heightCache, estimatedItemHeight, endIndex) - topSpacerHeight;
  const bottomSpacerHeight = Math.max(0, totalHeight - topSpacerHeight - renderedHeight);

  return {
    startIndex,
    endIndex,
    topSpacerHeight,
    bottomSpacerHeight,
    totalHeight,
  };
}

/**
 * @param {{
 *   scrollTop: number;
 *   scrollHeight: number;
 *   clientHeight: number;
 *   threshold?: number;
 * }} params
 */
export function isNearBottom({ scrollTop, scrollHeight, clientHeight, threshold = 64 }) {
  return scrollHeight - (scrollTop + clientHeight) <= threshold;
}

/**
 * @param {{
 *   previousScrollHeight: number;
 *   previousScrollTop: number;
 *   nextScrollHeight: number;
 * }} params
 */
export function getAdjustedScrollTopAfterPrepend({
  previousScrollHeight,
  previousScrollTop,
  nextScrollHeight,
}) {
  return previousScrollTop + Math.max(0, nextScrollHeight - previousScrollHeight);
}
