/**
 * Compute stagger delay array for N items.
 * Direction controls the order: 'start' (first→last), 'end' (last→first),
 * 'center' (middle→outward), 'edges' (edges→inward).
 */
export declare function staggerDelays(count: number, each: number, from?: 'start' | 'center' | 'end' | 'edges'): number[];
