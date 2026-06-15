# OmniDriver v2 Verification Plan — E2E Testing + Browser MCP

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verify the OmniDriver v2 implementation by running all 8 E2E tests (5 spec files) against the live editor, using Browser MCP for interactive validation. TypeScript must compile, Vite must build, and all E2E tests must pass.

**Architecture:** Start Python backend (port 5173) + Vite frontend (port 5174) → run `tsc --noEmit` + `vite build` → run 8 Playwright E2E tests → use Browser MCP (`browser_evaluate` + `browser_snapshot`) for interactive verification of the full Task Manager Pro app.

**Tech Stack:** TypeScript, Zustand, Playwright (E2E), Browser MCP (interactive testing), Vision MCP (screenshot diff)

**Spec:** [TEST_PLAN.md](../../../editor/e2e/TEST_PLAN.md), [SKILL.md](../../../.claude/skills/omni-driver/SKILL.md), [api-reference.md](../../../.claude/skills/omni-driver/api-reference.md)

---

## Pre-Flight Checks

### Check 0: Git Status Verification

- [ ] **Step 1: Verify working tree is clean for driver + e2e files**

Run: `cd "D:/space/OmniLang" && git status --short editor/client/src/driver/ editor/e2e/ build/test/omni-driver/`

Expected: No uncommitted changes (or only whitespace).

- [ ] **Step 2: Verify all 5 spec files exist**

Run: `cd "D:/space/OmniLang" && ls -la editor/e2e/omni-driver-*.spec.ts`

Expected: 5 files:
```
editor/e2e/omni-driver-code-tab.spec.ts
editor/e2e/omni-driver-counter.spec.ts
editor/e2e/omni-driver-customfn.spec.ts
editor/e2e/omni-driver-event-binding.spec.ts
editor/e2e/omni-driver-task-manager.spec.ts
```

### Check 1: Port Availability

- [ ] **Step 1: Kill any stale processes on ports 5173, 5174**

```bash
# Kill anything on port 5173 (Python backend)
netstat -ano | grep ":5173 " | grep LISTENING | awk '{print $NF}' | sort -u | xargs -r taskkill //F //PID 2>/dev/null

# Kill anything on port 5174 (Vite frontend)
netstat -ano | grep ":5174 " | grep LISTENING | awk '{print $NF}' | sort -u | xargs -r taskkill //F //PID 2>/dev/null

sleep 2
```

- [ ] **Step 2: Verify ports are free**

Run: `netstat -ano | grep -E ":5173|:5174" | grep LISTENING`
Expected: No output (ports free).

---

## Phase 1: TypeScript Compilation

### Task 1: TypeScript Type Check

**Files:** All `editor/client/src/` files

- [ ] **Step 1: Run tsc --noEmit**

Run: `cd "D:/space/OmniLang/editor/client" && npx tsc --noEmit --pretty false 2>&1`

Expected: Zero errors.

- [ ] **Step 2: If errors exist, categorize and fix**

| Error pattern | Likely cause | Fix |
|--------------|-------------|-----|
| `Property 'bindEvent' does not exist` | Method not added to ElementHandle | Add to OmniDriver.ts |
| `Property 'createTab' does not exist` | Method not added to OmniDriver | Add to OmniDriver.ts |
| `Property 'customFunction' does not exist` | Method not added to OmniDriver | Add to OmniDriver.ts |
| `Cannot find name 'FnParam'` | Types not exported | Check types.ts exports |
| `Type '...' is not assignable` | Store action signatures mismatch | Update editor.ts action types |

- [ ] **Step 3: Commit if fixes were needed**

```bash
cd "D:/space/OmniLang" && git add editor/client/src/ && git commit -m "fix(omni-driver): resolve TypeScript errors"
```

---

## Phase 2: Vite Production Build

### Task 2: Vite Build

**Files:** All `editor/client/` files

- [ ] **Step 1: Run vite build**

Run: `cd "D:/space/OmniLang/editor/client" && npx vite build 2>&1`

Expected: `✓ built in X.XXs`

- [ ] **Step 2: If build fails, diagnose**

Common Vite build failures:
| Symptom | Cause | Fix |
|---------|-------|-----|
| `Could not resolve '...'` | Missing import or broken path | Check import statement |
| `'...' is not exported from '...'` | Missing type export | Add export in source file |
| `RollupError: ...` | Tree-shaking issue | Check side-effects in package.json |

- [ ] **Step 3: Commit if fixes were needed**

---

## Phase 3: Infrastructure Startup

### Task 3: Start Backend + Frontend

- [ ] **Step 1: Start Python backend**

```bash
cd "D:/space/OmniLang" && python -m editor.server.main --dir build/test --port 5173 &
```

Wait 3s, then verify:
```bash
netstat -ano | grep ":5173 " | grep LISTENING
```
Expected: One line showing port 5173 is listening.

- [ ] **Step 2: Start Vite dev server**

```bash
cd "D:/space/OmniLang/editor/client" && npm run dev &
```

Wait 5s, then verify:
```bash
netstat -ano | grep ":5174 " | grep LISTENING
```
Expected: One line showing port 5174 is listening.

- [ ] **Step 3: Smoke test — browser navigates to editor**

```
mcp__playwright__browser_navigate → http://localhost:5174
```

Wait 3s. Take snapshot:
```
mcp__playwright__browser_snapshot
```

Expected: Editor UI is visible (ActivityBar, canvas area, etc.), no blank white page.

- [ ] **Step 4: Verify window.__OMNI__ is available**

```
mcp__playwright__browser_evaluate:
  function: () => { const O = window.__OMNI__; O.help(); return { version: O.help().version, hasPortRef: !!O.help().portReference }; }
```

Expected: `version: "2.0.0"`, `hasPortRef: true`.

---

## Phase 4: E2E Test Execution (Playwright)

> **Important:** Tests must run in order — simpler tests first to catch basic breakage before complex ones.

### Task 4: Run Event Binding Test

**File:** `editor/e2e/omni-driver-event-binding.spec.ts`

- [ ] **Step 1: Run the test**

```bash
cd "D:/space/OmniLang" && npx playwright test editor/e2e/omni-driver-event-binding.spec.ts --reporter=line 2>&1
```

Expected: `2 passed` (2 tests: bindEvent + createEventHandler deprecation).

- [ ] **Step 2: If failures, diagnose with Browser MCP**

```
mcp__playwright__browser_evaluate:
  function: () => {
    const O = window.__OMNI__;
    O.help();
    O.switchMode("design");
    O.clearAll();
    const btn = O.button("Submit", { variant: "primary" });
    btn.bindEvent("onPress", "handleSubmit");
    return {
      onPress: O.element(btn.id).data.props.onPress,
      availableEvents: btn.getAvailableEvents(),
      bindings: btn.getEventBindings(),
      allBindings: O.listAllEventBindings(),
    };
  }
```

Common failure modes:
| Symptom | Diagnostic |
|---------|-----------|
| `btn.bindEvent is not a function` | Method not added to ElementHandle |
| `onPress` is `undefined` | setProp not persisting in store |
| `getAvailableEvents` returns `[]` | COMPONENT_PROTOCOL not loaded |
| `listAllEventBindings` returns `[]` | Code graph store not accessible |

- [ ] **Step 3: Fix framework code if needed (NEVER fix the test!)**

---

### Task 5: Run Code Tab Management Test

**File:** `editor/e2e/omni-driver-code-tab.spec.ts`

- [ ] **Step 1: Run the test**

```bash
cd "D:/space/OmniLang" && npx playwright test editor/e2e/omni-driver-code-tab.spec.ts --reporter=line 2>&1
```

Expected: `1 passed`.

- [ ] **Step 2: If failures, diagnose with Browser MCP**

```
mcp__playwright__browser_evaluate:
  function: () => {
    const O = window.__OMNI__;
    O.help();
    O.switchMode("code");
    const tab = O.createTab("event", "Test Events", "🧪");
    O.switchTab(tab.id);
    O.addVariable(tab.id, "testVar", "i32", 0);
    return {
      tabCount: O.listTabs().length,
      activeTabLabel: O.getActiveTab()?.label,
      vars: O.getVariables(tab.id),
      exported: O.exportTab(tab.id) ? "valid" : "null",
    };
  }
```

Common failure modes:
| Symptom | Diagnostic |
|---------|-----------|
| `createTab is not a function` | Method not added to OmniDriver |
| `listTabs()` returns `[]` | Code graph store not initialized |
| `getVariables` returns `[]` | Variable store action not wired |
| `exportTab` returns `null` | Tab not found or serialization error |
| `cloneTab` returns `null` | Deep clone logic broken |

- [ ] **Step 3: Fix framework code if needed**

---

### Task 6: Run Custom Function Test

**File:** `editor/e2e/omni-driver-customfn.spec.ts`

- [ ] **Step 1: Run the test**

```bash
cd "D:/space/OmniLang" && npx playwright test editor/e2e/omni-driver-customfn.spec.ts --reporter=line 2>&1
```

Expected: `1 passed`.

- [ ] **Step 2: If failures, diagnose with Browser MCP**

```
mcp__playwright__browser_evaluate:
  function: () => {
    const O = window.__OMNI__;
    O.help();
    O.switchMode("code");
    O.clearAll();
    const fn = O.customFunction("testFn", [{ name: "x", type: "i32" }], "i32");
    const ret = O.returnNode(fn.id);
    const addNode = O.codeBlock("add", "addX", { typeAnnotation: "i32" });
    addNode.connectTo(ret, "result", "value");
    const report = O.validateFunction(fn.id);
    return {
      fnTag: O.element(fn.id).data.tag,
      ports: O.element(fn.id).data.ports?.map(p => p.name),
      retPorts: O.element(ret.id).data.ports?.map(p => p.name),
      valid: report.valid,
      paths: report.reachableReturnPaths,
      listCount: O.listCustomFunctions().length,
      found: O.findFunctionByName("testFn")?.fnName,
    };
  }
```

Common failure modes:
| Symptom | Diagnostic |
|---------|-----------|
| `customFunction is not a function` | Method not added to OmniDriver |
| `returnNode` throws "not a customFn" | Tag check failing |
| `validateFunction` returns `valid: false` | BFS not finding return node |
| `addParam` doesn't add port | Port regeneration logic broken |
| `setReturnType` doesn't update port | Port type update not persisting |

- [ ] **Step 3: Fix framework code if needed**

---

### Task 7: Run Counter v2 Test

**File:** `editor/e2e/omni-driver-counter.spec.ts`

- [ ] **Step 1: Review counter test for v2 compliance**

Key v2 requirements in the test:
1. Uses `bindEvent("onPress", "handleIncrement")` — NOT `createEventHandler`
2. Uses `customFunction("handleIncrement", [], "void")` in code mode
3. Uses `returnNode(fn.id)` after customFn
4. Asserts `listCustomFunctions().length >= 1` and `listTabs().length >= 1`
5. No `action` codeBlock references
6. `COUNTER_OMNI_SOURCE` uses `fn increment() { count += 1 }` (no action blocks in source)

- [ ] **Step 2: Run the test**

```bash
cd "D:/space/OmniLang" && npx playwright test editor/e2e/omni-driver-counter.spec.ts --reporter=line 2>&1
```

Expected: All tests in the spec pass.

- [ ] **Step 3: If failures, diagnose with Browser MCP**

The counter test has 4 tests:
1. Main counter creation test (v2 API)
2. `O.help()` and discovery APIs
3. `O.validate()` on empty canvas
4. External `.omni` source file write/read

Run the failing test scenario interactively via browser_evaluate to find the root cause.

---

### Task 8: Run Task Manager Pro v2 Test

**File:** `editor/e2e/omni-driver-task-manager.spec.ts`

- [ ] **Step 1: Run the test**

```bash
cd "D:/space/OmniLang" && npx playwright test editor/e2e/omni-driver-task-manager.spec.ts --reporter=line 2>&1
```

Expected: All tests pass.

- [ ] **Step 2: If the external script test fails**

The Task Manager test loads `build/test/omni-driver/task-manager-app.js` via `fs.readFileSync` + `eval()`. Verify the script is valid:

```bash
cd "D:/space/OmniLang" && node -e "
const fs = require('fs');
const src = fs.readFileSync('build/test/omni-driver/task-manager-app.js', 'utf-8');
try {
  new Function(src);
  console.log('Syntax OK');
} catch(e) {
  console.error('Syntax error:', e.message);
}
"
```

- [ ] **Step 3: If failures, run the script directly via Browser MCP**

Read the script file content, then:
```
mcp__playwright__browser_evaluate:
  function: (script) => { return eval(script); }
  (pass the entire IIFE script content)
```

Check the returned build report for:
- `isValid: true`
- `elementCount >= 70`
- `orphanElements.length === 0`

---

## Phase 5: Browser MCP Interactive Verification

> 🔴 **Iron Rule:** Use Browser MCP for ALL interactive verification. No bash scripts, no curl.

### Task 9: Full Task Manager Pro Interactive Build

**Goal:** Build the Task Manager Pro app from scratch via Browser MCP and verify every phase.

- [ ] **Step 1: Navigate and initialize**

```
mcp__playwright__browser_navigate → http://localhost:5174
mcp__playwright__browser_wait_for → time: 3
```

- [ ] **Step 2: Phase 0 — API Discovery**

```
mcp__playwright__browser_evaluate:
  function: () => {
    const O = window.__OMNI__;
    O.help();
    O.switchMode("design");
    O.clearAll();
    return {
      version: O.help().version,
      snippets: O.listSnippets(),
      components: O.listComponents().length,
      hasBindEvent: typeof O.listAllEventBindings === "function",
      hasCustomFn: typeof O.customFunction === "function",
      hasTabApi: typeof O.createTab === "function",
    };
  }
```

Expected:
- `version: "2.0.0"`
- `components >= 19`
- `hasBindEvent: true`, `hasCustomFn: true`, `hasTabApi: true`

- [ ] **Step 3: Phase 1 — Header (5 elements)**

```
mcp__playwright__browser_evaluate:
  function: () => {
    const O = window.__OMNI__;
    O.help(); O.switchMode("design"); O.clearAll();
    const root = O.card({ padding: "0", maxWidth: "1200px" });
    const header = O.flex("row", "0", { padding: "12px 32px", alignItems: "center" }, root.id);
    O.image("data:image/svg+xml,...", "Logo", header.id);
    O.heading("Task Manager Pro", "1", header.id);
    const r = O.validate();
    return { elements: O.elements.length, valid: r.isValid, orphans: r.orphanElements.length };
  }
```

Expected: `elements >= 4`, `valid: true`, `orphans: 0`.

- [ ] **Step 4: Phase 2 — Sidebar + Main Layout**

Continue building side-by-side layout. Validate after each step.

- [ ] **Step 5: Phase 3 — Signals, Store, Computed**

Create data-layer nodes. Use `codeBlock("signal", ...)`, `codeBlock("store", ...)`, `codeBlock("computed", ...)`.

- [ ] **Step 6: Phase 4 — Event Handlers (code mode)**

```
mcp__playwright__browser_evaluate:
  function: () => {
    const O = window.__OMNI__;
    O.help(); O.switchMode("code");
    const tab = O.createTab("event", "Task Events", "📡");
    O.switchTab(tab.id);
    const fn = O.customFunction("handleFilterAll", [], "void");
    O.returnNode(fn.id);
    fn.setSource("action handleFilterAll(event) { activeFilter.set('all') }");
    const r = O.validateFunction(fn.id);
    return { tabs: O.listTabs().length, fns: O.listCustomFunctions().length, valid: r.valid };
  }
```

Expected: `tabs >= 1`, `fns >= 1`, `valid: true`.

- [ ] **Step 7: Final Validation**

```
mcp__playwright__browser_evaluate:
  function: () => {
    const O = window.__OMNI__;
    const report = O.validate();
    return {
      isValid: report.isValid,
      elements: O.elements.length,
      wires: O.wires.length,
      orphans: report.orphanElements.length,
      unwired: report.unwiredEvents.length,
      mismatches: report.typeMismatches.length,
      unreachable: report.unreachableBlocks.length,
    };
  }
```

Expected: `isValid: true`, all issue counts at 0.

- [ ] **Step 8: Take screenshot for visual verification**

```
mcp__playwright__browser_take_screenshot → filename: task-manager-v2-final.png
```

- [ ] **Step 9: Verify with Vision MCP**

```
mcp__agent-vision__analyze_image:
  image: task-manager-v2-final.png
  prompt: "List all visible UI elements and their text content. Check if the editor shows a valid canvas with elements (not blank/placeholder text)."
```

---

## Phase 6: Deprecation Verification

### Task 10: Verify v1 APIs Throw Correct Errors

- [ ] **Step 1: createEventHandler throws**

```
mcp__playwright__browser_evaluate:
  function: () => {
    const O = window.__OMNI__;
    O.switchMode("design"); O.clearAll();
    const btn = O.button("Test");
    try { btn.createEventHandler("onPress"); return "NO_ERROR"; }
    catch(e) { return e.message; }
  }
```

Expected: Message contains "removed in v2" and "bindEvent".

- [ ] **Step 2: codeBlock('action', ...) throws (if applicable)**

```
mcp__playwright__browser_evaluate:
  function: () => {
    const O = window.__OMNI__;
    try { O.codeBlock("action", "test"); return "NO_ERROR"; }
    catch(e) { return e.message; }
  }
```

Expected: Error thrown (action type removed).

---

## Phase 7: Final Gates

### Task 11: All Quality Gates

- [ ] **Step 1: TypeScript compiles with zero errors**

```bash
cd "D:/space/OmniLang/editor/client" && npx tsc --noEmit --pretty false 2>&1
```
Expected: Exit code 0, zero error lines.

- [ ] **Step 2: Vite builds successfully**

```bash
cd "D:/space/OmniLang/editor/client" && npx vite build 2>&1
```
Expected: `✓ built in ...`

- [ ] **Step 3: All 5 E2E spec files pass (8+ tests total)**

```bash
cd "D:/space/OmniLang" && npx playwright test editor/e2e/omni-driver-*.spec.ts --reporter=line 2>&1
```

Expected output (approximate):
```
omni-driver-code-tab.spec.ts:      1 passed
omni-driver-counter.spec.ts:       4 passed
omni-driver-customfn.spec.ts:      1 passed
omni-driver-event-binding.spec.ts: 2 passed
omni-driver-task-manager.spec.ts:  1 passed
Total: 9 passed
```

- [ ] **Step 4: Browser MCP interactive verification passes**

All browser_evaluate calls in Task 9 return expected values.

- [ ] **Step 5: Commit any final fixes**

```bash
cd "D:/space/OmniLang" && git add -A && git status
# Only commit if there are meaningful changes
git commit -m "fix(omni-driver): final v2 verification fixes"
```

---

## Implementation Order

```
Check 0 (git status)
  → Check 1 (ports)
    → Task 1 (tsc)
      → Task 2 (vite build)
        → Task 3 (start infra)
          → Task 4 (event binding test) ─┐
          → Task 5 (code tab test)       ├─ Parallel possible
          → Task 6 (custom fn test)      │  if Playwright supports
          → Task 7 (counter test)        │  concurrent runs
          → Task 8 (task manager test)   ┘
            → Task 9 (Browser MCP interactive)
              → Task 10 (deprecation verify)
                → Task 11 (final gates)
```

Tasks 4-8 can run in parallel if Playwright is configured for concurrent execution. Tasks 9-10 must run sequentially (they share the browser session).

---

## Failure Protocol

**If any E2E test fails:**

1. **DO NOT edit the test file** unless the test expectation is provably wrong
2. **Read the error message carefully** — Playwright errors are descriptive
3. **Reproduce the failing scenario in Browser MCP** — run the same `page.evaluate()` code interactively
4. **Use `O.validate()` + `O.getState()` to diagnose** — check elements, wires, ports
5. **Fix the framework code** (OmniDriver.ts, editor.ts, etc.) — never the generated code
6. **Clear __pycache__ + restart backend** if Python code was changed
7. **Re-run the specific failing test** to verify the fix

**Common failure categories and root causes:**

| Failure | Root Cause Location | Diagnostic Command |
|---------|-------------------|-------------------|
| `TypeError: X is not a function` | Method missing in OmniDriver.ts | Check method name, verify it's added to the class |
| `Expected X but got Y` | Logic error in method | `browser_evaluate` the method in isolation |
| `Timeout waiting for __OMNI__` | Editor not loaded | Check Vite dev server is running on 5174 |
| `page.evaluate: ReferenceError` | Script syntax error | Check for missing imports, typos |
| `Test file can't be parsed` | TypeScript syntax in .ts file | Check imports, type annotations |
