const esbuild = require('esbuild')
const fs = require('fs')
const path = require('path')

/** Bundle the animation library to IIFE format using esbuild. */
async function build() {
  // Bundle to IIFE for <script> tag usage (OmniView)
  await esbuild.build({
    entryPoints: ['src/index.ts'],
    outfile: 'dist/omni-animation.js',
    format: 'iife',
    globalName: 'Omni',
    bundle: true,
    sourcemap: false,
    minify: false,
    target: 'es2020',
    tsconfig: 'tsconfig.json',
    logLevel: 'info',
  })

  // Also write .mjs with the same content (matching previous approach)
  fs.copyFileSync('dist/omni-animation.js', 'dist/omni-animation.mjs')

  const bytes = fs.statSync('dist/omni-animation.js').size
  console.log('Bundled omni-animation.js (' + bytes + ' bytes)')
}

build().catch(() => process.exit(1))
