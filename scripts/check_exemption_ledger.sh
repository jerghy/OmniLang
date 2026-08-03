#!/usr/bin/env bash
#
# Ledger audit for scripts/verify-react.exemptions (task 0.6).
#
# Every exemption entry must reference the deferral that owns its root cause
# (column 5, `#NN` — see .architecture/deferrals.md). A '-' or empty deferral
# means the root cause is not registered anywhere: the exemption cannot be
# traced to a fix task and must not be added without one.
#
# Prints one line per untracked exemption and exits non-zero when any exist;
# empty output (exit 0) means the ledger is fully reconnected.

awk -F'\t' '
  /^#/ { next }
  $5 == "" || $5 == "-" { print "untracked exemption: " $0; bad = 1 }
  END { if (bad) exit 1 }
' "$(dirname "$0")/verify-react.exemptions"
