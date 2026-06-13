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
5. **Always compare CSS/styling separately** — ask Vision MCP about colors, borders, shadows, padding explicitly, not just element presence.
6. **Loop until all checks pass** before git push.

## ⚠️ Known Pitfalls (READ BEFORE STARTING)

### Pitfall 1: Programmatic Elements ≠ .omni Source
**`O.button()` etc. create elements in Zustand store only.** `getSourceCode()` returns the FILE content, not programmatic state. To transpile, you MUST write .omni source and use `setSourceCode()` + `compile()`.

✅ **Correct flow:** Write `.omni` source → `setSourceCode()` in code mode → `compile()` → switch to design mode to see rendered result.

### Pitfall 2: Python Bytecode Cache
After editing `omniview/codegen/*.py` or `omnilang/**/*.py`, the running server WON'T pick up changes because Python caches `.pyc` files. You MUST:
```bash
# Find and kill backend
PID=$(netstat -ano | grep ":5173 " | grep LISTENING | awk '{print $NF}')
taskkill //F //PID "$PID"
# Clear ALL caches
find . -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null
# Restart
python -m editor.server.main --dir build/test --port 5173 &
```

### Pitfall 3: Worktree vs Main Repo
If using a git worktree, Python imports from the **worktree** path, not the main repo. Edit the worktree's copy of framework files. Verify with:
```bash
python -c "import omniview.codegen.react; import inspect; print(inspect.getfile(omniview.codegen.react))"
```

### Pitfall 4: useCallback Stale Closure (CRITICAL)
Generated React code uses `useCallback(() => { setX(x + 1) }, [])`. The empty deps `[]` + direct state read `(x + 1)` causes STALE CLOSURE — x is always the initial value (0).

**This is the #1 framework bug to check.** Fix: all component signal mutations must use functional updaters:
- ❌ `setX(x + 1)` — reads stale closure value
- ✅ `setX(prev => prev + 1)` — always reads latest state

Also check `else` branch (direct_assign with self-referencing RHS, e.g. `count = count + 1` parses as direct_assign, NOT increment).

### Pitfall 5: Screenshot Staleness
After reloading the editor or changing the React app, **always retake screenshots**. Vision MCP may analyze cached/old files. Take fresh screenshots immediately before comparison.

### Pitfall 6: Port Conflicts
Vite auto-increments port if occupied (5201→5202→5203...). Always check the actual port Vite started on and use it for screenshots:
```bash
curl -s http://localhost:5201 || curl -s http://localhost:5202 || ...
```

### Pitfall 7: Vision MCP Needs Explicit Prompts
Don't ask "compare these screenshots". Instead, ask SEPARATE targeted questions:
1. **Elements:** "List ALL visible UI elements with exact text"
2. **CSS:** "Describe button colors, card shadow, border radius, padding, text color"
3. **Layout:** "Describe flex direction, gap, alignment of product rows"

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
- What does the app do?
- What components? (Button, Input, List, Card, etc.)
- What interactions? (click, type, submit)
- What data flow? (signals, computed values, stores)
- Edge cases? (empty state, error handling)

### Stage 2: Visual Implementation

> ⚠️ **Use `setSourceCode` + `compile` approach** (not programmatic O.button()). See Pitfall 1.

1. Write `.omni` source file to `build/test/<app-name>/<AppName>.omni`
2. Navigate to `http://localhost:5174`
3. `O.help()` → `O.switchMode("code")`
4. `O.setSourceCode(source)` → `await O.compile()`
5. `O.switchMode("design")` — verify visual rendering
6. Take editor screenshot

### Stage 3: Save & Transpile

Save BOTH formats:
```javascript
// 3a. .omni is already written (Stage 2)

// 3b. Save .omni.json via API
const state = O.getState();
const resp = await fetch("/api/omni-json/export", {
  method: "POST", headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ source: O.getSourceCode(), filepath: activeFile, target: "react" }),
});
const { document: doc } = await resp.json();
doc.elements = state.elements;
doc.wires = state.wires;
await fetch("/api/omni-json/save", {
  method: "POST", headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ filePath: omniJsonPath, document: doc }),
});
```

Then transpile via API:
```javascript
const resp = await fetch("/api/compile", {
  method: "POST", headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ source: O.getSourceCode(), target: "react" }),
});
const { files } = await resp.json();
```

Write React files + scaffold Vite project to `build_output/<app-name>/`.

### Stage 4: React Runtime Screenshot

```bash
cd build_output/<app-name>
npm install
npx vite --port 5201 --host &
# ⚠️ Check actual port (Pitfall 6)
curl -s http://localhost:5201
```

Use Browser MCP to:
1. Navigate to React URL
2. **Retake screenshot** (Pitfall 5)
3. Capture DOM snapshot
4. Extract element inventory (buttons, inputs, headings, test-ids)

### Stage 5: Visual Diff Comparison

**Must compare BOTH structure AND CSS separately** (Pitfall 7):

**5a. Structure comparison:**
Use Vision MCP: "List ALL visible UI elements with exact text content"
Compare: headings match? button labels match? product text match?

**5b. CSS/Style comparison (SEPARATE prompt!):**
Use Vision MCP with these specific questions:
- Card container: white bg? rounded corners? box shadow? padding?
- Primary buttons: background color? text color? border? border-radius?
- Outline buttons: background? border color?
- Layout: flex direction? gap between elements?
- Text: color? font weight?

**5c. Common CSS bugs to check:**
- [ ] `variant` prop → is it inline style or useless `data-variant` attribute?
- [ ] `LAYOUT_PROPS` in react.py — does it include borderRadius, boxShadow, color, fontSize, fontWeight, background?
- [ ] Card tag — does it get default styles (bg, radius, shadow)?
- [ ] Button — are primary/outline variants visually distinct?
- [ ] Padding from Card → does it reach the React output?

Use `diff-analyzer.mjs` for structural comparison:
```bash
node .claude/skills/test-framework/helpers/diff-analyzer.mjs \
  --editor-state build/test/<app>.omni.json \
  --react-url http://localhost:<port>
```

### Stage 6: Logic Testing

Use Browser MCP to interact with React app:
- Click every button, verify state change
- Click multiple buttons, verify ACCUMULATED state (test for stale closure — Pitfall 4)
- Type in every input, verify update
- Test edge cases (empty state, max/min values, clear/reset)
- **Repeat clicks after Clear** to verify state works post-reset

### Stage 7: Framework Fix

**ONLY if Stage 5 or 6 found issues:**

1. Analyze root cause — use the Known Pitfalls as a checklist
2. **Verify which Python path is active** (Pitfall 3)
3. Fix the framework code (NOT generated code)
4. **Clear bytecode cache + restart backend** (Pitfall 2)
5. Verify fix: re-transpile → rebuild React → retake screenshots → re-compare
6. Repeat until all checks pass

### Completion: Git Push

```bash
git add omniview/codegen/react.py  # or whichever framework file
git commit -m "fix(framework): <describe root cause and fix>"
git push
```

## Self-Check Checklist

- [ ] Asked user what app to build (Stage 1)
- [ ] Wrote .omni source + compiled via setSourceCode (NOT programmatic) (Stage 2)
- [ ] Saved BOTH .omni and .omni.json (Stage 3)
- [ ] Cleared __pycache__ after any Python edits (Pitfall 2)
- [ ] Verified Python import path (Pitfall 3)
- [ ] Vision MCP: structure comparison done (Stage 5a)
- [ ] Vision MCP: CSS comparison done SEPARATELY (Stage 5b)
- [ ] Checked useCallback functional updaters (Pitfall 4)
- [ ] Checked variant→style mapping (Pitfall 5c)
- [ ] Logic testing: multi-click accumulation verified (Stage 6)
- [ ] NO generated code was modified
- [ ] Framework fix is root-cause, not symptom-patching
- [ ] All checks pass before git push

## Red Flags — STOP and Check

- **"I'll just fix the generated React code"** → STOP. Fix the framework.
- **"The screenshots look close enough"** → STOP. Run Vision MCP for precise diff.
- **"I'll skip .omni.json — .omni is enough"** → STOP. Both must be saved.
- **"I'll build the whole UI then validate"** → STOP. Progressive Build-Test Loop only.
- **"The Python fix is in the file but API still returns old code"** → STOP. Clear __pycache__ + restart backend (Pitfall 2).
- **"I'm editing the main repo but the worktree is running"** → STOP. Check import path (Pitfall 3).
- **"Vision MCP says everything matches" after one prompt** → STOP. Run separate CSS prompt (Pitfall 7).
- **"useCallback deps are empty but it should be fine"** → STOP. Check Pitfall 4 — this IS a bug.
