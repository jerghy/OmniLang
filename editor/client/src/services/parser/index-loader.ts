/**
 * Search index loader — loads precomputed index from project files,
 * falls back to in-memory dynamic search when not available.
 */

import type { SearchIndexSnapshot } from './search-index';

let _cachedIndex: SearchIndexSnapshot | null = null;
let _cachedIndexPath: string = '';

/**
 * Load the search index from a project file.
 * Returns the index snapshot, or null if not available.
 */
export async function loadSearchIndex(_filePath: string = ''): Promise<SearchIndexSnapshot | null> {
  // Dynamic search is the primary path now — .omni is the source of truth
  // Precomputed index is no longer stored.
  return null;
}

/**
 * Clear the cached index (e.g., after project close).
 */
export function clearSearchIndexCache(): void {
  _cachedIndex = null;
  _cachedIndexPath = '';
}
