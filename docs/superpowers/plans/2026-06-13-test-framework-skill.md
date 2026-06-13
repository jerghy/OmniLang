# Test-Framework Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a reusable Claude Code skill (`test-framework`) that executes a 7-stage closed-loop to validate the OmniLang framework by building a real app, transpiling to React, comparing output, and fixing framework bugs.

**Architecture:** SKILL.md defines the AI workflow; `compare.mjs` automates browser/screenshot/transpile steps via Playwright; `helpers/` contains Python transpile script and DOM diff analyzer. The skill uses `omni-driver` skill for editor control, Browser MCP for interaction, and Vision MCP for screenshot comparison.

**Tech Stack:** Node.js (Playwright), Python (omniview API), Bash (Vite server), Claude Code skills framework

---

## File Structure

```
.claude/skills/test-framework/
├── SKILL.md                 # Main skill — 7-stage closed-loop workflow for AI
├── compare.mjs              # Playwright orchestration: editor→transpile→screenshot→compare
└── helpers/
    ├── transpile.py         # Python: call /api/compile → write React files + scaffold Vite
    └── diff-analyzer.mjs    # Structural diff: editor state vs React DOM elements
```

| File | Responsibility |
|------|---------------|
| `SKILL.md` | Define when to use, 7-stage workflow steps, iron rules, red flags |
| `compare.mjs` | Full pipeline script: open editor, load/create app, save .omni+.omni.json, transpile, start Vite, screenshot both, run diff-analyzer |
| `helpers/transpile.py` | Call omniview API to compile .omni → React, write output files, scaffold Vite project |
| `helpers/diff-analyzer.mjs` | Compare editor element inventory vs React DOM, detect missing/mismatched elements |

---

### Task 1: Create Skill Directory and SKILL.md Skeleton

**Files:**
- Create: `.claude/skills/test-framework/SKILL.md`

- [ ] **Step 1: Create directory structure**

```bash
mkdir -p .claude/skills/test-framework/helpers
```

- [ ] **Step 2: Write SKILL.md with frontmatter and 7-stage workflow**

Write `.claude/skills/test-framework/SKILL.md`:

```markdown
---
name: test-framework
description: Use when testing OmniLang framework with real-world apps — build in visual editor, transpile to React, screenshot-compare with Vision MCP, test logic, fix framework code (never generated code), loop until passing, git push. Triggers: "test framework", "validate transpile", "real case test", "framework test"
---

# Test-Framework — OmniLang Framework Validation via Real-World Apps

## Iron Rules (MUST follow)

1. **NEVER modify generated/transpiled code.** Only fix framework code.
2. **Always save BOTH `.omni` AND `.omni.json`** after visual editor implementation.
3. **Always use Progressive Build-Test Loop** (validate every 3-5 elements).
4. **Always run Vision MCP comparison** — never skip visual diff.
5. **Loop until all checks pass** before git push.

## Framework Code Locations (these you CAN modify)

| Path | Responsibility |
|------|---------------|
| `omniview/codegen/react.py` | React code generation |
| `omniview/codegen/css_mapping.py` | CSS/style mapping |
| `omniview/lowering.py` | IR lowering |
| `omniview/parser.py` | .omni parsing |
| `omniview/semantic/` | Semantic analysis |
| `omnilang/codegen/` | Core codegen |
| `editor/client/src/` | Editor frontend |

## Generated Code Locations (these you CANNOT modify)

- `build_output/<app-name>/` — All React output files
- Any `.jsx`/`.tsx`/`.css` produced by transpilation

## 7-Stage Closed Loop

### Stage 1: Requirement Clarification

Ask the user what real app to build. Cover:
- What does the app do? (e.g., "Task manager with add/delete/complete")
- What components? (Button, Input, List, Card, etc.)
- What interactions? (click, type, submit)
- What data flow? (signals, computed values, stores)
- Edge cases? (empty state, error handling)

Output: written app spec saved as conversation context.

### Stage 2: Visual Implementation

Use **omni-driver** skill + Browser MCP to build the app in the visual editor:

```
1. Navigate to http://localhost:5174
2. Wait for window.__OMNI__
3. O.help() — discover API
4. O.switchMode("design")
5. O.clearAll()
6. Build UI with Progressive Build-Test Loop:
   - Create root container (O.card)
   - Add components (O.button, O.input, O.text, O.heading)
   - Add code blocks (O.codeBlock for signal/computed/action)
   - Wire events (createEventHandler)
   - Wire dataflow (connectTo)
   - O.validate() every 3-5 elements
7. Take editor screenshot for baseline
```

### Stage 3: Save & Transpile

Save both formats:

```javascript
// In browser context via omni-driver:
const O = window.__OMNI__;

// 3a. Save .omni source
O.setSourceCode(omniSourceCode);
await O.save();

// 3b. Save .omni.json (editor state)
const state = O.getState();
const omniJsonPath = activeFile.replace(/\.omni$/, ".omni.json");
// Use /api/omni-json/export for component/stores data
// Merge with elements/wires from O.getState()
await fetch('/api/omni-json/save', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    filePath: omniJsonPath,
    document: {
      schemaVersion: "1.0.0",
      meta: { /* ... */ },
      elements: state.elements,
      wires: state.wires,
      // ... from export API
    }
  })
});
```

Then transpile:
```bash
python .claude/skills/test-framework/helpers/transpile.py \
  --source build/test/<app>.omni \
  --target react \
  --output build_output/<app>/
```

### Stage 4: React Runtime Screenshot

```bash
cd build_output/<app>
npm install
npx vite --port 5201 --host &
```

Use Browser MCP to:
1. Navigate to `http://localhost:5201`
2. Wait for app to render
3. Take full-page screenshot
4. Capture DOM snapshot (accessibility tree)
5. Extract element inventory (buttons, inputs, headings, test-ids)

### Stage 5: Visual Diff Comparison

Use Vision MCP (`mcp__agent-vision__analyze_image`):
1. Load editor canvas screenshot
2. Load React runtime screenshot
3. Prompt: "Compare these two screenshots. Identify ALL differences in layout, spacing, colors, fonts, missing elements, text content. Be specific about what differs."

Use `diff-analyzer.mjs` for structural comparison:
```bash
node .claude/skills/test-framework/helpers/diff-analyzer.mjs \
  --editor-state build/test/<app>.omni.json \
  --react-url http://localhost:5201
```

### Stage 6: Logic Testing

Use Browser MCP to interact with React app:
- Click every button, verify state change
- Type in every input, verify update
- Check computed values match expected
- Test edge cases (empty state, max/min values)
- Test error conditions

Compare against visual editor behavior:
- Same interactions in editor (via omni-driver)
- Verify same results

### Stage 7: Framework Fix (if needed)

**ONLY if Stage 5 or 6 found issues:**

1. Analyze root cause — is it in react.py? css_mapping.py? lowering.py?
2. Fix the framework code (NOT generated code)
3. Verify fix by re-running the pipeline (back to Stage 2)
4. Repeat until all checks pass

### Completion: Git Push

```bash
git add -A
git commit -m "fix(framework): <describe what was fixed and why>"
git push
```

## Self-Check Checklist

- [ ] Asked user what app to build (Stage 1)
- [ ] Used Progressive Build-Test Loop (Stage 2)
- [ ] Saved BOTH .omni and .omni.json (Stage 3)
- [ ] Vision MCP comparison completed (Stage 5)
- [ ] Logic testing completed (Stage 6)
- [ ] NO generated code was modified
- [ ] Framework fix is root-cause, not symptom-patching
- [ ] All checks pass before git push

## Red Flags

- **"I'll just fix the generated React code"** → STOP. Fix the framework.
- **"The screenshots look close enough"** → STOP. Run Vision MCP for precise diff.
- **"I'll skip .omni.json — .omni is enough"** → STOP. Both must be saved.
- **"I'll build the whole UI then validate"** → STOP. Progressive Build-Test Loop only.
```

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/test-framework/SKILL.md
git commit -m "feat(test-framework): add SKILL.md with 7-stage closed-loop workflow"
```

---

### Task 2: Create transpile.py Helper

**Files:**
- Create: `.claude/skills/test-framework/helpers/transpile.py`

- [ ] **Step 1: Write transpile.py**

```python
#!/usr/bin/env python3
"""Transpile .omni source to React via omniview API, scaffold Vite project.

Usage:
  python transpile.py --source <file.omni> --target react --output <dir>
  python transpile.py --source <file.omni> --target react --output <dir> --port 5201
"""

import argparse
import json
import os
import subprocess
import sys
import urllib.request
import urllib.error


API_BASE = os.environ.get("OMNI_API", "http://localhost:5173")


def compile_omni(source_path: str, target: str) -> dict:
    """POST /api/compile with .omni source, return response JSON."""
    with open(source_path, "r", encoding="utf-8") as f:
        source = f.read()

    body = json.dumps({"source": source, "target": target}).encode("utf-8")
    req = urllib.request.Request(
        f"{API_BASE}/api/compile",
        data=body,
        headers={"Content-Type": "application/json"},
    )

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read())
    except urllib.error.URLError as e:
        print(f"ERROR: Cannot reach API at {API_BASE}/api/compile: {e}", file=sys.stderr)
        sys.exit(1)


def write_files(files: dict, output_dir: str):
    """Write transpiled files to output directory."""
    os.makedirs(output_dir, exist_ok=True)
    for fname, content in files.items():
        fpath = os.path.join(output_dir, fname)
        os.makedirs(os.path.dirname(fpath), exist_ok=True)
        with open(fpath, "w", encoding="utf-8") as f:
            f.write(content)
        print(f"  Wrote {fname} ({len(content)} chars)")


def scaffold_vite(output_dir: str, port: int, component_name: str):
    """Scaffold a minimal Vite + React project around the transpiled component."""
    # index.html
    index_html = f'''<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>{component_name} — React</title>
</head>
<body>
  <div id="root"></div>
  <script type="module" src="/main.jsx"></script>
</body>
</html>'''

    with open(os.path.join(output_dir, "index.html"), "w", encoding="utf-8") as f:
        f.write(index_html)

    # main.jsx
    main_jsx = f'''import React from "react";
import ReactDOM from "react-dom/client";
import {component_name} from "./{component_name}";

ReactDOM.createRoot(document.getElementById("root")).render(
  React.createElement(React.StrictMode, null, React.createElement({component_name}))
);'''

    with open(os.path.join(output_dir, "main.jsx"), "w", encoding="utf-8") as f:
        f.write(main_jsx)

    # vite.config.js
    vite_config = f'''import {{ defineConfig }} from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({{
  plugins: [react()],
  server: {{ port: {port} }},
}});'''

    with open(os.path.join(output_dir, "vite.config.js"), "w", encoding="utf-8") as f:
        f.write(vite_config)

    # package.json
    package_json = {
        "name": "omnilang-react-test",
        "private": True,
        "version": "1.0.0",
        "type": "module",
        "scripts": {
            "dev": "vite",
            "build": "vite build",
            "preview": "vite preview",
        },
        "dependencies": {
            "react": "^18.2.0",
            "react-dom": "^18.2.0",
            "zustand": "^4.5.0",
        },
        "devDependencies": {
            "@vitejs/plugin-react": "^4.2.0",
            "vite": "^5.4.0",
        },
    }

    with open(os.path.join(output_dir, "package.json"), "w", encoding="utf-8") as f:
        json.dump(package_json, f, indent=2)

    print(f"  Scaffolded Vite project (port={port})")


def install_deps(output_dir: str):
    """Run npm install in output directory."""
    node_modules = os.path.join(output_dir, "node_modules")
    if os.path.exists(node_modules):
        print("  node_modules exists, skipping npm install")
        return

    print("  Installing npm dependencies...")
    result = subprocess.run(
        ["npm", "install"],
        cwd=output_dir,
        capture_output=True,
        text=True,
        shell=(sys.platform == "win32"),
    )
    if result.returncode != 0:
        print(f"  WARNING: npm install failed:\n{result.stderr}")
    else:
        print("  npm install complete")


def main():
    parser = argparse.ArgumentParser(description="Transpile .omni → React + Vite scaffold")
    parser.add_argument("--source", required=True, help="Path to .omni source file")
    parser.add_argument("--target", default="react", help="Target language (default: react)")
    parser.add_argument("--output", required=True, help="Output directory for React project")
    parser.add_argument("--port", type=int, default=5201, help="Vite dev server port (default: 5201)")
    args = parser.parse_args()

    print(f"Transpiling {args.source} → {args.target}")
    print(f"  API: {API_BASE}")

    # Step 1: Compile
    result = compile_omni(args.source, args.target)
    if not result.get("success"):
        errors = result.get("errors", [])
        print(f"ERROR: Compile failed with {len(errors)} error(s):")
        for e in errors:
            print(f"  - {e}")
        sys.exit(1)

    files = result.get("files", {})
    print(f"  Compile OK — {len(files)} file(s): {', '.join(files.keys())}")

    # Step 2: Write transpiled files
    write_files(files, args.output)

    # Step 3: Determine component name from the first .jsx/.tsx file
    component_name = "App"
    for fname in files:
        if fname.endswith((".jsx", ".tsx")):
            base = os.path.splitext(os.path.basename(fname))[0]
            if base not in ("index", "main"):
                component_name = base
                break

    # Step 4: Scaffold Vite project
    scaffold_vite(args.output, args.port, component_name)

    # Step 5: Install deps
    install_deps(args.output)

    print(f"\nDone! React app ready at {args.output}")
    print(f"  cd {args.output} && npx vite --port {args.port}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Test transpile.py syntax**

```bash
python -c "import py_compile; py_compile.compile('.claude/skills/test-framework/helpers/transpile.py', doraise=True)"
```

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/test-framework/helpers/transpile.py
git commit -m "feat(test-framework): add transpile.py helper for .omni → React + Vite scaffold"
```

---

### Task 3: Create diff-analyzer.mjs Helper

**Files:**
- Create: `.claude/skills/test-framework/helpers/diff-analyzer.mjs`

- [ ] **Step 1: Write diff-analyzer.mjs**

```javascript
#!/usr/bin/env node
/**
 * Structural diff analyzer — compare editor state vs React DOM.
 *
 * Usage:
 *   node diff-analyzer.mjs --editor-state <file.omni.json> --react-url <url>
 *   node diff-analyzer.mjs --editor-state <file.omni.json> --react-html <file.html>
 */

import * as fs from "fs";
import { chromium } from "playwright";

async function loadEditorState(omniJsonPath) {
  const doc = JSON.parse(fs.readFileSync(omniJsonPath, "utf-8"));
  const views = (doc.elements || []).filter(e => e.nodeType === "view");

  return {
    totalElements: doc.elements?.length || 0,
    totalWires: doc.wires?.length || 0,
    views: views.map(e => ({
      tag: e.tag,
      id: e.id,
      label: (e.props && (e.props.label || e.props.text || e.props.placeholder)) || "",
      testId: (e.props && e.props["test-id"]) || "",
      parentId: e.parentId,
    })),
    tagCounts: views.reduce((acc, e) => {
      acc[e.tag] = (acc[e.tag] || 0) + 1;
      return acc;
    }, {}),
    codeblocks: (doc.elements || []).filter(e => e.nodeType === "codeblock").map(cb => ({
      tag: cb.tag,
      typeAnnotation: cb.typeAnnotation || "",
      ports: cb.ports || [],
    })),
  };
}

async function captureReactDOM(reactUrl) {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

  try {
    await page.goto(reactUrl, { waitUntil: "networkidle", timeout: 15000 });
    await page.waitForTimeout(1000);

    const dom = await page.evaluate(() => {
      const allElements = document.querySelectorAll("*");
      const tagCounts = {};
      const interactive = [];
      const testIdElements = [];
      const textContent = [];

      allElements.forEach(el => {
        const tag = el.tagName.toLowerCase();
        tagCounts[tag] = (tagCounts[tag] || 0) + 1;

        if (["button", "input", "select", "textarea"].includes(tag)) {
          interactive.push({
            tag,
            text: (el.textContent || "").trim().substring(0, 50),
            type: el.getAttribute("type") || "",
            placeholder: el.getAttribute("placeholder") || "",
            testId: el.getAttribute("data-testid") || "",
          });
        }

        if (el.hasAttribute("data-testid")) {
          testIdElements.push({
            tag,
            testId: el.getAttribute("data-testid"),
            text: (el.textContent || "").trim().substring(0, 40),
          });
        }
      });

      // Headings
      const headings = [];
      for (let i = 1; i <= 6; i++) {
        document.querySelectorAll(`h${i}`).forEach(h => {
          headings.push({ level: i, text: h.textContent?.trim() || "" });
        });
      }

      return { tagCounts, interactive, testIdElements, headings };
    });

    return dom;
  } finally {
    await browser.close();
  }
}

function compare(editor, react) {
  const issues = [];

  // Compare tag counts (map Omni tags → HTML tags)
  const TAG_MAP = {
    Container: "div", Flex: "div", Grid: "div", Card: "div",
    Text: "span", Heading: "h2", Image: "img", Link: "a",
    Button: "button", Input: "input", TextArea: "textarea",
    Checkbox: "input", Select: "select", List: "ul", ListItem: "li",
    Spinner: "div", Progress: "progress", Alert: "div", Form: "form",
  };

  for (const [omniTag, count] of Object.entries(editor.tagCounts)) {
    const htmlTag = TAG_MAP[omniTag] || "div";
    const reactCount = react.tagCounts[htmlTag] || 0;

    if (omniTag === "Input") {
      // Input maps to input[type=...], need special handling
      const inputs = react.interactive.filter(e => e.tag === "input");
      if (inputs.length !== count) {
        issues.push({
          type: "COUNT_MISMATCH",
          element: omniTag,
          editor: count,
          react: inputs.length,
          detail: `Input: editor has ${count}, React has ${inputs.length} <input> elements`,
        });
      }
      continue;
    }

    if (omniTag === "Heading") {
      // Heading count = sum of all h1-h6
      const headingCount = react.headings.length;
      if (headingCount !== count) {
        issues.push({
          type: "COUNT_MISMATCH",
          element: omniTag,
          editor: count,
          react: headingCount,
          detail: `Heading: editor has ${count}, React has ${headingCount} headings`,
        });
      }
      continue;
    }

    // Generic comparison — not exact (div is used for many things), flag only
    // when count is 0 on one side and >0 on the other
    if (count > 0 && reactCount === 0 && !["Container", "Flex", "Grid", "Card"].includes(omniTag)) {
      issues.push({
        type: "MISSING_IN_REACT",
        element: omniTag,
        editor: count,
        react: 0,
        detail: `${omniTag}: ${count} in editor, 0 in React`,
      });
    }
  }

  // Compare interactive elements
  const editorButtons = (editor.views || []).filter(v => v.tag === "Button").length;
  const reactButtons = react.interactive.filter(e => e.tag === "button").length;
  if (editorButtons !== reactButtons) {
    issues.push({
      type: "BUTTON_COUNT",
      editor: editorButtons,
      react: reactButtons,
      detail: `Buttons: editor has ${editorButtons}, React has ${reactButtons}`,
    });
  }

  // Compare test-id elements
  const editorTestIds = (editor.views || []).filter(v => v.testId).map(v => v.testId).sort();
  const reactTestIds = react.testIdElements.map(e => e.testId).sort();

  const missingTestIds = editorTestIds.filter(id => !reactTestIds.includes(id));
  const extraTestIds = reactTestIds.filter(id => !editorTestIds.includes(id));

  if (missingTestIds.length > 0) {
    issues.push({
      type: "MISSING_TEST_IDS",
      ids: missingTestIds,
      detail: `Test IDs in editor but missing in React: ${missingTestIds.join(", ")}`,
    });
  }
  if (extraTestIds.length > 0) {
    issues.push({
      type: "EXTRA_TEST_IDS",
      ids: extraTestIds,
      detail: `Test IDs in React but not in editor: ${extraTestIds.join(", ")}`,
    });
  }

  return {
    match: issues.length === 0,
    issues,
    summary: {
      editor: { elements: editor.totalElements, wires: editor.totalWires, views: editor.views.length },
      react: { tagCounts: react.tagCounts, interactiveCount: react.interactive.length, headingsCount: react.headings.length },
    },
  };
}

async function main() {
  const args = process.argv.slice(2);
  const getArg = (name) => {
    const idx = args.indexOf(name);
    return idx >= 0 ? args[idx + 1] : null;
  };

  const omniJsonPath = getArg("--editor-state");
  const reactUrl = getArg("--react-url");

  if (!omniJsonPath) {
    console.error("ERROR: --editor-state <path> is required");
    process.exit(1);
  }
  if (!reactUrl) {
    console.error("ERROR: --react-url <url> is required");
    process.exit(1);
  }

  console.log("═".repeat(60));
  console.log("STRUCTURAL DIFF: Editor State vs React DOM");
  console.log("═".repeat(60));

  // Load editor state
  console.log(`\n📋 Editor state: ${omniJsonPath}`);
  const editor = await loadEditorState(omniJsonPath);
  console.log(`   Elements: ${editor.totalElements} (${editor.views.length} views + ${editor.codeblocks.length} codeblocks)`);
  console.log(`   Wires: ${editor.totalWires}`);
  console.log(`   Tags: ${JSON.stringify(editor.tagCounts)}`);

  // Capture React DOM
  console.log(`\n🌐 React DOM: ${reactUrl}`);
  const react = await captureReactDOM(reactUrl);
  console.log(`   Tags: ${JSON.stringify(react.tagCounts)}`);
  console.log(`   Interactive: ${react.interactive.length}`);
  console.log(`   Headings: ${react.headings.length}`);

  // Compare
  console.log(`\n🔍 Comparison:`);
  const result = compare(editor, react);

  if (result.match) {
    console.log("   ✅ MATCH — no structural differences found");
  } else {
    console.log(`   ❌ ${result.issues.length} issue(s) found:`);
    for (const issue of result.issues) {
      console.log(`   - [${issue.type}] ${issue.detail}`);
    }
  }

  console.log("\n" + "═".repeat(60));
  console.log(JSON.stringify(result.summary, null, 2));
  console.log("═".repeat(60));

  process.exit(result.match ? 0 : 1);
}

main().catch(e => { console.error(e); process.exit(1); });
```

- [ ] **Step 2: Test diff-analyzer.mjs syntax**

```bash
node --check .claude/skills/test-framework/helpers/diff-analyzer.mjs
```

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/test-framework/helpers/diff-analyzer.mjs
git commit -m "feat(test-framework): add diff-analyzer.mjs for structural editor vs React comparison"
```

---

### Task 4: Create compare.mjs Orchestration Script

**Files:**
- Create: `.claude/skills/test-framework/compare.mjs`

- [ ] **Step 1: Write compare.mjs**

```javascript
#!/usr/bin/env node
/**
 * Full pipeline: Editor → Transpile → React → Screenshot → Compare
 *
 * Usage:
 *   node compare.mjs --omni <file.omni> --app-name <name>
 *   node compare.mjs --omni <file.omni> --app-name <name> --skip-install
 *
 * Output:
 *   build_output/<app-name>/compare-editor.png
 *   build_output/<app-name>/compare-react.png
 *   build_output/<app-name>/diff-report.json
 */

import { chromium } from "playwright";
import * as fs from "fs";
import * as path from "path";
import { execSync, spawn } from "child_process";

const EDITOR_URL = process.env.OMNI_EDITOR_URL || "http://localhost:5174";
const REACT_PORT = parseInt(process.env.REACT_PORT || "5201", 10);
const PROJECT_DIR = process.env.OMNI_PROJECT_DIR || path.resolve(
  new URL(import.meta.url).pathname.replace(/^\/([A-Z]:\/)/, "$1"), "../../.."
);

async function sleep(ms) {
  return new Promise(r => setTimeout(r, ms));
}

async function stage1_loadEditor(browser, omniSourcePath) {
  console.log("─".repeat(50));
  console.log("Stage 1: Loading app in visual editor...");

  const omniSource = fs.readFileSync(omniSourcePath, "utf-8");
  const omniJsonPath = omniSourcePath.replace(/\.omni$/, ".omni.json");

  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.goto(EDITOR_URL, { waitUntil: "networkidle", timeout: 30000 });
  await page.waitForFunction(() => window.__OMNI__);

  // Load and compile
  const compileResult = await page.evaluate(async (source) => {
    const O = window.__OMNI__;
    O.help();
    O.switchMode("code");
    O.setSourceCode(source);
    const result = await O.compile();
    O.switchMode("design");
    return {
      success: result.success,
      errors: (result.errors || []).length,
      elements: O.elements.length,
      wires: O.wires.length,
    };
  }, omniSource);

  console.log(`  Compile: ${compileResult.success ? "OK" : "FAIL"}, errors: ${compileResult.errors}`);
  console.log(`  Canvas: ${compileResult.elements} elements, ${compileResult.wires} wires`);

  await page.waitForTimeout(1000);

  // Take editor screenshot
  const editorScreenshotPath = path.join(PROJECT_DIR, "build_output", path.basename(omniSourcePath, ".omni"), "compare-editor.png");
  const outDir = path.dirname(editorScreenshotPath);
  fs.mkdirSync(outDir, { recursive: true });
  await page.screenshot({ path: editorScreenshotPath, fullPage: false });

  // Save .omni.json
  await page.evaluate(async (jsonPath) => {
    const O = window.__OMNI__;
    const state = O.getState();
    const activeFile = O.getActiveFilePath() || "project.omni";
    const jsonFilePath = jsonPath || activeFile.replace(/\.omni$/, ".omni.json");

    // Try the API
    try {
      const exportResp = await fetch("/api/omni-json/export", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          source: O.getSourceCode(),
          filepath: activeFile,
          target: "react",
        }),
      });
      const exportData = await exportResp.json();
      if (exportData.success && exportData.document) {
        const doc = exportData.document;
        doc.elements = state.elements;
        doc.wires = state.wires;
        await fetch("/api/omni-json/save", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ filePath: jsonFilePath, document: doc }),
        });
        return { saved: true, path: jsonFilePath };
      }
    } catch (e) {
      // Fallback: save state directly
      const doc = {
        schemaVersion: "1.0.0",
        meta: { projectName: "test", omniVersion: "0.4.0", createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(), isHandWritten: false, conversionWarnings: [] },
        components: [], stores: [], themes: [], imports: [],
        elements: state.elements,
        wires: state.wires,
        ownershipRegions: [], searchIndex: { entries: [], version: 1, lastBuilt: new Date().toISOString() },
        canvasLayout: { positions: {} },
      };
      await fetch("/api/fs/write", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ filePath: jsonFilePath, content: JSON.stringify(doc, null, 2) }),
      });
      return { saved: true, path: jsonFilePath, fallback: true };
    }
  }, omniJsonPath);

  console.log(`  .omni.json saved: ${omniJsonPath}`);
  console.log(`  Screenshot: ${editorScreenshotPath}`);

  return { page, editorScreenshotPath, omniJsonPath, compileResult };
}

async function stage2_transpile(omniSourcePath, appName) {
  console.log("\n" + "─".repeat(50));
  console.log("Stage 2: Transpiling to React...");

  const outputDir = path.join(PROJECT_DIR, "build_output", appName);
  const transpileScript = path.join(PROJECT_DIR, ".claude/skills/test-framework/helpers/transpile.py");

  try {
    execSync(
      `python "${transpileScript}" --source "${omniSourcePath}" --target react --output "${outputDir}" --port ${REACT_PORT}`,
      { stdio: "pipe", encoding: "utf-8", shell: process.platform === "win32" }
    );
    console.log("  Transpile OK");
  } catch (e) {
    console.error("  Transpile FAILED:", e.stderr || e.message);
    return null;
  }

  return outputDir;
}

async function stage3_startReact(outputDir) {
  console.log("\n" + "─".repeat(50));
  console.log("Stage 3: Starting React app...");

  const viteProc = spawn("npx", ["vite", "--port", String(REACT_PORT), "--host"], {
    cwd: outputDir,
    stdio: "pipe",
    shell: process.platform === "win32",
  });

  await sleep(4000);
  console.log(`  Vite started on port ${REACT_PORT}`);
  return viteProc;
}

async function stage4_captureReact(browser, appName) {
  console.log("\n" + "─".repeat(50));
  console.log("Stage 4: Capturing React app...");

  const reactUrl = `http://localhost:${REACT_PORT}`;
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

  try {
    await page.goto(reactUrl, { waitUntil: "networkidle", timeout: 15000 });
  } catch {
    console.log("  WARNING: networkidle timeout, continuing...");
  }
  await page.waitForTimeout(1000);

  const reactScreenshotPath = path.join(PROJECT_DIR, "build_output", appName, "compare-react.png");
  await page.screenshot({ path: reactScreenshotPath, fullPage: true });

  // Extract DOM snapshot
  const domSnapshot = await page.evaluate(() => {
    const buttons = Array.from(document.querySelectorAll("button")).map(b => ({
      text: (b.textContent || "").trim().substring(0, 30),
      testId: b.getAttribute("data-testid") || "",
    }));
    const inputs = Array.from(document.querySelectorAll("input")).map(i => ({
      placeholder: i.getAttribute("placeholder") || "",
      type: i.getAttribute("type") || "text",
      testId: i.getAttribute("data-testid") || "",
    }));
    const headings = Array.from(document.querySelectorAll("h1,h2,h3,h4,h5,h6")).map(h => ({
      level: parseInt(h.tagName[1], 10),
      text: (h.textContent || "").trim().substring(0, 50),
    }));
    const testIds = Array.from(document.querySelectorAll("[data-testid]")).map(e => ({
      testId: e.getAttribute("data-testid"),
      tag: e.tagName.toLowerCase(),
    }));
    return { buttons, inputs, headings, testIds, buttonCount: buttons.length, inputCount: inputs.length, headingCount: headings.length, testIdCount: testIds.length };
  });

  console.log(`  Screenshot: ${reactScreenshotPath}`);
  console.log(`  DOM: ${domSnapshot.buttonCount} buttons, ${domSnapshot.inputCount} inputs, ${domSnapshot.headingCount} headings, ${domSnapshot.testIdCount} test-ids`);

  await page.close();
  return { reactScreenshotPath, domSnapshot };
}

async function stage5_structuralDiff(omniJsonPath, reactUrl, appName) {
  console.log("\n" + "─".repeat(50));
  console.log("Stage 5: Structural diff...");

  const diffScript = path.join(PROJECT_DIR, ".claude/skills/test-framework/helpers/diff-analyzer.mjs");
  const diffReportPath = path.join(PROJECT_DIR, "build_output", appName, "diff-report.json");

  try {
    const result = execSync(
      `node "${diffScript}" --editor-state "${omniJsonPath}" --react-url "${reactUrl}"`,
      { stdio: "pipe", encoding: "utf-8", shell: process.platform === "win32" }
    );
    console.log(result);
    return { match: true, issues: [] };
  } catch (e) {
    // diff-analyzer exits with code 1 if issues found
    console.log(e.stdout || e.message);
    return { match: false, output: e.stdout || "" };
  }
}

async function main() {
  const args = process.argv.slice(2);
  const getArg = (name) => {
    const idx = args.indexOf(name);
    return idx >= 0 ? args[idx + 1] : null;
  };

  const omniPath = getArg("--omni");
  const appName = getArg("--app-name");
  const skipInstall = args.includes("--skip-install");

  if (!omniPath || !appName) {
    console.error("Usage: node compare.mjs --omni <file.omni> --app-name <name> [--skip-install]");
    process.exit(1);
  }

  console.log("╔══════════════════════════════════════════════╗");
  console.log("║  OmniLang Framework Test — Compare Pipeline ║");
  console.log("╚══════════════════════════════════════════════╝");
  console.log(`  Source: ${omniPath}`);
  console.log(`  App: ${appName}`);
  console.log(`  Editor: ${EDITOR_URL}`);
  console.log(`  React port: ${REACT_PORT}`);

  const browser = await chromium.launch({ headless: true });
  let viteProc = null;

  try {
    // Stage 1: Load in editor
    const { page: editorPage, editorScreenshotPath, omniJsonPath, compileResult } =
      await stage1_loadEditor(browser, omniPath);

    if (!compileResult.success) {
      console.error("ERROR: Compile failed — cannot continue");
      await browser.close();
      process.exit(1);
    }

    // Stage 2: Transpile
    const outputDir = await stage2_transpile(omniPath, appName);
    if (!outputDir) {
      await browser.close();
      process.exit(1);
    }

    // Stage 3: Start React
    viteProc = await stage3_startReact(outputDir);

    // Stage 4: Capture React
    const { reactScreenshotPath, domSnapshot } = await stage4_captureReact(browser, appName);

    // Stage 5: Structural diff
    const diffResult = await stage5_structuralDiff(omniJsonPath, `http://localhost:${REACT_PORT}`, appName);

    // Summary
    console.log("\n" + "═".repeat(60));
    console.log("PIPELINE COMPLETE");
    console.log("═".repeat(60));
    console.log(`  Editor screenshot: ${editorScreenshotPath}`);
    console.log(`  React screenshot:  ${reactScreenshotPath}`);
    console.log(`  .omni.json:        ${omniJsonPath}`);
    console.log(`  Structural diff:   ${diffResult.match ? "✅ MATCH" : "❌ ISSUES FOUND"}`);

    if (!diffResult.match) {
      console.log("\n⚠ Run Vision MCP comparison next:");
      console.log(`  Compare: ${editorScreenshotPath}`);
      console.log(`  vs:      ${reactScreenshotPath}`);
    }

    // Write summary JSON
    const summaryPath = path.join(PROJECT_DIR, "build_output", appName, "pipeline-summary.json");
    fs.writeFileSync(summaryPath, JSON.stringify({
      timestamp: new Date().toISOString(),
      appName,
      editor: { screenshot: editorScreenshotPath, omniJson: omniJsonPath, elements: compileResult.elements, wires: compileResult.wires },
      react: { screenshot: reactScreenshotPath, port: REACT_PORT, ...domSnapshot },
      diff: diffResult,
    }, null, 2));
    console.log(`  Summary: ${summaryPath}`);

  } finally {
    if (viteProc) viteProc.kill();
    await browser.close();
    console.log("\nDone!");
  }
}

main().catch(e => { console.error(e); process.exit(1); });
```

- [ ] **Step 2: Test compare.mjs syntax**

```bash
node --check .claude/skills/test-framework/compare.mjs
```

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/test-framework/compare.mjs
git commit -m "feat(test-framework): add compare.mjs full pipeline orchestration script"
```

---

### Task 5: Integration Test — Run Full Pipeline with Counter App

**Files:**
- Use: `tests/frameworks/behavior/01_counter.omni` (existing)

- [ ] **Step 1: Ensure editor and backend are running**

```bash
bash scripts/start-editor.sh &
# Wait for startup
sleep 8
```

- [ ] **Step 2: Run compare.mjs**

```bash
node .claude/skills/test-framework/compare.mjs \
  --omni tests/frameworks/behavior/01_counter.omni \
  --app-name counter-test
```

- [ ] **Step 3: Verify output files exist**

```bash
ls -la build_output/counter-test/
# Should contain: compare-editor.png, compare-react.png, diff-report.json, pipeline-summary.json
# Plus React files: Counter.jsx, main.jsx, index.html, vite.config.js, package.json
```

- [ ] **Step 4: Run Vision MCP comparison**

Use `mcp__agent-vision__analyze_image` to compare editor and React screenshots.

- [ ] **Step 5: Run logic tests via Browser MCP**

Navigate to `http://localhost:5201`, click buttons, verify counter increments/decrements.

- [ ] **Step 6: If issues found, fix framework code and re-run**

- [ ] **Step 7: Commit results**

```bash
git add -A
git commit -m "test(framework): integration test run — counter app pipeline results"
```

---

### Task 6: Final Validation — Real App Test (User's Choice)

- [ ] **Step 1: Ask user what real app to build** (Stage 1 of the skill)

- [ ] **Step 2: Build app in visual editor via omni-driver** (Stage 2)

- [ ] **Step 3: Run full compare pipeline** (Stages 3-5)

- [ ] **Step 4: Run logic tests** (Stage 6)

- [ ] **Step 5: Fix framework code if needed** (Stage 7)

- [ ] **Step 6: Loop until all checks pass**

- [ ] **Step 7: Git push**

```bash
git push
```
