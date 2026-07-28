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
