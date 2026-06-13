/**
 * Search index loader — loads precomputed index from .omni.json on project open,
 * falls back to in-memory dynamic search when .omni.json is not available.
 */

import type { SearchIndexSnapshot } from '../omniJson/types';
import { api } from '../api/client';

let _cachedIndex: SearchIndexSnapshot | null = null;
let _cachedIndexPath: string = '';

/**
 * Load the search index from a .omni.json file.
 * Returns the index snapshot, or null if not available.
 */
export async function loadSearchIndex(filePath: string = 'project.omni.json'): Promise<SearchIndexSnapshot | null> {
  // Return cached if same path
  if (_cachedIndex && _cachedIndexPath === filePath) {
    return _cachedIndex;
  }

  try {
    const resp = await api.post('/api/omni-json/load', {
      body: JSON.stringify({ filePath }),
    });
    const doc = resp.document as Record<string, unknown> | undefined;
    if (resp.success && doc?.searchIndex) {
      _cachedIndex = doc.searchIndex as SearchIndexSnapshot;
      _cachedIndexPath = filePath;
      return _cachedIndex;
    }
  } catch {
    // Fall through to null
  }

  return null;
}

/**
 * Clear the cached index (e.g., after project close).
 */
export function clearSearchIndexCache(): void {
  _cachedIndex = null;
  _cachedIndexPath = '';
}
