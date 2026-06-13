import { chromium } from 'playwright';
import * as fs from 'fs';
import * as path from 'path';
import { execSync, spawn } from 'child_process';

const EDITOR_URL = 'http://localhost:5174';
const REACT_PORT = 5201;

async function main() {
  const browser = await chromium.launch({ headless: true });

  // ═════════════════════════════════════════════════════
  // Step 1: Load KanbanBoard in visual editor
  // ═════════════════════════════════════════════════════
  console.log('Step 1: Loading KanbanBoard in visual editor...');
  const editorPage = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await editorPage.goto(EDITOR_URL, { waitUntil: 'networkidle' });
  await editorPage.waitForFunction(() => window.__OMNI__);

  // Read KanbanBoard.omni source
  const omniSource = fs.readFileSync(
    'D:/space/OmniLang/tests/frameworks/behavior/07_kanban_board.omni',
    'utf-8'
  );

  // Load and compile via OmniDriver
  const compileResult = await editorPage.evaluate(async (source) => {
    const O = window.__OMNI__;
    O.help();
    O.switchMode('code');
    O.setSourceCode(source);
    const result = await O.compile();
    O.switchMode('design');
    return {
      success: result.success,
      errors: result.errors?.length || 0,
      elements: O.elements.length,
      wires: O.wires.length
    };
  }, omniSource);

  console.log('  Compile: ' + (compileResult.success ? 'OK' : 'FAIL') + ', errors: ' + compileResult.errors);
  console.log('  Canvas: ' + compileResult.elements + ' elements, ' + compileResult.wires + ' wires');

  await editorPage.waitForTimeout(1000);
  await editorPage.screenshot({ path: 'D:/space/OmniLang/compare-editor.png', fullPage: false });
  console.log('  Screenshot: compare-editor.png');

  // Get canvas elements detail
  const editorSnapshot = await editorPage.evaluate(() => {
    const O = window.__OMNI__;
    const state = O.getState();
    const views = state.elements.filter(e => e.nodeType === 'view');
    const topLevel = views.filter(e => !e.parentId || e.parentId === '');
    const codeblocks = state.elements.filter(e => e.nodeType === 'codeblock');
    return {
      total: state.elements.length,
      views: views.length,
      codeblocks: codeblocks.length,
      wires: state.wires.length,
      topTags: topLevel.map(e => e.tag + (e.props?.label ? ':"' + e.props.label + '"' : '')),
      allTags: [...new Set(views.map(e => e.tag))].sort()
    };
  });
  console.log('  Editor elements: ' + JSON.stringify(editorSnapshot, null, '    '));

  // ═════════════════════════════════════════════════════
  // Step 2: Compile to React via API
  // ═════════════════════════════════════════════════════
  console.log('\nStep 2: Compiling to React...');

  const compileResponse = await editorPage.evaluate(async (source) => {
    const resp = await fetch('/api/compile', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ source, target: 'react' })
    });
    return resp.json();
  }, omniSource);

  console.log('  API compile: ' + (compileResponse.success ? 'OK' : 'FAIL'));
  console.log('  Files: ' + Object.keys(compileResponse.files || {}).join(', '));

  // Write React files
  const reactDir = 'D:/space/OmniLang/build_output/kanban-compare';
  fs.mkdirSync(reactDir, { recursive: true });

  for (const [fname, content] of Object.entries(compileResponse.files || {})) {
    fs.writeFileSync(path.join(reactDir, fname), content);
    console.log('  Wrote ' + fname + ' (' + content.length + ' chars)');
  }

  // Scaffold React project files
  fs.writeFileSync(path.join(reactDir, 'index.html'),
    '<!DOCTYPE html>\n<html lang="en">\n<head><meta charset="UTF-8"><title>Kanban React</title></head>\n<body><div id="root"></div><script type="module" src="/main.jsx"></script></body>\n</html>');

  fs.writeFileSync(path.join(reactDir, 'main.jsx'),
    "import React from 'react'\nimport ReactDOM from 'react-dom/client'\nimport { KanbanBoard } from './KanbanBoard'\n\nReactDOM.createRoot(document.getElementById('root')).render(\n  React.createElement(React.StrictMode, null, React.createElement(KanbanBoard))\n)");

  fs.writeFileSync(path.join(reactDir, 'vite.config.js'),
    "import { defineConfig } from 'vite'\nimport react from '@vitejs/plugin-react'\nexport default defineConfig({ plugins: [react()], server: { port: " + REACT_PORT + " } })");

  fs.writeFileSync(path.join(reactDir, 'package.json'), JSON.stringify({
    name: "kanban-compare", private: true, version: "1.0.0", type: "module",
    dependencies: { "react": "^18.2.0", "react-dom": "^18.2.0", "zustand": "^4.5.0" },
    devDependencies: { "@vitejs/plugin-react": "^4.2.0", "vite": "^5.4.0" }
  }, null, 2));

  // Install deps
  if (!fs.existsSync(path.join(reactDir, 'node_modules'))) {
    console.log('  Installing dependencies...');
    execSync('npm install', { cwd: reactDir, stdio: 'pipe' });
    console.log('  Done');
  }

  // ═════════════════════════════════════════════════════
  // Step 3: Start React app and screenshot
  // ═════════════════════════════════════════════════════
  console.log('\nStep 3: Starting React app...');
  const viteProc = spawn('npx', ['vite', '--port', String(REACT_PORT), '--host'], {
    cwd: reactDir, stdio: 'pipe', shell: true,
  });

  await new Promise(r => setTimeout(r, 4000));
  console.log('  Vite started on port ' + REACT_PORT);

  // ═════════════════════════════════════════════════════
  // Step 4: Capture React app
  // ═════════════════════════════════════════════════════
  console.log('\nStep 4: Capturing React app...');
  const reactPage = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await reactPage.goto('http://localhost:' + REACT_PORT, { waitUntil: 'networkidle', timeout: 10000 });
  await reactPage.waitForTimeout(1000);

  const reactSnapshot = await reactPage.evaluate(() => {
    const h1 = document.querySelector('h1');
    const buttons = document.querySelectorAll('button');
    const inputs = document.querySelectorAll('input');
    const testIds = document.querySelectorAll('[data-testid]');
    return {
      title: h1?.textContent || '',
      buttonCount: buttons.length,
      inputCount: inputs.length,
      buttons: Array.from(buttons).map(b => b.textContent?.trim().substring(0, 20)),
      inputs: Array.from(inputs).map(i => i.getAttribute('placeholder')),
      testIdCount: testIds.length,
      testIds: Array.from(testIds).map(e => e.getAttribute('data-testid'))
    };
  });
  console.log('  React app: ' + JSON.stringify(reactSnapshot, null, '    '));

  await reactPage.screenshot({ path: 'D:/space/OmniLang/compare-react.png', fullPage: true });
  console.log('  Screenshot: compare-react.png');

  // ═════════════════════════════════════════════════════
  // Step 5: Compare
  // ═════════════════════════════════════════════════════
  console.log('\n' + '='.repeat(60));
  console.log('COMPARISON: Visual Editor vs React Runtime');
  console.log('='.repeat(60));
  console.log('');
  console.log('VISUAL EDITOR (Design Mode Canvas):');
  console.log('  Elements: ' + editorSnapshot.total + ' (' + editorSnapshot.views + ' views + ' + editorSnapshot.codeblocks + ' codeblocks)');
  console.log('  Wires:    ' + editorSnapshot.wires);
  console.log('  View tags: ' + editorSnapshot.allTags.join(', '));
  console.log('');
  console.log('REACT RUNTIME (Browser App):');
  console.log('  Title:    "' + reactSnapshot.title + '"');
  console.log('  Buttons:  ' + reactSnapshot.buttonCount);
  console.log('  Inputs:   ' + reactSnapshot.inputCount);
  console.log('  Test IDs: ' + reactSnapshot.testIdCount);
  console.log('  Buttons:  ' + JSON.stringify(reactSnapshot.buttons));
  console.log('  Inputs:   ' + JSON.stringify(reactSnapshot.inputs));
  console.log('');
  console.log('='.repeat(60));
  console.log('Screenshots: compare-editor.png | compare-react.png');
  console.log('='.repeat(60));

  // Cleanup
  viteProc.kill();
  await browser.close();
  console.log('\nDone!');
}

main().catch(e => { console.error(e); process.exit(1); });
