#!/bin/bash
# eval-run-omnilang.sh — Eval harness runner for omnilang compiler crate
# Usage: bash .claude/evals/eval-run-omnilang.sh [eval-id]
#   No args: run all code-based evals
#   With arg:  run specific eval (c1, c2, c3, r1, r2, etc.)

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OMNILANG_DIR="$REPO_ROOT/omnilang"

cd "$OMNILANG_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass=0
fail=0
warn=0
results=""
BASELINE_CLIPPY=0
BASELINE_UNWRAP=0
BASELINE_TESTS=801
BASELINE_OVER_800=12

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
echo "║  EVAL HARNESS: omnilang compiler crate      ║"
echo "╚══════════════════════════════════════════════╝"
echo ""

# ── C1: Test Suite ──────────────────────────────────
run_c1() {
  echo "── C1: Test Suite (baseline: $BASELINE_TESTS tests) ──"
  local out
  if out=$(cargo test 2>&1); then
    local count
    # Get the unit test result line (not doc-tests). It has the pattern "N passed; 0 failed"
    count=$(echo "$out" | grep "test result:" | grep -v "Doc-tests" | grep "ok" | head -1 | sed -E 's/.* ([0-9]+) passed.*/\1/')
    count=$(echo "$count" | tr -d '[:space:]')
    if [ "$count" -ge "$BASELINE_TESTS" ] 2>/dev/null; then
      grade "C1" "cargo test: $count passed (≥ baseline $BASELINE_TESTS)" "PASS" "$count tests"
    else
      grade "C1" "cargo test: $count passed (< baseline $BASELINE_TESTS)" "FAIL" "$count tests"
    fi
  else
    local failed_info
    failed_info=$(echo "$out" | grep "FAILED" | head -5 | tr '\n' ' ')
    grade "C1" "cargo test" "FAIL" "$failed_info"
  fi
  echo ""
}

# ── C2: Clippy Warnings ─────────────────────────────
run_c2() {
  echo "── C2: Clippy Warnings (baseline: $BASELINE_CLIPPY) ──"
  local out
  local count
  out=$(cargo clippy 2>&1) || true
  count=$(echo "$out" | grep -c "warning:" || true)
  count=$(echo "$count" | tr -d '[:space:]')
  count=${count:-0}
  if [ "$count" -lt "$BASELINE_CLIPPY" ] 2>/dev/null; then
    grade "C2" "Clippy warnings: $count (↓$((BASELINE_CLIPPY - count)) from baseline)" "PASS" "$count warnings"
  elif [ "$count" -eq "$BASELINE_CLIPPY" ] 2>/dev/null; then
    grade "C2" "Clippy warnings: $count (no change)" "WARN" "$count warnings"
  else
    grade "C2" "Clippy warnings: $count (↑$((count - BASELINE_CLIPPY)) from baseline)" "FAIL" "$count warnings"
  fi
  echo ""
}

# ── C3: Release Build ───────────────────────────────
run_c3() {
  echo "── C3: Release Build ──"
  local out
  if out=$(cargo build --release 2>&1); then
    grade "C3" "cargo build --release" "PASS"
  else
    grade "C3" "cargo build --release" "FAIL" "$(echo "$out" | grep "^error" | head -3)"
  fi
  echo ""
}

# ── C4: WASM Build (optional) ───────────────────────
run_c4() {
  echo "── C4: WASM Build ──"
  if ! rustup target list --installed 2>/dev/null | grep -q "wasm32-unknown-unknown"; then
    echo -e "  ${YELLOW}SKIP${NC}  wasm32-unknown-unknown target not installed"
    ((warn++)) || true
    results+="C4|WASM build|SKIP|target not installed"$'\n'
    echo ""
    return
  fi
  local out
  if out=$(cargo build --features wasm --target wasm32-unknown-unknown 2>&1); then
    grade "C4" "WASM build succeeds" "PASS"
  else
    grade "C4" "WASM build fails" "FAIL" "$(echo "$out" | grep "^error" | head -3)"
  fi
  echo ""
}

# ── C5: Compiler Pipeline Smoke Test ────────────────
run_c5() {
  echo "── C5: Compiler Pipeline Smoke Test ──"
  local out
  if out=$(cargo test --lib 2>&1); then
    local count
    count=$(echo "$out" | grep "test result:" | grep "ok" | head -1 | sed -E 's/.* ([0-9]+) passed.*/\1/')
    count=$(echo "$count" | tr -d '[:space:]')
    count=${count:-0}
    if [ "$count" -ge 300 ] 2>/dev/null; then
      grade "C5" "Pipeline smoke tests: $count lib tests pass" "PASS" "$count tests"
    else
      grade "C5" "Pipeline: only $count tests (expected ≥300)" "FAIL" "$count tests"
    fi
  else
    grade "C5" "Pipeline smoke tests" "FAIL" "$(echo "$out" | grep "FAILED" | head -3)"
  fi
  echo ""
}

# ── R1: Unsafe Without Safety Comments ──────────────
run_r1() {
  echo "── R1: Unsafe Blocks Without Safety Comments ──"
  local missing=0
  local missing_lines=""

  while IFS= read -r match; do
    # Parse: file:line:content (only split first two colons)
    local file="${match%%:*}"
    local rest="${match#*:}"
    local line="${rest%%:*}"
    local content="${rest#*:}"

    # Skip test code
    echo "$content" | grep -q "#\[cfg(test)\]" && continue
    # Skip lines that already have SAFETY inline
    echo "$content" | grep -q "// SAFETY:" && continue
    # Skip `unsafe extern`
    echo "$content" | grep -q "unsafe extern" && continue

    # Check preceding 5 lines for // SAFETY: comment
    local found=0
    for offset in 1 2 3 4 5; do
      local check_line=$((line - offset))
      if [ "$check_line" -lt 1 ]; then
        continue
      fi
      if sed -n "${check_line}p" "$file" 2>/dev/null | grep -q "// SAFETY:"; then
        found=1
        break
      fi
    done
    if [ "$found" -eq 0 ]; then
      ((missing++)) || true
      missing_lines+="$file:$line: $content"$'\n'
    fi
  done < <(grep -rn "unsafe {" src/ --include="*.rs" | grep -v "#\[cfg(test)\]" || true)

  if [ "$missing" -eq 0 ]; then
    grade "R1" "All unsafe blocks have SAFETY comments" "PASS"
  else
    grade "R1" "Unsafe blocks missing SAFETY comments" "FAIL" "$missing instances"
    echo "$missing_lines" | head -20
  fi
  echo ""
}

# ── R2: No .unwrap() in Non-Test Code ───────────────
run_r2() {
  echo "── R2: Production .unwrap() Count (clippy, baseline: $BASELINE_UNWRAP) ──"
  local out
  local count
  out=$(cargo clippy -- -W clippy::unwrap_used 2>&1) || true
  count=$(echo "$out" | grep -c "warning: used \`.unwrap()\`" || true)
  count=$(echo "$count" | tr -d '[:space:]')
  count=${count:-0}
  if [ "$count" -lt "$BASELINE_UNWRAP" ] 2>/dev/null; then
    grade "R2" "Production .unwrap() calls: $count (↓$((BASELINE_UNWRAP - count)) from baseline)" "PASS" "$count unwraps"
  elif [ "$count" -eq "$BASELINE_UNWRAP" ] 2>/dev/null; then
    grade "R2" "Production .unwrap() calls: $count (no change)" "WARN" "$count unwraps"
  else
    grade "R2" "Production .unwrap() calls: $count (↑$((count - BASELINE_UNWRAP)) from baseline)" "FAIL" "$count unwraps"
  fi
  echo ""
}

# ── R3: Codegen No `_ => "null"` Fallbacks ──────────
run_r3() {
  echo "── R3: Codegen Wildcard → 'null' Fallbacks ──"
  local matches
  matches=$(grep -rn '_ => "null"' src/codegen/ --include="*.rs" || true)
  if [ -z "$matches" ]; then
    grade "R3" "No 'null' wildcard fallbacks in codegen" "PASS"
  else
    local count
    count=$(echo "$matches" | wc -l)
    grade "R3" "Found wildcard → 'null' in codegen" "FAIL" "$count instances"
    echo "$matches" | head -10
  fi
  echo ""
}

# ── R4: File Sizes ──────────────────────────────────
run_r4() {
  echo "── R4: File Size Limits (<800 lines) ──"
  local over
  over=$(find src -name "*.rs" -exec wc -l {} \; | sort -rn | awk '$1 > 800 {printf "  %5d %s\n", $1, $2}')
  local count
  count=$(echo "$over" | grep -c "." || echo 0)
  count=$(echo "$count" | tr -d '[:space:]')
  if [ "$count" -le "$BASELINE_OVER_800" ] 2>/dev/null; then
    if [ "$count" -lt "$BASELINE_OVER_800" ] 2>/dev/null; then
      grade "R4" "Files >800 lines: $count (↓ from $BASELINE_OVER_800)" "PASS" "$count files over 800"
    else
      grade "R4" "Files >800 lines: $count (no change)" "WARN" "$count files over 800"
    fi
  else
    grade "R4" "Files >800 lines: $count (↑ from $BASELINE_OVER_800)" "FAIL" "$count files over 800"
  fi
  echo "$over" | head -15
  echo ""
}

# ── Top 5 largest files ─────────────────────────────
run_top_files() {
  echo "── Top 10 largest source files ──"
  find src -name "*.rs" -exec wc -l {} \; | sort -rn | head -10
  echo ""
}

# ── Report ──────────────────────────────────────────
report() {
  local total=$((pass + fail + warn))
  echo "══════════════════════════════════════"
  echo -e "  ${GREEN}PASS: $pass${NC}  ${YELLOW}WARN: $warn${NC}  ${RED}FAIL: $fail${NC}  (total: $total)"
  echo "══════════════════════════════════════"

  # Append to log
  local log="$SCRIPT_DIR/omnilang-compiler.log"
  local date_str
  date_str=$(date -Iseconds)
  local sha
  sha=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
  echo "$date_str | $sha | pass=$pass warn=$warn fail=$fail | $(echo "$results" | tr '\n' ' ')" >> "$log"
}

# ── Run specific or all ─────────────────────────────
if [ $# -gt 0 ]; then
  case "$1" in
    c1) run_c1 ;;
    c2) run_c2 ;;
    c3) run_c3 ;;
    c4) run_c4 ;;
    c5) run_c5 ;;
    r1) run_r1 ;;
    r2) run_r2 ;;
    r3) run_r3 ;;
    r4) run_r4 ;;
    top) run_top_files ;;
    *) echo "Unknown eval: $1. Options: c1 c2 c3 c4 c5 r1 r2 r3 r4 top" ;;
  esac
else
  run_c1
  run_c2
  run_c3
  run_c4
  run_c5
  run_r1
  run_r2
  run_r3
  run_r4
  run_top_files
fi

report
