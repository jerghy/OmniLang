#!/usr/bin/env python3
"""Syntax-lifecycle gate — design P6 mechanism (v3 plan Task 7.4).

docs/superpowers/specs/syntax-lifecycle.md is the syntax lifecycle
institution: a new syntax enters the BNF only after 提案 (proposal) → spike →
语料试点 (corpus pilot) → 收口评审 (closure review) → BNF 批准. The lesson
(design P6): v2 domain 8 added the language methods filter/map/count straight
into the BNF with 0/93 corpus consumers — deletion had discipline (hard
diagnostics + migration hints), addition had none. This gate makes the
three-stage record the minimal machine-checkable contract:

  stages       — the doc must carry the four lifecycle stages
                 (提案 / spike / 语料试点 / 收口评审) plus BNF 批准
  criteria     — the 批准标准 section must carry the three approval
                 criteria: ≥1 demo 使用（语料试点）+ 三目标门禁
                 (React tsc+eslint / Flutter dart analyze / OmniView
                 browser) + 使用基准预估
  records      — every approved syntax in the registry table must have
                 non-empty 提案记录 / 语料试点 / 批准记录 cells
                 (「提案 → 试点 → 批准」三段记录 — a BNF entry without a
                 proposal trace fails the gate)
  traceability — every 语料试点 cell must reference ≥1 backticked file
                 path that resolves under the repo (the demo evidence
                 must exist on disk, not just be claimed)
  move-sync    — moving a learn-omniview source file REQUIRES syncing the
                 demo path in the lifecycle doc (7.5b-S1 lesson: 11 files
                 moved components/→lib/, spec paths were manually synced).
                 The gate checks every pilot path against the LIVE tree
                 AND, when a path under src/ is missing, searches the live
                 tree for the same basename and names the new location in
                 the diagnostic — the gate stays red in the same CI run
                 as the move until the spec reference is synced. No
                 hardcoded stale-path table to maintain: the hint is
                 derived from the live tree at check time.

A syntax added to the BNF without a lifecycle record — or a record whose
pilot evidence file is missing — fails the gate, so the filter/map/count
class (0-consumer syntax shipped in the BNF) cannot recur.

Usage:
  python scripts/verify-syntax-lifecycle.py [--check|--inspect]
  (default)  gate mode — exit 1 on any mismatch
  --check    gate mode, explicit
  --inspect  audit mode — never exit nonzero
"""
import argparse
import glob
import os
import re
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SPECS_DIR = os.path.join(REPO_ROOT, "docs", "superpowers", "specs")

# ── document contract ─────────────────────────────────────────────────────

# The four lifecycle stages, in order, as '### 阶段 N: <keyword>' headings.
STAGES = [
    ("阶段 1", "提案"),
    ("阶段 2", "Spike"),
    ("阶段 3", "语料试点"),
    ("阶段 4", "收口评审"),
]
APPROVAL_HEADING = "批准标准"
REGISTRY_HEADING = "登记册"
# BNF approval must be a real heading — a plain-text mention or a deleted
# stage subsection would otherwise pass a whole-text substring check.
BNF_APPROVAL_HEADING_RE = re.compile(r"^### BNF 批准", re.M)
# Approval-criteria keywords that must appear in the 批准标准 section.
CRITERIA_KEYWORDS = ("demo 使用", "三目标门禁", "使用基准")
# Registry table columns — the three-stage record contract.
COLUMN_PROPOSAL = "提案记录"
COLUMN_PILOT = "语料试点"
COLUMN_APPROVAL = "批准记录"
COLUMN_BNF = "BNF 事实源"
REQUIRED_COLUMNS = (COLUMN_BNF, COLUMN_PROPOSAL, COLUMN_PILOT, COLUMN_APPROVAL)
# Pilot cells must reference at least one path-like file token (backticked,
# containing '/', with a known source extension) that exists under the repo.
FILE_TOKEN_RE = re.compile(r"`([^`\n]+/[^`\n]+\.(?:omni|rs|md))`")
# Backfilled legacy rows are exempt from the "≥1 demo" pilot rule only when
# the cell says so explicitly (documented one-time backfill).
PILOT_EXEMPT_KEYWORD = "补档豁免"
# learn-omniview demo tree — the canonical corpus whose file moves must be
# reflected in spec paths (move-sync rule, see module docstring).
LEARN_SRC_DIR = os.path.join(REPO_ROOT, "test", "projects", "learn-omniview", "src")
LEARN_SRC_PREFIX = "test/projects/learn-omniview/src/"


def read_file(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


def table_rows(text):
    """Markdown table → (headers, rows). Malformed tables return (None, None)."""
    headers = None
    rows = []
    seen_separator = False
    for line in text.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            if headers is not None:
                break
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if headers is None:
            headers = [c for c in cells]
            continue
        if all(re.fullmatch(r":?-{2,}:?", c or "-") for c in cells):
            seen_separator = True
            continue
        rows.append(cells)
    if not headers or not seen_separator:
        return None, None
    for cells in rows:
        if not cells or not cells[0]:
            return None, None
    return headers, rows


def subsection(text, header):
    # Heading may carry a parenthetical suffix ('## 登记册（已批准语法——…）').
    m = re.search(rf'^## {re.escape(header)}[^\n]*\n(.*?)(?=^## |\Z)', text, re.M | re.S)
    return m.group(1) if m else None


def find_moved_location(tok):
    """Move-sync hint: if a learn-omniview pilot path no longer exists but a
    file with the same basename lives elsewhere under the live src tree,
    return its repo-relative location. Derives the 'old → new' mapping from
    the live tree itself — no hardcoded table to go stale."""
    if not tok.startswith(LEARN_SRC_PREFIX):
        return None
    name = os.path.basename(tok)
    for root, _dirs, files in os.walk(LEARN_SRC_DIR):
        if name in files:
            rel = os.path.relpath(os.path.join(root, name), REPO_ROOT).replace("\\", "/")
            if rel != tok:
                return rel
    return None


def audit_file(path, problems):
    """Audit one lifecycle doc. `problems` is the shared collector list."""
    text = read_file(path)
    rel = os.path.relpath(path, REPO_ROOT)

    def fail(msg):
        problems.append(f"{rel}: {msg}")

    # ── four lifecycle stages ────────────────────────────────────────────
    for num, keyword in STAGES:
        if not re.search(rf'^### {re.escape(num)}[^\n]*{re.escape(keyword)}', text, re.M):
            fail(f"missing stage heading '### {num}: …{keyword}…'")
    if not BNF_APPROVAL_HEADING_RE.search(text):
        fail(f"missing BNF approval stage ('### BNF 批准' heading)")

    # ── approval criteria section ────────────────────────────────────────
    criteria = subsection(text, APPROVAL_HEADING)
    if criteria is None:
        fail(f"missing '## {APPROVAL_HEADING}' section")
    else:
        for kw in CRITERIA_KEYWORDS:
            if kw not in criteria:
                fail(f"'## {APPROVAL_HEADING}' missing approval criterion keyword '{kw}'")

    # ── registry table: three-stage records per approved syntax ─────────
    registry = subsection(text, REGISTRY_HEADING)
    if registry is None:
        fail(f"missing '## {REGISTRY_HEADING}' section (approved-syntax registry)")
        return
    headers, rows = table_rows(registry)
    if headers is None or rows is None or not rows:
        fail(f"'## {REGISTRY_HEADING}' must contain a well-formed table (header + separator + ≥1 row, no empty first column)")
        return
    idx = {}
    for col in REQUIRED_COLUMNS:
        if col not in headers:
            fail(f"registry table missing required column '{col}' (got {headers})")
    if any(col not in headers for col in REQUIRED_COLUMNS):
        return
    for col in REQUIRED_COLUMNS:
        idx[col] = headers.index(col)

    groups = [r[0] for r in rows]
    if len(groups) != len(set(groups)):
        fail(f"duplicate 语法组 entries {sorted({w for w in groups if groups.count(w) > 1})}")

    for row in rows:
        name = row[0]
        # 三段记录: 提案 → 试点 → 批准 — every approved syntax must carry all three.
        for col in REQUIRED_COLUMNS:
            if not row[idx[col]].strip():
                fail(f"approved syntax '{name}': missing {col} (「提案 → 试点 → 批准」三段记录 required)")
        pilot = row[idx[COLUMN_PILOT]]
        # Pilot evidence must exist: every 语料试点 cell cites ≥1 resolvable
        # file path, unless the row is an explicitly-marked backfill exemption.
        tokens = FILE_TOKEN_RE.findall(pilot)
        if not tokens:
            if PILOT_EXEMPT_KEYWORD not in pilot:
                fail(f"approved syntax '{name}': 语料试点 must cite ≥1 existing file path (backticked path ending .omni/.rs/.md)")
            continue
        missing = []
        hints = []
        for tok in tokens:
            if not os.path.exists(os.path.join(REPO_ROOT, tok)):
                missing.append(tok)
                new_loc = find_moved_location(tok)
                if new_loc:
                    hints.append(f"{tok} → {new_loc}")
        if missing:
            msg = f"approved syntax '{name}': 语料试点 cites missing file(s): {sorted(set(missing))}"
            if hints:
                msg += f" — 文件已移动？请同步 spec 中的 demo 路径: {'; '.join(hints)}"
            fail(msg)


def audit():
    problems = []
    matches = sorted(glob.glob(os.path.join(SPECS_DIR, "*lifecycle*.md")))
    if not matches:
        problems.append(
            f"no lifecycle doc found under {os.path.relpath(SPECS_DIR, REPO_ROOT)} "
            f"(expected 'docs/superpowers/specs/*lifecycle*.md')"
        )
        return problems
    for path in matches:
        audit_file(path, problems)
    return problems


def main():
    # UTF-8 output on Windows (GBK) consoles so CJK diagnostics print cleanly.
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8")
        except (AttributeError, ValueError):
            pass
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="exit 1 on failure (gate mode)")
    ap.add_argument("--inspect", action="store_true", help="never exit nonzero (audit mode)")
    args = ap.parse_args()

    problems = audit()
    print(f"\n=== syntax-lifecycle gate: {problems and len(problems) or 0} problem(s) ===")
    for p in problems:
        print(f"FAIL {p}")
    if problems and not args.inspect:
        print("\nGATE FAILED: an approved syntax lacks its 提案 → 试点 → 批准 record, or the pilot evidence is missing/stale (moved file — sync the spec path).")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
