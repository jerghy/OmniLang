#!/bin/bash
# eval-run.sh — Eval harness runner for editor/client
# Usage: bash .claude/evals/eval-run.sh [eval-id]
#   No args: run all code-based evals
#   With arg:  run specific eval (c1, c2, r2, etc.)

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CLIENT_DIR="$(cd "$SCRIPT_DIR/../../editor/client" && pwd)"

cd "$CLIENT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass=0
fail=0
warn=0
results=""
BASELINE_ERRORS=372

grade() {
  local id="$1"
  local label="$2"
  local status="$3"
  local detail="${4:-}"
  if [ "$status" = "PASS" ]; then
    echo -e "  ${GREEN}PASS${NC}  $label"
    ((pass++)) || true
    results+="$id|$label|PASS|$detail"$'\n'
  elif [ "$status" = "WARN" ]; then
    echo -e "  ${YELLOW}WARN${NC}  $label"
    ((warn++)) || true
    results+="$id|$label|WARN|$detail"$'\n'
  else
    echo -e "  ${RED}FAIL${NC}  $label"
    ((fail++)) || true
    results+="$id|$label|FAIL|$detail"$'\n'
  fi
}

echo "╔══════════════════════════════════════════════╗"
echo "║  EVAL HARNESS: editor/client                ║"
echo "╚══════════════════════════════════════════════╝"
echo ""

# ── C1: TypeScript Error Count ──────────────────────
run_c1() {
  echo "── C1: TypeScript Error Count (baseline: $BASELINE_ERRORS) ──"
  local out
  local count
  out=$(npx tsc -p tsconfig.app.json --noEmit 2>&1) || true
  count=$(echo "$out" | grep -c "error TS" || true)
  # Strip whitespace
  count=$(echo "$count" | tr -d '[:space:]')
  count=${count:-0}
  if [ "$count" -lt "$BASELINE_ERRORS" ] 2>/dev/null; then
    grade "C1" "Type errors: $count (↓$((BASELINE_ERRORS - count)) from baseline)" "PASS" "$count errors"
  elif [ "$count" -eq "$BASELINE_ERRORS" ] 2>/dev/null; then
    grade "C1" "Type errors: $count (no change)" "WARN" "$count errors"
  else
    grade "C1" "Type errors: $count (↑$((count - BASELINE_ERRORS)) from baseline)" "FAIL" "$count errors"
  fi
  echo ""
}

# ── C2-C5: Full Test Suite ──────────────────────────
run_tests() {
  echo "── C2-C5: Full Test Suite ──"
  local out
  if out=$(npx vitest run --reporter=verbose 2>&1); then
    # Count from the summary line like "Tests  196 passed (196)"
    local total
    total=$(echo "$out" | grep "Tests" | grep "passed" | tail -1 | sed 's/.*Tests[[:space:]]*\([0-9]*\)[[:space:]]*passed.*/\1/')
    grade "C2-5" "vitest run (${total:-all} tests passed)" "PASS" "${total:-?} tests"
  else
    local failed_info
    failed_info=$(echo "$out" | grep "FAIL" | head -5 | tr '\n' ' ')
    grade "C2-5" "vitest run" "FAIL" "$failed_info"
  fi
  echo ""
}

# ── R2: No `as any` ─────────────────────────────────
run_r2() {
  echo "── R2: No 'as any' Type Bypasses ──"
  local matches
  matches=$(grep -rn "as any" src/ --include="*.ts" --include="*.tsx" 2>/dev/null || true)
  if [ -z "$matches" ]; then
    grade "R2" "No 'as any' in source" "PASS"
  else
    grade "R2" "Found 'as any' bypasses" "FAIL" "$(echo "$matches" | wc -l) instances"
    echo "$matches"
  fi
  echo ""
}

# ── R3: No `console.log` ────────────────────────────
run_r3() {
  echo "── R3: No 'console.log' in Source ──"
  local matches
  matches=$(grep -rn "console\.log" src/ --include="*.ts" --include="*.tsx" | grep -v __tests__ || true)
  if [ -z "$matches" ]; then
    grade "R3" "No console.log in source" "PASS"
  else
    grade "R3" "Found console.log in source" "FAIL" "$(echo "$matches" | wc -l) instances"
    echo "$matches"
  fi
  echo ""
}

# ── R4: File Size ───────────────────────────────────
run_r4() {
  echo "── R4: File Size Limits (<800 lines) ──"
  local bad
  bad=$(find src -name "*.ts" -o -name "*.tsx" | while read -r f; do
    lines=$(wc -l < "$f")
    if [ "$lines" -gt 800 ]; then
      echo "  $lines $f"
    fi
  done)
  if [ -z "$bad" ]; then
    grade "R4" "All files < 800 lines" "PASS"
  else
    grade "R4" "Files over 800 lines" "FAIL" "$bad"
  fi
  echo ""
}

# ── Top 5 largest files ─────────────────────────────
run_top_files() {
  echo "── Top 5 largest source files ──"
  find src -name "*.ts" -o -name "*.tsx" | xargs wc -l 2>/dev/null | sort -rn | head -6
  echo ""
}

# ── Report ──────────────────────────────────────────
report() {
  local total=$((pass + fail + warn))
  echo "══════════════════════════════════════"
  echo -e "  ${GREEN}PASS: $pass${NC}  ${YELLOW}WARN: $warn${NC}  ${RED}FAIL: $fail${NC}  (total: $total)"
  echo "══════════════════════════════════════"

  # Append to log
  local log="$SCRIPT_DIR/editor-client.log"
  local date_str
  date_str=$(date -Iseconds)
  local sha
  sha=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
  echo "$date_str | $sha | pass=$pass warn=$warn fail=$fail | $(echo "$results" | tr '\n' ' ')" >> "$log"
}

# ── Dispatch ────────────────────────────────────────
run_c1
run_tests
run_r2
run_r3
run_r4
run_top_files
report
