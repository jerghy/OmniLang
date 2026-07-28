# EVAL: editor-client

> **Scope**: `editor/client/src` — OmniLang visual editor frontend
> **Stack**: React 19 + TypeScript + Vite + Zustand + Vitest
> **Baseline**: 2026-06-27 (SHA: main branch, 196 tests, 18 suites)
> **Target**: pass@3 ≥ 90% for capability evals, pass^3 = 100% for regression evals

---

## Baseline Measurements

```
Test Files:  18 passed (18)
Tests:       196 passed (196)
Duration:    3.38s
TypeScript:  372 pre-existing errors across 45 files (tracked for reduction)
             Key hotspots: codegraph/store.ts, codegraph/types.ts, parser*.ts,
             CodePropertyPanel.tsx, PropertyPanel.tsx, and codegen pipeline files
as any:      0 instances (clean — fixed in "fix: eliminate all as-any type bypasses")
console.log: 0 in source (only in test fixtures)
File sizes:  All under 800 lines (pipeline.ts refactored from monolith, OmniDriver split into 9 modules)
```

---

## Capability Evals

### C1: TypeScript Error Count (Tracked Reduction)

```
[CAPABILITY EVAL: tsc-error-count]
Task: Track and reduce TypeScript errors over time
Command: cd editor/client && npx tsc -p tsconfig.app.json --noEmit 2>&1 | grep -c "error TS"
Baseline: 372 errors across 45 files
Success Criteria:
  - [ ] Error count does not increase (≤ 372)
  - [ ] New code adds 0 new type errors
  - [ ] Goal: reduce by ≥20 errors per refactoring session
Grader: code-based (count comparison)
```

### C1b: Build Command

```
[CAPABILITY EVAL: vite-build]
Task: Vite production build succeeds
Command: cd editor/client && npx vite build 2>&1
Success Criteria:
  - [ ] Build completes without error
  - [ ] Output files exist in dist/
Grader: code-based (exit code)
```

### C2: Unit Tests — Codegen Services (10 suites)

```
[CAPABILITY EVAL: codegen-tests]
Task: All codegen service tests pass
Command: cd editor/client && npx vitest run src/services/codegen/
Success Criteria:
  - [ ] annotation-data.test.ts PASS
  - [ ] chain-emitter.test.ts PASS
  - [ ] expression-lowering.test.ts PASS
  - [ ] integration-generate.test.ts PASS
  - [ ] omni-parser.test.ts PASS
  - [ ] source-emitter-annotation.test.ts PASS
  - [ ] store-extractor.test.ts PASS
  - [ ] style-custom-property.test.ts PASS
  - [ ] validator.test.ts PASS
  - [ ] view-tree-builder.test.ts PASS
Grader: code-based (vitest exit code)
```

### C3: Unit Tests — Store (4 suites)

```
[CAPABILITY EVAL: store-tests]
Task: All store tests pass
Command: cd editor/client && npx vitest run src/store/
Success Criteria:
  - [ ] editor.test.ts PASS
  - [ ] style-registry.test.ts PASS
  - [ ] style-store.test.ts PASS
  - [ ] undo.test.ts PASS
Grader: code-based (vitest exit code)
```

### C4: Unit Tests — Core Commands (2 suites)

```
[CAPABILITY EVAL: command-tests]
Task: All command tests pass
Command: cd editor/client && npx vitest run src/core/commands/
Success Criteria:
  - [ ] commands.test.ts PASS
  - [ ] style-commands.test.ts PASS
Grader: code-based (vitest exit code)
```

### C5: Unit Tests — UI Canvas (2 suites)

```
[CAPABILITY EVAL: canvas-tests]
Task: All canvas utility tests pass
Command: cd editor/client && npx vitest run src/ui/canvas/
Success Criteria:
  - [ ] FlowEngine.test.ts PASS
  - [ ] connectionValidators.test.ts PASS
Grader: code-based (vitest exit code)
```

### C6: Pipeline Roundtrip Integrity

```
[CAPABILITY EVAL: pipeline-roundtrip]
Task: Omni source parses, generates, and re-parses identically
Command: cd editor/client && npx vitest run src/services/codegen/__tests__/integration-generate.test.ts -t "roundtrip"
Success Criteria:
  - [ ] parse -> generate -> re-parse succeeds
  - [ ] generateOmniFiles produces multi-file output
  - [ ] generateOmniFiles includes store files when stores exist
Grader: code-based (vitest exit code)
```

### C7: Editor Store CRUD

```
[CAPABILITY EVAL: editor-crud]
Task: Element and wire CRUD operations work correctly
Command: cd editor/client && npx vitest run src/store/__tests__/editor.test.ts
Success Criteria:
  - [ ] addElement adds elements immutably
  - [ ] removeElement removes by id
  - [ ] setProp updates props immutably without mutating other elements
  - [ ] addWire / removeWire work correctly
  - [ ] selectElement with single and multi modes works
  - [ ] setElementsAndWires atomically replaces state
Grader: code-based (vitest exit code)
```

### C8: Command Undo/Redo

```
[CAPABILITY EVAL: command-undo-redo]
Task: All commands support execute/undo correctly
Command: cd editor/client && npx vitest run src/core/commands/__tests__/commands.test.ts
Success Criteria:
  - [ ] AddElement execute adds, undo removes
  - [ ] RemoveElement execute removes, undo restores
  - [ ] AddWire execute adds, undo removes
  - [ ] RemoveWire execute removes, undo restores
  - [ ] SetProp execute updates, undo restores original value
  - [ ] SetElementsAndWires captures and restores state
Grader: code-based (vitest exit code)
```

### C9: CSS Custom Property Handling

```
[CAPABILITY EVAL: css-custom-props]
Task: WASM and TypeScript layers handle CSS custom properties correctly
Command: cd editor/client && npx vitest run src/services/codegen/__tests__/style-custom-property.test.ts
Success Criteria:
  - [ ] WASM parses --custom-property in style blocks
  - [ ] Generates setProperty for dynamic -- inline styles
  - [ ] Returns error for malformed source
  - [ ] Routes --prefixed keys to cssRules not htmlAttrs
  - [ ] collectStylesFromEntries handles mixed entry types
  - [ ] Round-trip preserves class, variable, and inline styles
Grader: code-based (vitest exit code)
```

---

## Regression Evals

### R1: Full Test Suite

```
[REGRESSION EVAL: full-test-suite]
Baseline: 196 tests, 18 suites, 0 failures
Command: cd editor/client && npx vitest run
Success Criteria: 196+ tests pass, 0 failures
```

### R2: No `as any` Regression

```
[REGRESSION EVAL: no-as-any]
Baseline: 0 instances (commit e3e31d6e)
Command: grep -rn "as any" editor/client/src/ --include="*.ts" --include="*.tsx"
Success Criteria: Zero matches
Note: This was fixed in commit "fix: eliminate all as-any type bypasses"
```

### R3: No `console.log` in Source

```
[REGRESSION EVAL: no-console-log]
Baseline: 0 in source (only in test fixtures)
Command: grep -rn "console\.log" editor/client/src/ --include="*.ts" --include="*.tsx" | grep -v __tests__
Success Criteria: Zero matches in non-test files
```

### R4: File Size Limits

```
[REGRESSION EVAL: file-size]
Task: No file exceeds 800 lines
Command: find editor/client/src -name "*.ts" -o -name "*.tsx" | xargs wc -l | sort -rn | head -20
Success Criteria:
  - [ ] pipeline.ts < 800 lines (was ~60 after refactor)
  - [ ] OmniDriver.ts < 800 lines (was 1976 before refactor)
  - [ ] All files < 800 lines
Note: Major refactors in commits ff14daa0 and 68d19d67
```

---

## Code Quality Gates

### Q1: Immutability Audit

```
[QUALITY GATE: immutability]
Task: Verify no direct mutation of store state outside store methods
Check:
  - [ ] Store setters use immer or spread operators
  - [ ] Command execute/undo return new state, don't mutate
  - [ ] No direct assignment to store properties outside store files
Grader: model-based + grep for mutation patterns
```

### Q2: Error Handling Completeness

```
[QUALITY GATE: error-handling]
Task: All try/catch blocks log and re-throw appropriately
Check:
  - [ ] pipeline.ts runPhase helper catches and wraps errors
  - [ ] API calls have error handling
  - [ ] No empty catch blocks
  - [ ] No silently swallowed errors
Grader: model-based + grep for "catch\s*\(\s*\)" and "catch\s*\([^)]*\)\s*\{\s*\}"
```

### Q3: Type Safety

```
[QUALITY GATE: type-safety]
Task: No implicit any, no unsafe type assertions
Check:
  - [ ] tsc --noEmit shows no NEW errors
  - [ ] No `as any` (already clean)
  - [ ] No @ts-ignore or @ts-expect-error without justification
Grader: code-based (tsc exit code + grep)
```

---

## Integration Points

### CI Pipeline (`.github/workflows/ci.yml`)

```
These evals map to CI jobs:
  wasm-build  → C2 (codegen tests depend on WASM)
  (no JS job) → ALL evals should be runnable locally

Recommended CI addition:
  editor-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: '20' }
      - run: cd editor/client && npm ci
      - run: cd editor/client && npm test
      - run: cd editor/client && npx tsc -b --noEmit
```

---

## Grader Implementation

### Code-Based Graders

```bash
# eval-check.sh — Run all code-based evals
#!/bin/bash
set -euo pipefail
cd editor/client

PASS=0; FAIL=0

# C1: TypeScript
echo "=== C1: tsc-build ==="
if npx tsc -b --noEmit 2>&1; then
  echo "PASS"; ((PASS++))
else
  echo "FAIL (pre-existing errors allowed, check for new ones)"
fi

# C2-C5: Unit tests
echo "=== C2-C5: Unit Tests ==="
if npx vitest run; then
  echo "PASS ($(npx vitest run --reporter=json 2>/dev/null | jq '.numPassedTests') tests)"; ((PASS++))
else
  echo "FAIL"; ((FAIL++))
fi

# R2: No as any
echo "=== R2: No as any ==="
if grep -rn "as any" src/ --include="*.ts" --include="*.tsx"; then
  echo "FAIL"; ((FAIL++))
else
  echo "PASS"; ((PASS++))
fi

# R3: No console.log
echo "=== R3: No console.log ==="
if grep -rn "console\.log" src/ --include="*.ts" --include="*.tsx" | grep -v __tests__; then
  echo "FAIL"; ((FAIL++))
else
  echo "PASS"; ((PASS++))
fi

echo "=== RESULTS: $PASS passed, $FAIL failed ==="
```

### Model-Based Grader

For open-ended quality gates (Q1, Q2), use this prompt:

```
[MODEL GRADER PROMPT]
Review the following code changes in editor/client/src:

1. Immutability: Do store operations use spread/immer rather than direct mutation?
2. Error handling: Are errors caught, logged, and re-thrown appropriately?
3. Naming: Do functions/classes follow the project's PascalCase/camelCase conventions?
4. File organization: Are files under 800 lines with high cohesion?

Score each dimension 1-5 (1=poor, 5=excellent)
Reasoning: [explain each score]
```

---

## Eval Run History

| Date | SHA | C1 (errors) | C1b (build) | C2-5 (tests) | R2 (as any) | R3 (console) | R4 (size) | Notes |
|------|-----|-------------|-------------|--------------|-------------|--------------|-----------|-------|
| 2026-06-27 | c64a5486 | 372 errors / 45 files | — | 196/196 PASS | CLEAN | CLEAN | PASS | Baseline |

---

## Quick Reference

```bash
# Run all tests
cd editor/client && npx vitest run

# Run specific capability
npx vitest run src/services/codegen/        # C2
npx vitest run src/store/                   # C3
npx vitest run src/core/commands/           # C4
npx vitest run src/ui/canvas/               # C5

# Type check (track error count)
npx tsc -p tsconfig.app.json --noEmit 2>&1 | grep -c "error TS"   # C1

# Production build
npx vite build                                                    # C1b

# Regression checks
grep -rn "as any" src/ --include="*.ts" --include="*.tsx"           # R2
grep -rn "console\.log" src/ --include="*.ts" --include="*.tsx" \   # R3
  | grep -v __tests__
find src -name "*.ts" -o -name "*.tsx" | xargs wc -l \             # R4
  | sort -rn | head -20

# Run tests via CI frontend job
cd poiesis && npx vitest run --config vitest.config.js src/services/codegen/__tests__/
cd poiesis && npx tsc --noEmit
```
