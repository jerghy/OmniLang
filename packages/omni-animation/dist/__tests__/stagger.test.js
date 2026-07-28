import { describe, it, expect } from 'vitest';
import { staggerDelays } from '../stagger';
describe('staggerDelays', () => {
    it('computes evenly-spaced delays', () => {
        const delays = staggerDelays(5, 100);
        expect(delays).toEqual([0, 100, 200, 300, 400]);
    });
    it('handles single element', () => {
        expect(staggerDelays(1, 100)).toEqual([0]);
    });
});
