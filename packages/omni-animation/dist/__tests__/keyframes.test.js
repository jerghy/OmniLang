import { describe, it, expect } from 'vitest';
import { buildKeyframes, buildProperties } from '../keyframes';
describe('buildProperties', () => {
    it('converts x/y/rotate to transform properties', () => {
        const result = buildProperties({ x: 100, y: 50, rotate: 45, opacity: 0.5 });
        expect(result.transform).toBe('translateX(100px) translateY(50px) rotate(45deg)');
        expect(result.opacity).toBe('0.5');
    });
    it('handles percentage values', () => {
        const result = buildProperties({ x: '50%' });
        expect(result.transform).toBe('translateX(50%)');
    });
    it('handles scale shorthand', () => {
        const result = buildProperties({ scale: 1.5 });
        expect(result.transform).toContain('scale(1.5)');
    });
    it('maps translateX/Y/Z to CSS transform functions with px units', () => {
        const result = buildProperties({ translateX: 100, translateY: 50, translateZ: 10 });
        expect(result.transform).toBe('translateX(100px) translateY(50px) translateZ(10px)');
    });
    it('handles percentage strings for translateX/Y', () => {
        const result = buildProperties({ translateX: '50%', translateY: '-25%' });
        expect(result.transform).toBe('translateX(50%) translateY(-25%)');
    });
    it('mixes semantic translate names with legacy x/y aliases', () => {
        const result = buildProperties({ translateX: 10, y: 20 });
        expect(result.transform).toBe('translateX(10px) translateY(20px)');
    });
    it('coerces translateZ string values to px (length-only property)', () => {
        // translateZ accepts lengths only — % strings are coerced via parseFloat,
        // numeric strings keep their value in px.
        const pct = buildProperties({ translateZ: '50%' });
        expect(pct.transform).toBe('translateZ(50px)');
        const str = buildProperties({ translateZ: '10' });
        expect(str.transform).toBe('translateZ(10px)');
    });
});
describe('buildKeyframes', () => {
    it('builds WAAPI-compatible keyframes', () => {
        const kf = buildKeyframes([
            { offset: 0, properties: { opacity: 0 } },
            { offset: 1, properties: { opacity: 1 } },
        ]);
        expect(kf).toHaveLength(2);
        expect(kf[0].opacity).toBe('0');
        expect(kf[1].opacity).toBe('1');
    });
});
