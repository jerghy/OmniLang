import { describe, it, expect } from 'vitest';
import { resolveEasing } from '../easing';
describe('resolveEasing', () => {
    it('resolves named easings to cubic-bezier', () => {
        expect(resolveEasing('power1In')).toContain('cubic-bezier');
    });
    it('resolves parameterized easings', () => {
        const result = resolveEasing('backIn(1.5)');
        expect(result).toContain('cubic-bezier');
    });
    it('returns unknown easings as-is', () => {
        expect(resolveEasing('customCurve')).toBe('customCurve');
    });
});
