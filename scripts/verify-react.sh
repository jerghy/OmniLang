#!/usr/bin/env bash
#
# Real React gate for generated OmniLang output.
#
# Runs, over the committed harness test/projects/learn-omniview/build/react-test-app:
#   1. tsc --noEmit (strict)        — the whole generated src/ must type-check
#   2. eslint src --max-warnings 0  — react-hooks (rules-of-hooks, exhaustive-deps)
#                                     + @eslint/js + typescript-eslint recommended
#
# Known root-caused errors are documented in scripts/verify-react.exemptions
# (one entry per error code, each referencing the task that fixes the root
# cause — T1.3 style channel, T1.6 function emission, T3.3 store slices).
#
# Gate semantics:
#   - any NON-exempt error  -> GATE FAILED  (that is a new regression)
#   - any UNUSED exemption  -> GATE FAILED  (stale entry — delete it once the
#     root-cause task lands; exemptions must be removed, never silenced)
#
# Prerequisites (see .github/workflows/ci.yml e2e job):
#   cd test/projects/learn-omniview
#   ../../target/release/omnic build --target react -o build/react-test-app

set -uo pipefail

TSC_RE='error[[:space:]]+(TS[0-9]+)'
TSC_PATH_RE='^([^(]+)\('
ESLINT_RE='^(.+):([0-9]+):([0-9]+): .*\[((Error/)?[^]]+)\]$'

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HARNESS="$REPO_ROOT/test/projects/learn-omniview/build/react-test-app"
EXEMPTIONS="$REPO_ROOT/scripts/verify-react.exemptions"

cd "$HARNESS" || {
  echo "FATAL: harness dir missing: $HARNESS"
  echo "       regenerate: cd test/projects/learn-omniview && ../../target/release/omnic build --target react -o build/react-test-app"
  exit 2
}
[ -f App.tsx ] || {
  echo "FATAL: generated output missing (App.tsx / components/ / pages/ / stores/)."
  echo "       regenerate: cd test/projects/learn-omniview && ../../target/release/omnic build --target react -o build/react-test-app"
  exit 2
}

# Gate scope — generated output + the harness entry. Mirrors tsconfig include.
LINT_SCOPE="src App.tsx SimpleApp.tsx useDesignTokens.ts useThemeStore.ts components pages stores"

fail=0

# ---- exemptions (glob, code, task, reason) ----
declare -a EX_GLOB=() EX_CODE=() EX_TASK=() EX_REASON=() EX_USED=()
load_exemptions() {
  local glob code task reason
  while IFS=$'\t' read -r glob code task reason; do
    [[ -z "$glob" || "$glob" == \#* ]] && continue
    EX_GLOB+=("${glob//\*\*/*}") # bash [[ == ]] pattern: '*' crosses '/'
    EX_CODE+=("$code")
    EX_TASK+=("$task")
    EX_REASON+=("$reason")
    EX_USED+=(0)
  done < "$EXEMPTIONS"
}

# is_exempt <path> <code> — returns 0 when the (path, code) pair is exempted
is_exempt() {
  local path="$1" code="$2" i
  for i in "${!EX_GLOB[@]}"; do
    if [[ "$path" == ${EX_GLOB[$i]} && "$code" == "${EX_CODE[$i]}" ]]; then
      EX_USED[$i]=1
      return 0
    fi
  done
  return 1
}

check_unused() {
  local i
  for i in "${!EX_GLOB[@]}"; do
    if [[ "${EX_USED[$i]}" != 1 ]]; then
      echo "STALE EXEMPTION (delete it — error class gone): ${EX_GLOB[$i]} ${EX_CODE[$i]} [${EX_TASK[$i]}] ${EX_REASON[$i]}"
      fail=1
    fi
  done
}

# norm_path <path> — make tool-reported paths comparable to exemption globs:
# absolute/backslash paths (eslint) become relative forward-slash (tsc).
norm_path() {
  local p="${1//\\//}"
  echo "${p##*react-test-app/}"
}

load_exemptions

# ---- 1. tsc --noEmit (strict) ----
echo "==> tsc --noEmit (strict)"
tsc_out="$(npx --no-install tsc --noEmit --pretty false 2>&1)"
tsc_exit=$?
if [ "$tsc_exit" -ne 0 ]; then
  exempted=0
  while IFS= read -r line; do
    if [[ "$line" =~ $TSC_RE ]]; then
      code="${BASH_REMATCH[1]}"
      path=""
      if [[ "$line" =~ $TSC_PATH_RE ]]; then path="$(norm_path "${BASH_REMATCH[1]}")"; fi
      if is_exempt "$path" "$code"; then
        exempted=$((exempted + 1))
      else
        echo "TSC [$code]: $line"
        fail=1
      fi
    elif [[ "$line" == [[:space:]]* ]]; then
      : # indented continuation of a multi-line tsc message (already handled above)
    else
      echo "TSC (unparsed — config/tool failure): $line"
      fail=1
    fi
  done <<< "$tsc_out"
  echo "    $(grep -c 'error TS' <<<"$tsc_out") tsc errors, $exempted exempted"
else
  echo "    tsc clean"
fi

# ---- 2. eslint (react-hooks, zero warnings) ----
echo "==> eslint $LINT_SCOPE --max-warnings 0"
eslint_out="$(npx --no-install eslint $LINT_SCOPE --max-warnings 0 --format unix 2>&1)"
eslint_exit=$?
if [ "$eslint_exit" -ne 0 ]; then
  exempted=0
  while IFS= read -r line; do
    if [[ "$line" =~ $ESLINT_RE ]]; then
      path="$(norm_path "${BASH_REMATCH[1]}")"
      rule="${BASH_REMATCH[4]#Error/}"
      if is_exempt "$path" "$rule"; then
        exempted=$((exempted + 1))
      else
        echo "ESLINT [$rule]: $line"
        fail=1
      fi
    elif [[ -n "$line" && "$line" != *"problems"* ]]; then
      echo "ESLINT (unparsed — config/tool failure): $line"
      fail=1
    fi
  done <<< "$eslint_out"
  echo "    $(grep -cE '^[^ ]+:[0-9]+:[0-9]+:' <<<"$eslint_out") lint problems, $exempted exempted"
else
  echo "    eslint clean"
fi

check_unused

if [ "$fail" -ne 0 ]; then
  echo
  echo "GATE FAILED"
  echo "  - non-exempt errors are NEW regressions in generated output"
  echo "  - stale exemptions must be deleted (root cause already fixed)"
  echo "  - exemption registry: scripts/verify-react.exemptions"
  exit 1
fi

echo
echo "GATE PASSED — all tsc/eslint errors match documented exemptions"
echo "(exemptions are self-removing: delete entries as root-cause tasks land)"
