/**
 * Client-side structural validation for .omni.json documents.
 * Uses lightweight structural checks (no JSON Schema library dependency).
 */

import type { OmniJsonDocument } from './types';

const REQUIRED_KEYS = [
  'schemaVersion',
  'meta',
  'components',
  'stores',
  'elements',
  'wires',
] as const;

const SUPPORTED_VERSIONS = ['1.0.0'] as const;

export interface ValidationError {
  path: string;
  message: string;
}

export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
}

/**
 * Validate a parsed .omni.json document at the structural level.
 * Returns all errors found (does not stop at first error).
 */
export function validateOmniJson(data: unknown): ValidationResult {
  const errors: ValidationError[] = [];

  if (!data || typeof data !== 'object') {
    return { valid: false, errors: [{ path: '$', message: 'Document must be an object' }] };
  }

  const doc = data as Record<string, unknown>;

  // Check required top-level keys
  for (const key of REQUIRED_KEYS) {
    if (!(key in doc)) {
      errors.push({ path: `$.${key}`, message: `Missing required field: ${key}` });
    }
  }

  // Check schema version
  const version = doc.schemaVersion as string;
  if (version && !(SUPPORTED_VERSIONS as readonly string[]).includes(version)) {
    errors.push({
      path: '$.schemaVersion',
      message: `Unsupported schema version: ${version}. Supported: ${SUPPORTED_VERSIONS.join(', ')}`,
    });
  }

  // Validate arrays are actually arrays
  const arrayFields = ['components', 'stores', 'themes', 'imports', 'elements', 'wires', 'ownershipRegions'];
  for (const field of arrayFields) {
    const val = doc[field];
    if (val !== undefined && !Array.isArray(val)) {
      errors.push({ path: `$.${field}`, message: `${field} must be an array` });
    }
  }

  // Validate elements have required fields
  if (Array.isArray(doc.elements)) {
    for (let i = 0; i < doc.elements.length; i++) {
      const el = doc.elements[i] as Record<string, unknown>;
      if (!el.id) {
        errors.push({ path: `$.elements[${i}]`, message: 'Element missing required field: id' });
      }
      if (el.nodeType && !['view', 'codeblock', 'group'].includes(el.nodeType as string)) {
        errors.push({
          path: `$.elements[${i}].nodeType`,
          message: `Invalid nodeType: ${el.nodeType}. Must be view, codeblock, or group`,
        });
      }
    }
  }

  // Validate wires have required fields
  if (Array.isArray(doc.wires)) {
    for (let i = 0; i < doc.wires.length; i++) {
      const w = doc.wires[i] as Record<string, unknown>;
      if (!w.id) {
        errors.push({ path: `$.wires[${i}]`, message: 'Wire missing required field: id' });
      }
    }
  }

  // Validate search index entries
  const si = doc.searchIndex as Record<string, unknown> | undefined;
  if (si && Array.isArray(si.entries)) {
    for (let i = 0; i < (si.entries as unknown[]).length; i++) {
      const entry = (si.entries as Record<string, unknown>[])[i];
      if (!entry.id || !entry.title) {
        errors.push({
          path: `$.searchIndex.entries[${i}]`,
          message: 'Search index entry missing required field: id or title',
        });
      }
    }
  }

  return { valid: errors.length === 0, errors };
}

/**
 * Quick check: is the parsed data likely a valid OmniJsonDocument?
 */
export function isOmniJsonDocument(data: unknown): data is OmniJsonDocument {
  return validateOmniJson(data).valid;
}
