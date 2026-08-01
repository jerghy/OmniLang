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

describe('resolveEasing kebab-case aliases', () => {
  // Semantic DSL compiles easing names to canonical kebab-case
  // ("power2-out") which the runtime must resolve to the same value
  // as the camelCase form ("power2Out").
  const kebabPairs: [string, string][] = [
    ['ease-in', 'easeIn'],
    ['ease-out', 'easeOut'],
    ['ease-in-out', 'easeInOut'],
    ['power1-in', 'power1In'],
    ['power1-out', 'power1Out'],
    ['power1-in-out', 'power1InOut'],
    ['power2-in', 'power2In'],
    ['power2-out', 'power2Out'],
    ['power2-in-out', 'power2InOut'],
    ['power3-in', 'power3In'],
    ['power3-out', 'power3Out'],
    ['power3-in-out', 'power3InOut'],
    ['power4-in', 'power4In'],
    ['power4-out', 'power4Out'],
    ['power4-in-out', 'power4InOut'],
    ['back-in', 'backIn'],
    ['back-out', 'backOut'],
    ['back-in-out', 'backInOut'],
    ['bounce-in', 'bounceIn'],
    ['bounce-out', 'bounceOut'],
    ['bounce-in-out', 'bounceInOut'],
    ['elastic-in', 'elasticIn'],
    ['elastic-out', 'elasticOut'],
    ['elastic-in-out', 'elasticInOut'],
    ['step-start', 'stepStart'],
    ['step-end', 'stepEnd'],
  ];

  it.each(kebabPairs)('resolves kebab alias "%s" identically to "%s"', (kebab, camel) => {
    expect(resolveEasing(kebab)).toBe(resolveEasing(camel));
  });

  it('resolves kebab power easings to their exact cubic-bezier (no linear fallback)', () => {
    expect(resolveEasing('power2-out')).toBe('cubic-bezier(0.215,0.610,0.355,1.000)');
    expect(resolveEasing('power4-in')).toBe('cubic-bezier(0.755,0.050,0.855,0.060)');
  });

  it('resolves kebab keywords and steps to CSS-native values', () => {
    expect(resolveEasing('ease-in')).toBe('ease-in');
    expect(resolveEasing('step-start')).toBe('steps(1,start)');
    expect(resolveEasing('linear')).toBe('linear');
  });
});
