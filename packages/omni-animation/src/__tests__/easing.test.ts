import { describe, it, expect } from 'vitest';
import { resolveEasing } from '../easing';

describe('resolveEasing', () => {
  it('resolves named easings to cubic-bezier', () => {
    expect(resolveEasing('power1In')).toContain('cubic-bezier');
  });

  it('returns parameterized easings as-is (removed feature)', () => {
    const result = resolveEasing('backIn(1.5)');
    expect(result).toBe('backIn(1.5)');
  });

  it('returns unknown easings as-is', () => {
    expect(resolveEasing('customCurve')).toBe('customCurve');
  });
});
