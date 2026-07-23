const fs = require('fs');
const path = require('path');

function findFiles(dir, exts) {
  const results = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) {
      if (e.name === 'node_modules') continue;
      results.push(...findFiles(full, exts));
    } else {
      const ext = path.extname(e.name);
      if (exts.includes(ext)) {
        results.push(full.replace(/\\/g, '/'));
      }
    }
  }
  return results;
}

// Skip files that should not be modified
const skipFiles = [
  'tokens.css', 'legacy-bridge.css',
];

const replacements = [
  // border colors
  ['#e2e8f0', 'var(--ol-border)'],
  ['#cbd5e1', 'var(--ol-border-hover)'],

  // surface/background
  ['#ffffff', 'var(--ol-surface)'],
  ['#f8fafc', 'var(--ol-gray-50)'],
  ['#f1f5f9', 'var(--ol-gray-100)'],
  ['#f3f4f6', 'var(--ol-surface-hover)'],
  ['#f9fafb', 'var(--ol-gray-50)'],

  // text colors
  ['#94a3b8', 'var(--ol-text-muted)'],
  ['#9ca3af', 'var(--ol-text-muted)'],
  ['#64748b', 'var(--ol-text-secondary)'],
  ['#6b7280', 'var(--ol-text-secondary)'],
  ['#475569', 'var(--ol-text-secondary)'],
  ['#4b5563', 'var(--ol-text-secondary)'],
  ['#374151', 'var(--ol-gray-700)'],
  ['#334155', 'var(--ol-gray-700)'],
  ['#1e293b', 'var(--ol-gray-800)'],
  ['#1f2937', 'var(--ol-gray-800)'],

  // primary
  ['#4f46e5', 'var(--ol-color-primary-hover)'],
  ['#6366f1', 'var(--ol-color-primary)'],
  ['#4338ca', 'var(--ol-color-primary-active)'],
  ['#eef2ff', 'var(--ol-color-primary-subtle)'],
  ['#eef0ff', 'var(--ol-color-primary-subtle)'],
  ['#c7d2fe', 'var(--ol-color-primary-muted)'],

  // danger
  ['#dc2626', 'var(--ol-color-danger)'],
  ['#b91c1c', 'var(--ol-color-danger-hover)'],
  ['#fecaca', 'var(--ol-color-danger-muted)'],

  // warning/success/info/nonstandard
  ['#f59e0b', 'var(--ol-color-warning)'],
  ['#d97706', 'var(--ol-color-warning)'],
  ['#b45309', 'var(--ol-color-warning-hover)'],
  ['#d1d5db', 'var(--ol-border-hover)'],
  ['#e5e7eb', 'var(--ol-border)'],
  ['#fef2f2', 'var(--ol-color-danger-subtle)'],
  ['#fca5a5', 'var(--ol-color-danger-muted)'],
  ['#ef4444', 'var(--ol-color-danger)'],
  ['#10b981', 'var(--ol-color-success)'],
  ['#3b82f6', 'var(--ol-color-info)'],
  ['#8b5cf6', 'var(--ol-color-primary)'],
  ['#7c3aed', 'var(--ol-color-primary)'],
  ['#6d28d9', 'var(--ol-color-primary)'],
  ['#e0e7ff', 'var(--ol-color-primary-subtle)'],
  ['#c4b5fd', 'var(--ol-color-primary-muted)'],
  ['#fde68a', 'var(--ol-color-warning-muted)'],
  ['#ecfdf5', 'var(--ol-color-success-subtle)'],
  ['#eff6ff', 'var(--ol-color-info-subtle)'],
  ['#f5f3ff', 'var(--ol-color-primary-subtle)'],
  ['#f0fdf4', 'var(--ol-color-success-subtle)'],
  ['#bbf7d0', 'var(--ol-color-success-muted)'],
  ['#fffbeb', 'var(--ol-color-warning-subtle)'],
  ['#92400e', 'var(--ol-color-warning-active)'],
  ['#065f46', 'var(--ol-color-success-active)'],
  ['#1e40af', 'var(--ol-color-info-active)'],
  ['#0f172a', 'var(--ol-text)'],
  ['#111827', 'var(--ol-text)'],
  ['#166534', 'var(--ol-color-success-active)'],
  ['#15803d', 'var(--ol-color-success-hover)'],
  ['#16a34a', 'var(--ol-color-success)'],
  ['#991b1b', 'var(--ol-color-danger-active)'],
  ['#0284c7', 'var(--ol-color-info-hover)'],
  ['#0369a1', 'var(--ol-color-info-active)'],
  ['#0ea5e9', 'var(--ol-color-info)'],
  ['#f0f9ff', 'var(--ol-color-info-subtle)'],
  ['#bae6fd', 'var(--ol-color-info-muted)'],
  ['#030712', 'var(--ol-gray-950)'],
  ['#bfdbfe', 'var(--ol-color-info-muted)'],
  ['#2563eb', 'var(--ol-color-info)'],
  ['#d4d4d8', 'var(--ol-border-hover)'],
  ['#f0eef7', 'var(--ol-color-primary-subtle)'],
  ['#e8e5f2', 'var(--ol-border)'],
  ['#a09db8', 'var(--ol-text-muted)'],
  ['#6b6893', 'var(--ol-text-secondary)'],
  ['#1e1b4b', 'var(--ol-gray-700)'],
  ['#f7f6fa', 'var(--ol-gray-50)'],

  // 3-digit shortcuts (LAST to avoid partial matches)
  ['"#FFF"', '\"var(--ol-surface)\"'],
  ['"#fff"', '\"var(--ol-surface)\"'],
  ["'#FFF'", "'var(--ol-surface)'"],
  ["'#fff'", "'var(--ol-surface)'"],
  ['#FFF;', 'var(--ol-surface);'],
  ['#fff;', 'var(--ol-surface);'],
  ['#FFF)', 'var(--ol-surface))'],
  ['#fff)', 'var(--ol-surface))'],
  ['#FFF ', 'var(--ol-surface) '],
  ['#fff ', 'var(--ol-surface) '],
  [',#CCC,', ',var(--ol-border-hover),'],
  [',#ccc,', ',var(--ol-border-hover),'],
  ['"#CCC"', '\"var(--ol-border-hover)\"'],
  ["'#CCC'", "'var(--ol-border-hover)'"],
  ['"#ccc"', '\"var(--ol-border-hover)\"'],
  ["'#ccc'", "'var(--ol-border-hover)'"],
  ['#666', 'var(--ol-text-secondary)'],
  ['#999', 'var(--ol-text-muted)'],
  ['#f5f5f5', 'var(--ol-gray-100)'],
  ['#FAFAFA', 'var(--ol-gray-50)'],
  ['#fafafa', 'var(--ol-gray-50)'],

  // Case-insensitive variants
  ['#6366F1', 'var(--ol-color-primary)'],
  ['#3730A3', 'var(--ol-color-primary-active)'],
  ['#3730a3', 'var(--ol-color-primary-active)'],
  ['#818CF8', 'var(--ol-color-primary)'],
  ['#818cf8', 'var(--ol-color-primary)'],
  ['#EDE9FE', 'var(--ol-color-primary-subtle)'],
  ['#ede9fe', 'var(--ol-color-primary-subtle)'],
  ['#F43F5E', 'var(--ol-color-danger)'],
  ['#BE123C', 'var(--ol-color-danger-active)'],
  ['#FFF1F2', 'var(--ol-color-danger-subtle)'],
  ['#6B7280', 'var(--ol-text-secondary)'],
  ['#475569', 'var(--ol-text-secondary)'],
  ['#FFF', 'var(--ol-surface)'],
  ['#fff', 'var(--ol-surface)'],
  ['#CCC', 'var(--ol-border-hover)'],
  ['#ccc', 'var(--ol-border-hover)'],
];

const files = findFiles('poiesis/src', ['.tsx', '.ts', '.css']);
let totalFiles = 0;
let totalReplaces = 0;

for (const file of files) {
  const base = path.basename(file);
  if (skipFiles.includes(base)) continue;
  if (file.includes('legacy-bridge')) continue;

  let content = fs.readFileSync(file, 'utf8');
  const original = content;

  for (const [hex, token] of replacements) {
    // Skip if the file already uses var(--ol- tokens (already processed)
    // But we still need to handle files that have some tokens and some hardcoded
    const escaped = hex.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const regex = new RegExp(escaped + '(?!\\w)', 'g');
    content = content.replace(regex, token);
  }

  if (content !== original) {
    fs.writeFileSync(file, content, 'utf8');
    totalFiles++;
    totalReplaces += (original.match(/#[0-9a-fA-F]{3,6}[^0-9a-fA-F]/g) || []).length;
    console.log(path.relative('poiesis/src', file));
  }
}

console.log('\n---');
console.log('Files modified:', totalFiles);
console.log('Approx replaces:', totalReplaces);

// Clean up any double-wrapped var(var(--ol- patterns
console.log('\nCleaning up double wraps...');
let cleanCount = 0;
for (const file of findFiles('poiesis/src', ['.tsx', '.ts', '.css'])) {
  const base = path.basename(file);
  if (skipFiles.includes(base)) continue;
  let content = fs.readFileSync(file, 'utf8');
  const original = content;
  content = content.split('var(var(--ol-').join('var(--ol-');
  if (content !== original) {
    fs.writeFileSync(file, content, 'utf8');
    cleanCount++;
  }
}
console.log('Double-wrap fixes:', cleanCount);
