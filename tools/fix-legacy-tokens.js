const fs = require('fs');
const path = require('path');

function findFiles(dir) {
  const results = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) results.push(...findFiles(full));
    else if (e.name.endsWith('.module.css')) results.push(full.replace(/\\/g, '/'));
  }
  return results;
}

const repl = [
  ['var(--dp-border, #e8e5f2)', 'var(--ol-border)'],
  ['var(--dp-text-muted, #a09db8)', 'var(--ol-text-muted)'],
  ['var(--dp-text-secondary, #6b6893)', 'var(--ol-text-secondary)'],
  ['var(--dp-text, #1e1b4b)', 'var(--ol-text)'],
  ['var(--dp-bg, #f7f6fa)', 'var(--ol-surface-hover)'],
  ['var(--dp-surface-hover, #f5f3ff)', 'var(--ol-surface-hover)'],
  ['var(--dp-accent, #6366f1)', 'var(--ol-color-primary)'],
  ['var(--dp-accent-subtle, #eef0ff)', 'var(--ol-color-primary-subtle)'],
];

// Also fix --color-* legacy tokens
const colorLegacy = [
  ['var(--color-primary)', 'var(--ol-color-primary)'],
  ['var(--color-surface)', 'var(--ol-surface)'],
  ['var(--color-surface-hover)', 'var(--ol-surface-hover)'],
  ['var(--color-border)', 'var(--ol-border)'],
  ['var(--color-text)', 'var(--ol-text)'],
  ['var(--color-text-secondary)', 'var(--ol-text-secondary)'],
  ['var(--color-text-muted)', 'var(--ol-text-muted)'],
  ['var(--color-danger)', 'var(--ol-color-danger)'],
  ['var(--color-danger-light)', 'var(--ol-color-danger-subtle)'],
  ['var(--color-success)', 'var(--ol-color-success)'],
  ['var(--color-warning)', 'var(--ol-color-warning)'],
  ['var(--color-info)', 'var(--ol-color-info)'],
];

const files = findFiles('poiesis/src');
let total = 0;

for (const file of files) {
  let content = fs.readFileSync(file, 'utf8');
  const original = content;

  for (const [from, to] of repl) {
    content = content.split(from).join(to);
  }

  for (const [from, to] of colorLegacy) {
    content = content.split(from).join(to);
  }

  // Fix double-wrapped var(var(--ol patterns
  content = content.split('var(var(--ol-').join('var(--ol-');

  if (content !== original) {
    fs.writeFileSync(file, content, 'utf8');
    total++;
    console.log('OK:', path.relative('poiesis/src', file));
  }
}

console.log('\nTotal updated:', total);
