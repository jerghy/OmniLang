// ── LRU Tab Data Eviction ──
// Shared utility to prevent unbounded tabData growth when closing tabs or removing components.

const DEFAULT_TAB_DATA_MAX = 20;

export function lruEvictTabData<T>(
  tabData: Record<string, T>,
  openTabIds: Set<string>,
  maxSize: number = DEFAULT_TAB_DATA_MAX,
): Record<string, T> {
  const keys = Object.keys(tabData);
  if (keys.length <= maxSize) return tabData;
  const toRemove = keys.length - maxSize;
  const removable = keys.filter((k) => !openTabIds.has(k)).sort();
  let result = tabData;
  for (let i = 0; i < Math.min(toRemove, removable.length); i++) {
    const { [removable[i]]: _, ...rest } = result;
    result = rest as Record<string, T>;
  }
  return result;
}
