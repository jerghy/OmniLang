// Simple bundler: concatenate TypeScript-compiled modules into one IIFE
// that exposes window.Omni for <script> tag usage (OmniView).
const fs = require('fs')
const path = require('path')

const files = ['types.js', 'easing.js', 'keyframes.js', 'animate.js',
  'timeline.js', 'stagger.js', 'scroll.js', 'flip.js', 'hero.js']

let bundle = '(function(){\n"use strict";\n\n'
for (const f of files) {
  const fp = path.join('dist', f)
  if (!fs.existsSync(fp)) { console.error('Missing: ' + fp); process.exit(1); }
  let content = fs.readFileSync(fp, 'utf-8')
  // Strip import/export statements
  content = content.replace(/^import .+$/gm, '').replace(/^export /gm, '')
  // Remove "use strict" since we have it at the top
  content = content.replace(/^"use strict";?\n/gm, '')
  bundle += '// ' + f + '\n' + content + '\n'
}

// Attach to global
bundle += `
const Omni = { animate, timeline, buildKeyframes, buildProperties, resolveEasing, staggerDelays, scrollProgress, flipBefore, flipAfter, heroBefore, heroAfter };
window.Omni = Omni;
})();
`

fs.writeFileSync('dist/omni-animation.js', bundle)
fs.writeFileSync('dist/omni-animation.mjs', bundle)
console.log('Bundled omni-animation.js (' + bundle.length + ' bytes)')
