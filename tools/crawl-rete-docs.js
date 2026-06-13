/**
 * Crawls Rete.js documentation (https://retejs.org/docs/) and saves
 * each page as a local Markdown file.
 *
 * Usage: node tools/crawl-rete-docs.js
 * Output: docs/rete-docs/
 */

const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const BASE_URL = 'https://retejs.org';
const OUTPUT_DIR = path.join(__dirname, '..', 'docs', 'rete-docs');

// All unique doc pages extracted from the sidebar navigation
const PAGES = [
  { href: '/docs', title: 'Introduction' },
  { href: '/docs/getting-started', title: 'Getting Started' },
  { href: '/docs/concepts/plugin-system', title: 'Plugin System' },
  { href: '/docs/concepts/presets', title: 'Presets' },
  { href: '/docs/concepts/editor', title: 'Editor' },
  { href: '/docs/concepts/engine', title: 'Engine' },
  { href: '/docs/concepts/integration', title: 'Integration' },
  { href: '/docs/guides/basic', title: 'Basic Editor Guide' },
  { href: '/docs/guides/renderers/react', title: 'React Renderer' },
  { href: '/docs/guides/renderers/vue', title: 'Vue Renderer' },
  { href: '/docs/guides/renderers/angular', title: 'Angular Renderer' },
  { href: '/docs/guides/renderers/svelte', title: 'Svelte Renderer' },
  { href: '/docs/guides/renderers/lit', title: 'Lit Renderer' },
  { href: '/docs/guides/processing/dataflow', title: 'Dataflow Processing' },
  { href: '/docs/guides/processing/control-flow', title: 'Control Flow Processing' },
  { href: '/docs/guides/processing/hybrid', title: 'Hybrid Engine' },
  { href: '/docs/guides/processing/codegen', title: 'Codegen' },
  { href: '/docs/guides/3d', title: '3D Editor' },
  { href: '/docs/guides/data-structures', title: 'Data Structures' },
  { href: '/docs/guides/arrange', title: 'Arrange Nodes' },
  { href: '/docs/guides/selectable', title: 'Selectable' },
  { href: '/docs/guides/selectable/connections', title: 'Selectable Connections' },
  { href: '/docs/guides/connections', title: 'Connections' },
  { href: '/docs/guides/context-menu', title: 'Context Menu' },
  { href: '/docs/guides/readonly', title: 'Readonly' },
  { href: '/docs/guides/modules', title: 'Modules' },
  { href: '/docs/guides/scopes', title: 'Scopes' },
  { href: '/docs/guides/import-export', title: 'Import/Export' },
  { href: '/docs/guides/validation', title: 'Validation' },
  { href: '/docs/guides/minimap', title: 'Minimap' },
  { href: '/docs/guides/dock-menu', title: 'Dock Menu' },
  { href: '/docs/guides/connection-path', title: 'Connection Path' },
  { href: '/docs/guides/reroute', title: 'Reroute' },
  { href: '/docs/guides/undo-redo', title: 'Undo/Redo' },
  { href: '/docs/guides/comments', title: 'Comments' },
  { href: '/docs/best-practices/performance', title: 'Performance Best Practices' },
  { href: '/docs/quality-assurance', title: 'Quality Assurance' },
  { href: '/docs/development', title: 'Development' },
  { href: '/docs/development/rete-cli', title: 'Rete CLI' },
  { href: '/docs/development/rete-kit', title: 'Rete Kit' },
  { href: '/docs/development/ai-assistance', title: 'AI Assistance' },
  { href: '/docs/development/ai-assistance/llms', title: 'LLMs.txt' },
  { href: '/docs/development/ai-assistance/rete-kit-ai', title: 'Rete Kit AI' },
  { href: '/docs/troubleshooting', title: 'Troubleshooting' },
  { href: '/docs/licensing', title: 'Licensing' },
  { href: '/docs/code-of-conduct', title: 'Code of Conduct' },
  { href: '/docs/contribution', title: 'Contribution' },
  { href: '/docs/api', title: 'API Reference' },
  { href: '/docs/api/overview', title: 'API Overview' },
  { href: '/docs/api/rete', title: 'API: rete' },
  { href: '/docs/api/rete-area-plugin', title: 'API: rete-area-plugin' },
  { href: '/docs/api/rete-area-3d-plugin', title: 'API: rete-area-3d-plugin' },
  { href: '/docs/api/rete-connection-plugin', title: 'API: rete-connection-plugin' },
  { href: '/docs/api/rete-auto-arrange-plugin', title: 'API: rete-auto-arrange-plugin' },
  { href: '/docs/api/rete-context-menu-plugin', title: 'API: rete-context-menu-plugin' },
  { href: '/docs/api/rete-engine', title: 'API: rete-engine' },
  { href: '/docs/api/rete-history-plugin', title: 'API: rete-history-plugin' },
  { href: '/docs/api/rete-minimap-plugin', title: 'API: rete-minimap-plugin' },
  { href: '/docs/api/rete-readonly-plugin', title: 'API: rete-readonly-plugin' },
  { href: '/docs/api/rete-angular-plugin', title: 'API: rete-angular-plugin' },
  { href: '/docs/api/rete-lit-plugin', title: 'API: @retejs/lit-plugin' },
  { href: '/docs/api/rete-react-plugin', title: 'API: rete-react-plugin' },
  { href: '/docs/api/rete-svelte-plugin', title: 'API: rete-svelte-plugin' },
  { href: '/docs/api/rete-vue-plugin', title: 'API: rete-vue-plugin' },
  { href: '/docs/api/rete-render-utils', title: 'API: rete-render-utils' },
  { href: '/docs/api/rete-scopes-plugin', title: 'API: rete-scopes-plugin' },
  { href: '/docs/api/rete-dock-plugin', title: 'API: rete-dock-plugin' },
  { href: '/docs/api/rete-comment-plugin', title: 'API: rete-comment-plugin' },
  { href: '/docs/api/rete-connection-path-plugin', title: 'API: rete-connection-path-plugin' },
  { href: '/docs/api/rete-connection-reroute-plugin', title: 'API: rete-connection-reroute-plugin' },
  { href: '/docs/faq', title: 'FAQ' },
  { href: '/docs/migration', title: 'Migration' },
];

function ensureDir(dir) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

function sanitizeFilename(str) {
  return str.replace(/[^a-zA-Z0-9\-_]/g, '_').toLowerCase();
}

async function extractContent(page) {
  // Wait for the Nuxt SPA to fully render the content
  // The key content element is div.content.markdown
  try {
    await page.waitForSelector('.content.markdown', { timeout: 15000 });
  } catch {
    // Try alternative: wait for network idle
    await page.waitForLoadState('networkidle', { timeout: 15000 }).catch(() => {});
    await page.waitForTimeout(3000);
  }

  // Extra wait for any remaining JS rendering
  await page.waitForTimeout(500);

  // Extract the main article content
  const result = await page.evaluate(() => {
    // Primary target: the markdown content area
    let contentEl = document.querySelector('.content.markdown');
    if (!contentEl) {
      // Fallback: try to find any div with markdown class
      contentEl = document.querySelector('[class*="markdown"]');
    }
    if (!contentEl) {
      contentEl = document.querySelector('.docs > div:last-child');
    }

    if (!contentEl) return { title: '', html: '' };

    // Get page title from h1
    const h1 = contentEl.querySelector('h1');
    const title = h1 ? h1.textContent.trim() : document.title.replace(' - Rete.js', '').trim();

    // Clone to avoid mutations affecting the live DOM
    const clone = contentEl.cloneNode(true);

    // Remove elements that shouldn't be in the output
    const removables = clone.querySelectorAll(
      'nav, script, style, iframe, .share-buttons, ' +
      '[class*="pagination"], .next-prev, .edit-link, ' +
      '.copy-button, [class*="toc"], .table-of-contents'
    );
    removables.forEach(n => n.remove());

    // Build a simplified markdown-like string by walking the DOM
    function elementToMarkdown(el) {
      const tag = el.tagName?.toLowerCase();
      const children = Array.from(el.childNodes);

      // Process headings
      if (/^h[1-6]$/.test(tag)) {
        const level = parseInt(tag[1]);
        return '\n' + '#'.repeat(level) + ' ' + el.textContent.trim() + '\n\n';
      }

      // Process paragraphs
      if (tag === 'p') {
        return '\n' + getRichText(el) + '\n\n';
      }

      // Process code blocks
      if (tag === 'pre') {
        const code = el.querySelector('code');
        const lang = code?.className?.match(/language-(\w+)/)?.[1] || '';
        const text = el.textContent;
        return '\n```' + lang + '\n' + text + '\n```\n\n';
      }

      // Process unordered lists
      if (tag === 'ul') {
        const items = Array.from(el.children).filter(c => c.tagName === 'LI');
        return '\n' + items.map(li => '- ' + getRichText(li)).join('\n') + '\n\n';
      }

      // Process ordered lists
      if (tag === 'ol') {
        const items = Array.from(el.children).filter(c => c.tagName === 'LI');
        return '\n' + items.map((li, i) => (i + 1) + '. ' + getRichText(li)).join('\n') + '\n\n';
      }

      // Process blockquotes
      if (tag === 'blockquote') {
        const lines = el.textContent.trim().split('\n');
        return '\n' + lines.map(l => '> ' + l).join('\n') + '\n\n';
      }

      // Process tables
      if (tag === 'table') {
        const rows = Array.from(el.querySelectorAll('tr'));
        if (rows.length === 0) return '';
        let md = '\n';
        const firstRowCells = rows[0].querySelectorAll('th, td');
        if (firstRowCells.length > 0) {
          md += '| ' + Array.from(firstRowCells).map(c => c.textContent.trim()).join(' | ') + ' |\n';
          const hasHeader = rows[0].querySelectorAll('th').length > 0;
          if (hasHeader) {
            md += '| ' + Array(firstRowCells.length).fill('---').join(' | ') + ' |\n';
            for (let i = 1; i < rows.length; i++) {
              const cells = rows[i].querySelectorAll('td');
              md += '| ' + Array.from(cells).map(c => c.textContent.trim()).join(' | ') + ' |\n';
            }
          } else {
            md += '| ' + Array(firstRowCells.length).fill('---').join(' | ') + ' |\n';
            for (let i = 1; i < rows.length; i++) {
              const cells = rows[i].querySelectorAll('td');
              md += '| ' + Array.from(cells).map(c => c.textContent.trim()).join(' | ') + ' |\n';
            }
          }
        }
        return md + '\n';
      }

      // Process horizontal rules
      if (tag === 'hr') {
        return '\n---\n\n';
      }

      // Process divs and sections by recursing into children
      if (tag === 'div' || tag === 'section' || tag === 'article' || tag === 'main') {
        return children.map(c => elementToMarkdown(c)).join('');
      }

      // For unknown elements, recurse into children
      if (el.nodeType === 1 && children.length > 0) {
        return children.map(c => elementToMarkdown(c)).join('');
      }

      // Text nodes
      if (el.nodeType === 3) {
        return el.textContent;
      }

      return '';
    }

    function getRichText(el) {
      let result = '';
      for (const child of el.childNodes) {
        if (child.nodeType === 3) {
          result += child.textContent;
        } else if (child.nodeType === 1) {
          const tag = child.tagName?.toLowerCase();
          if (tag === 'strong' || tag === 'b') {
            result += '**' + child.textContent + '**';
          } else if (tag === 'em' || tag === 'i') {
            result += '*' + child.textContent + '*';
          } else if (tag === 'code') {
            result += '`' + child.textContent + '`';
          } else if (tag === 'a') {
            const href = child.getAttribute('href') || '';
            if (href.startsWith('http')) {
              result += '[' + child.textContent + '](' + href + ')';
            } else {
              result += child.textContent;
            }
          } else if (tag === 'img') {
            const alt = child.getAttribute('alt') || '';
            const src = child.getAttribute('src') || '';
            result += '![' + alt + '](' + src + ')';
          } else {
            result += getRichText(child);
          }
        }
      }
      return result;
    }

    // Walk all top-level children of the content element
    let html = '';
    for (const child of clone.childNodes) {
      html += elementToMarkdown(child);
    }

    // Clean up
    html = html
      .replace(/\n{3,}/g, '\n\n')
      .replace(/[ \t]+$/gm, '')
      .trim();

    return { title, html };
  });

  return result;
}

async function main() {
  console.log('Launching browser...');
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'
  });

  const page = await context.newPage();
  const outputDir = OUTPUT_DIR;
  ensureDir(outputDir);

  let successCount = 0;
  let failCount = 0;

  // Create index file
  let indexContent = '# Rete.js Documentation (Local Copy)\n\n';
  indexContent += `> Downloaded from https://retejs.org/docs/\n\n`;
  indexContent += '## Table of Contents\n\n';

  // Group pages by section
  const sections = {};
  for (const doc of PAGES) {
    const parts = doc.href.replace('/docs/', '').split('/');
    let section = 'General';
    if (parts.length > 1) {
      section = parts[0].charAt(0).toUpperCase() + parts[0].slice(1);
    }
    if (!sections[section]) sections[section] = [];
    sections[section].push(doc);
  }

  // Write section headers to index
  for (const [section, docs] of Object.entries(sections)) {
    indexContent += `\n### ${section}\n\n`;
    for (const doc of docs) {
      const filename = sanitizeFilename(doc.href.replace('/docs/', '').replace(/^\//, '') || 'index') + '.md';
      indexContent += `- [${doc.title}](${filename})\n`;
    }
  }

  // Process each page
  for (let i = 0; i < PAGES.length; i++) {
    const doc = PAGES[i];
    const url = BASE_URL + doc.href;
    const filename = sanitizeFilename(doc.href.replace('/docs/', '').replace(/^\//, '') || 'index') + '.md';
    const filepath = path.join(outputDir, filename);

    console.log(`[${i + 1}/${PAGES.length}] Fetching: ${doc.href} (${doc.title})`);

    try {
      // Try networkidle first, fall back to domcontentloaded + wait for pages with persistent network activity (e.g., demos, iframes)
      try {
        await page.goto(url, { waitUntil: 'networkidle', timeout: 15000 });
      } catch {
        console.log('  ⏳ networkidle timed out, falling back to domcontentloaded + wait');
        await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 15000 });
        // Wait for SPA to render
        await page.waitForTimeout(5000);
      }

      const { title, html } = await extractContent(page);

      if (!html || html.trim().length < 50) {
        console.log(`  ⚠️  Content too short, may have failed to extract`);
        failCount++;
        continue;
      }

      // Build final markdown
      const pageTitle = title || doc.title;
      const markdown = `# ${pageTitle}\n\n> Source: [${url}](${url})\n\n${html}\n`;

      fs.writeFileSync(filepath, markdown, 'utf-8');
      console.log(`  ✅ Saved ${filename} (${html.length} chars)`);
      successCount++;
    } catch (err) {
      console.log(`  ❌ Failed: ${err.message}`);
      failCount++;
    }
  }

  // Write index
  fs.writeFileSync(path.join(outputDir, 'README.md'), indexContent, 'utf-8');
  console.log(`\n✅ Index saved to README.md`);

  await browser.close();

  console.log(`\n=== Summary ===`);
  console.log(`  Success: ${successCount}`);
  console.log(`  Failed: ${failCount}`);
  console.log(`  Output: ${outputDir}`);
}

main().catch(err => {
  console.error('Fatal error:', err);
  process.exit(1);
});
