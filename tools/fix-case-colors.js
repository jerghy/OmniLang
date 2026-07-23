const fs = require('fs');
const path = require('path');

function findFiles(dir, exts) {
  const r = [];
  for (const e of fs.readdirSync(dir, {withFileTypes: true})) {
    const f = path.join(dir, e.name).replace(/\\/g, '/');
    if (e.isDirectory()) {
      if (e.name !== 'node_modules') r.push(...findFiles(f, exts));
    } else if (exts.includes(path.extname(e.name))) {
      if (!f.includes('tokens.css') && !f.includes('legacy-bridge') && !f.includes('.d.ts'))
        r.push(f);
    }
  }
  return r;
}

const CASE_REPL = [
  ['#94A3B8', 'var(--ol-text-muted)'],
  ['#9CA3AF', 'var(--ol-text-muted)'],
  ['#FFFBEB', 'var(--ol-color-warning-subtle)'],
  ['#F59E0B', 'var(--ol-color-warning)'],
  ['#92400E', 'var(--ol-color-warning-active)'],
  ['#FDE68A', 'var(--ol-color-warning-muted)'],
  ['#D97706', 'var(--ol-color-warning)'],
  ['#DC2626', 'var(--ol-color-danger)'],
  ['#059669', 'var(--ol-color-success-active)'],
  ['#1F2937', 'var(--ol-gray-800)'],
  ['#E5E7EB', 'var(--ol-border)'],
  ['#F3F4F6', 'var(--ol-surface-hover)'],
  ['#4F46E5', 'var(--ol-color-primary-hover)'],
  ['#7C3AED', 'var(--ol-color-primary)'],
  ['#6D28D9', 'var(--ol-color-primary)'],
  ['#5b21b6', 'var(--ol-color-primary-active)'],
  ['#000000', 'var(--ol-gray-950)'],
];

let t = 0;
for (const file of findFiles('poiesis/src', ['.tsx', '.ts', '.css'])) {
  let c = fs.readFileSync(file, 'utf8');
  const o = c;
  for (const [hex, tok] of CASE_REPL) {
    c = c.split(hex).join(tok);
  }
  if (c !== o) {
    fs.writeFileSync(file, c, 'utf8');
    t++;
    console.log(path.relative('poiesis/src', file));
  }
}
console.log('---');
console.log('Fixed:', t);
