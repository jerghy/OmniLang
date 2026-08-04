#!/usr/bin/env python3
"""Vocabulary-registry gate — design P1 recurrence mechanism (v3 plan Task 7.1).

docs/superpowers/specs/vocabulary-registry.md is the closed-vocabulary
registry of the semantic DSL: five closed vocabulary domains (events / a11y
role / motion properties / RefOps / design-token types). Every entry must
answer 「词汇的语义域是什么」with four required fields:

  语义词汇（table, first column = the semantic vocabulary word）
  来源域    (paragraph — the authoritative semantic domain)
  目标映射  (table — React / Flutter / OmniView mapping shapes)
  排除词汇  (table, first column = platform flag names that must NOT become
             semantic vocabulary — the borrow-word blacklist)

The cross-assertions pin the registry to the compiler's actual whitelists —
the same ground truth the codegen and semantic passes consume:

  events      ↔ canonical names in crates/omnilang-core/src/codegen/shared/event_names.rs
                (4.2 事件白名单 — the shared event-name table)
  a11y role   ↔ FLUTTER_ROLE_SUBSET / LEGACY_FLAG_ROLES / LIVE_VALUES in
                crates/omnilang-core/src/semantic/validate_a11y.rs
                (4.3 a11y 词汇 — WAI-ARIA-canonical roles)
  motion      ↔ semantic_anim_prop production in the BNF spec
                (19 属性白名单, ADR-053 D3)
  RefOp       ↔ RefOpIR variants in crates/omnilang-core/src/ir/types.rs
                (RefOp 10 ops)
  token type  ↔ token_type production in the BNF spec
                (13 类型 — parser-accepted lowercase names)

A word added to any compiler whitelist forces the registry to learn it and
vice versa — the registry can never drift from the whitelists, and a platform
flag name can never slip in as semantic vocabulary without failing the
borrow-word check (排除词汇 must be disjoint from 语义词汇 and, for a11y,
must equal the legacy-flag list exactly).

Usage:
  python scripts/verify-vocabulary.py [--check|--inspect] [-v]
  (default)  gate mode — exit 1 on any mismatch
  --check    gate mode, explicit
  --inspect  audit mode — never exit nonzero
"""
import argparse
import os
import re
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REGISTRY_FILE = os.path.join(REPO_ROOT, "docs", "superpowers", "specs", "vocabulary-registry.md")
EVENT_TABLE_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "codegen", "shared", "event_names.rs"
)
A11Y_VALIDATE_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "semantic", "validate_a11y.rs"
)
REFOP_IR_FILE = os.path.join(REPO_ROOT, "crates", "omnilang-core", "src", "ir", "types.rs")
BNF_SPEC_FILE = os.path.join(
    REPO_ROOT, "docs", "superpowers", "specs",
    "2026-07-31-semantic-level-dsl-syntax-spec.md",
)

# ── registry section contract ────────────────────────────────────────────
# Domain titles must contain the expected keyword (in order 1..5).
DOMAIN_TITLES = {
    1: "事件",
    2: "a11y",
    3: "motion",
    4: "RefOp",
    5: "design token",
}
# Required sub-section headers per domain (the four entry fields).
REQUIRED_SUBSECTIONS = ("来源域", "语义词汇", "目标映射", "排除词汇")
# a11y-only: the Flutter-mappable role subset and the live value domain.
A11Y_EXTRA_SUBSECTIONS = ("role 子集", "live 值域")

# ── whitelist extraction (compiler ground truth) ─────────────────────────

def read_file(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


def event_canonical_names(text):
    """Canonical event names from the shared event table — every
    `... => Some("name")` match arm (4.2 事件白名单)."""
    return sorted(set(re.findall(r'=>\s*Some\("([^"]+)"\)', text)))


def rust_str_array(text, const_name):
    """Words of a `const NAME: &[&str] = &["a", "b", ...];` array literal."""
    m = re.search(rf'{re.escape(const_name)}[^=]*=\s*&\[(.*?)\]', text, re.S)
    return sorted(set(re.findall(r'"([^"]+)"', m.group(1)))) if m else None


def legacy_flag_roles(text):
    """(flag_name, canonical_role) pairs of `LEGACY_FLAG_ROLES`."""
    m = re.search(r'LEGACY_FLAG_ROLES[^=]*=\s*&\[(.*?)\]', text, re.S)
    return re.findall(r'\("([^"]+)",\s*"([^"]+)"\)', m.group(1)) if m else None


def refop_variants(text):
    """RefOpIR enum variant names (RefOp 10 ops)."""
    m = re.search(r'pub enum RefOpIR \{(.*?)\n\}', text, re.S)
    if not m:
        return None
    return sorted({v for v in re.findall(r'^    ([A-Z][A-Za-z]+)\b', m.group(1), re.M)})


def bnf_production_words(text, lhs):
    """Quoted single words of a BNF production and its `|` continuation
    lines — `lhs ::= 'a' | 'b' ...` with following lines starting with `|`."""
    lines = text.splitlines()
    for i, line in enumerate(lines):
        if re.match(rf'^{re.escape(lhs)}\s*::=', line.strip()):
            words = []
            j = i
            while j < len(lines) and (j == i or lines[j].lstrip().startswith("|")):
                words += re.findall(r"'([A-Za-z]+)'", lines[j])
                j += 1
            return sorted(set(words))
    return None


# ── registry parsing ─────────────────────────────────────────────────────

def split_domains(text):
    """`## 域 N: <title>` sections → {N: (title, body)}."""
    parts = re.split(r'^## 域 (\d): ([^\n]+)', text, flags=re.M)
    sections = {}
    for i in range(1, len(parts), 3):
        sections[int(parts[i])] = (parts[i + 1], parts[i + 2])
    return sections


def subsection(body, header):
    m = re.search(rf'^### {re.escape(header)}\s*\n(.*?)(?=^### |\Z)', body, re.M | re.S)
    return m.group(1) if m else None


def first_column_rows(text):
    """Rows of the first markdown table in `text` (header + separator row
    excluded) → first-column cell values. Malformed tables return None."""
    rows = []
    seen_header = False
    seen_separator = False
    for line in text.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            if seen_header:
                break
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if not seen_header:
            seen_header = True
            continue
        if all(re.fullmatch(r":?-{2,}:?", c or "-") for c in cells):
            seen_separator = True
            continue
        rows.append(cells)
    if not seen_header or not seen_separator:
        return None
    return [cells[0] for cells in rows if cells and cells[0]]


# ── audit ────────────────────────────────────────────────────────────────

def audit(verbose):
    problems = []

    def fail(msg):
        problems.append(msg)

    if not os.path.exists(REGISTRY_FILE):
        fail(f"registry missing: {os.path.relpath(REGISTRY_FILE, REPO_ROOT)}")
        return problems

    registry = read_file(REGISTRY_FILE)
    sections = split_domains(registry)
    for n in range(1, 6):
        if n not in sections:
            fail(f"domain {n} missing (expected '## 域 {n}: …{DOMAIN_TITLES[n]}…')")
            continue
        title, body = sections[n]
        if DOMAIN_TITLES[n] not in title:
            fail(f"domain {n} title '{title}' missing expected keyword '{DOMAIN_TITLES[n]}'")
        for header in REQUIRED_SUBSECTIONS:
            sub = subsection(body, header)
            if sub is None:
                fail(f"domain {n} ({title}): missing '### {header}'")
                continue
            if header == "来源域":
                if not sub.strip():
                    fail(f"domain {n} ({title}): '### {header}' is empty")
                continue
            words = first_column_rows(sub)
            if words is None or not words:
                fail(f"domain {n} ({title}): '### {header}' must be a table with ≥1 row")
                continue
            if header == "语义词汇" and len(words) != len(set(words)):
                fail(f"domain {n} ({title}): duplicate 语义词汇 {sorted({w for w in words if words.count(w) > 1})}")
            if header == "排除词汇" and len(words) != len(set(words)):
                fail(f"domain {n} ({title}): duplicate 排除词汇 {sorted({w for w in words if words.count(w) > 1})}")

    # ── cross-assertions against compiler whitelists ────────────────────
    def vocab(n):
        """First-column words of the 语义词汇 table of domain n (or None)."""
        if n not in sections:
            return None
        sub = subsection(sections[n][1], "语义词汇")
        return first_column_rows(sub)

    def excluded(n):
        if n not in sections:
            return None
        sub = subsection(sections[n][1], "排除词汇")
        return first_column_rows(sub)

    def assert_set(domain_label, got, expected, what):
        if got is None or expected is None:
            fail(f"domain {domain_label}: cannot cross-assert {what} (source extraction failed)")
            return
        got_s, exp_s = sorted(set(got)), sorted(set(expected))
        if got_s != exp_s:
            fail(
                f"domain {domain_label}: {what} ≠ whitelist\n"
                f"    registry-only:   {sorted(set(got_s) - set(exp_s))}\n"
                f"    whitelist-only:  {sorted(set(exp_s) - set(got_s))}"
            )

    # events (4.2): registry vocabulary == shared event table canonical names
    ev = vocab(1)
    if ev is not None:
        assert_set("事件", ev, event_canonical_names(read_file(EVENT_TABLE_FILE)), "语义词汇")
    ex_ev = excluded(1)
    if ev is not None and ex_ev is not None:
        overlap = sorted(set(ev) & set(ex_ev))
        if overlap:
            fail(f"domain 事件: 排除词汇 overlaps 语义词汇 (借词违规): {overlap}")

    # a11y (4.3): primitives == a11y_entry BNF; role 子集 == FLUTTER_ROLE_SUBSET;
    # live 值域 == LIVE_VALUES; 排除词汇 == legacy flag names
    if 2 in sections:
        title, body = sections[2]
        a11y_src = read_file(A11Y_VALIDATE_FILE)
        assert_set("a11y", vocab(2), bnf_production_words(read_file(BNF_SPEC_FILE), "a11y_entry"), "语义词汇")
        role_sub = first_column_rows(subsection(body, "role 子集"))
        if role_sub is None:
            fail(f"domain 2 ({title}): missing '### role 子集' table")
        else:
            assert_set("a11y", role_sub, rust_str_array(a11y_src, "FLUTTER_ROLE_SUBSET"), "role 子集")
        live_sub = first_column_rows(subsection(body, "live 值域"))
        if live_sub is None:
            fail(f"domain 2 ({title}): missing '### live 值域' table")
        else:
            assert_set("a11y", live_sub, rust_str_array(a11y_src, "LIVE_VALUES"), "live 值域")
        legacy = legacy_flag_roles(a11y_src)
        ex_a11y = excluded(2)
        if ex_a11y is not None:
            if legacy is None:
                fail("domain a11y: cannot cross-assert 排除词汇 (LEGACY_FLAG_ROLES extraction failed)")
            else:
                assert_set("a11y", ex_a11y, [name for name, _ in legacy], "排除词汇")

    # motion: registry vocabulary == BNF semantic_anim_prop (19 属性白名单)
    assert_set("motion", vocab(3), bnf_production_words(read_file(BNF_SPEC_FILE), "semantic_anim_prop"), "语义词汇")

    # RefOp: registry vocabulary (casefolded) == RefOpIR variants (10 ops)
    refop = vocab(4)
    if refop is not None:
        variants = refop_variants(read_file(REFOP_IR_FILE))
        if variants is None:
            fail("domain RefOp: cannot cross-assert 语义词汇 (RefOpIR extraction failed)")
        else:
            assert_set("RefOp", [w.casefold() for w in refop], [v.casefold() for v in variants], "语义词汇")

    # design token: registry vocabulary == BNF token_type (13 类型)
    assert_set("design token", vocab(5), bnf_production_words(read_file(BNF_SPEC_FILE), "token_type"), "语义词汇")

    # borrow-word rule across all domains: 排除词汇 must be disjoint from 语义词汇
    for n, label in ((1, "事件"), (2, "a11y"), (3, "motion"), (4, "RefOp"), (5, "design token")):
        v, x = vocab(n), excluded(n)
        if v is not None and x is not None:
            overlap = sorted(set(v) & set(x))
            if overlap:
                fail(f"domain {n} ({label}): 排除词汇 overlaps 语义词汇 (借词违规): {overlap}")
    return problems


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="exit 1 on failure (gate mode)")
    ap.add_argument("--inspect", action="store_true", help="never exit nonzero (audit mode)")
    ap.add_argument("--verbose", "-v", action="store_true")
    args = ap.parse_args()

    problems = audit(args.verbose)
    print(f"\n=== vocabulary-registry gate: {problems and len(problems) or 0} problem(s) ===")
    for p in problems:
        print(f"FAIL {p}")
    if problems and not args.inspect:
        print("\nGATE FAILED: registry/vocabulary drifted from the compiler whitelists.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
