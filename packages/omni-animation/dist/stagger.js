/**
 * Compute stagger delay array for N items.
 * Direction controls the order: 'start' (first→last), 'end' (last→first),
 * 'center' (middle→outward), 'edges' (edges→inward).
 */
export function staggerDelays(count, each, from = 'start') {
    const indices = [];
    if (from === 'start') {
        for (let i = 0; i < count; i++)
            indices.push(i);
    }
    else if (from === 'end') {
        for (let i = count - 1; i >= 0; i--)
            indices.push(i);
    }
    else if (from === 'edges') {
        let l = 0, r = count - 1;
        while (l <= r) {
            indices.push(l++);
            if (l <= r)
                indices.push(r--);
        }
    }
    else { // center
        const mid = Math.floor((count - 1) / 2);
        indices.push(mid);
        let l = mid - 1, r = mid + 1;
        while (l >= 0 || r < count) {
            if (l >= 0)
                indices.push(l--);
            if (r < count)
                indices.push(r++);
        }
    }
    const delays = new Array(count).fill(0);
    indices.forEach((origIdx, pos) => { delays[origIdx] = pos * each; });
    return delays;
}
