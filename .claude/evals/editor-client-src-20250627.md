# EVAL DEFINITION: editor-client-src

**Target:** `D:\space\OmniLang\editor\client\src`
**Date:** 2026-06-27
**Baseline:** `main` branch (uncommitted changes present)
**Framework:** Eval-Driven Development (EDD)

---

## CAPABILITY EVALS

### C1: TypeScript type check passes cleanly
**Grader:** Code-based (`tsc --noEmit`)
**Command:** `cd editor/client && npx tsc --noEmit`
**Success:** Exit code 0, zero errors

### C2: Vite production build succeeds
**Grader:** Code-based (`vite build`)
**Command:** `cd editor/client && npx vite build`
**Success:** Exit code 0, no module resolution errors

### C3: All test suites pass
**Grader:** Code-based (`vitest run`)
**Command:** `cd editor/client && npx vitest run`
**Success:** All test files pass (0 failures), all individual tests pass

### C4: ESLint produces zero errors
**Grader:** Code-based (`eslint src`)
**Command:** `cd editor/client && npx eslint src`
**Success:** Exit code 0, 0 errors, 0 warnings

### C5: No file exceeds 800-line limit
**Grader:** Code-based (wc -l)
**Command:** `find src -name '*.ts' -o -name '*.tsx' | xargs wc -l | awk '$1 > 800 {print $2, $1}'`
**Success:** Zero files found

### C6: No `as any` type bypasses
**Grader:** Code-based (grep)
**Command:** `grep -rP 'as any\b' src/`
**Success:** Zero matches

### C7: No `console.log` in production code
**Grader:** Code-based (grep)
**Command:** `grep -rP 'console\.(log|debug|warn)' src/`
**Success:** Zero matches (or only in documented debug panels)

---

## REGRESSION EVALS

### R1: Existing test count maintained (≥111 passing tests)
**Baseline:** 111 tests pass across 12 test files
**Metric:** pass^1 (100% of previously passing tests still pass)

### R2: Type safety maintained (0 type errors)
**Baseline:** tsc --noEmit = clean
**Metric:** pass^1

### R3: Build integrity maintained (vite build succeeds)
**Baseline:** Previously passing (assumed)
**Metric:** pass^1

---

## SUCCESS METRICS

| Metric | Target | Scope |
|--------|--------|-------|
| pass@1 | > 80% | Capability evals |
| pass^3 | 100% | Regression evals |
| Test pass rate | 100% | All test suites |

---
