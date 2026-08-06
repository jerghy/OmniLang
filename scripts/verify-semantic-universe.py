#!/usr/bin/env python3
"""Semantic-universe gate — design P3 mechanism (v3 plan Task 7.2).

docs/superpowers/specs/semantic-universe.md is the semantic full set: the
FRONTEND FEATURE UNIVERSE (independent of implementation progress), split
from the implemented set. Five semantic domains (animatable properties /
array operations / events / RefOps / design-token types). Every entry must
map to an implementation status — the brief's 语义集条目 → 实现状态矩阵:

  语义域定义  (paragraph — the semantic domain; why these entries are one
               feature universe, independent of implementation)
  语义集条目  (table — first column = semantic-set member; columns 实现状态
               and 诊断行为 are REQUIRED; 状态 ∈ {支持, 部分, 未支持};
               every entry must have a status and a diagnostic behavior)
  已实现白名单(table — first column = the compiler's implemented whitelist;
               must equal the compiler ground truth AND be a subset of the
               semantic set, with 支持/部分 entries in the whitelist and
               未支持 entries outside it — the semantic surface never
               drifts with implementation progress)

Cross-assertions pin the universe to the compiler's actual whitelists — the
same ground truth verify-vocabulary.py (Task 7.1) consumes:

  motion  ↔ SEMANTIC_ANIM_PROPS in
            crates/omnilang-core/src/parser/top_level/motion.rs
            (19 属性白名单 — the implemented subset of the animatable set;
            v4 D-V4-9: parser 常量是事实源，原 BNF semantic_anim_prop
            production 降级为「文档 ⊆ 代码」单向检查)
  array   ↔ ARRAY_METHODS in crates/omnilang-core/src/semantic/validate_array_methods.rs
            (filter/map/count — the implemented language methods)
  events  ↔ canonical names in crates/omnilang-core/src/codegen/shared/event_names.rs
            (54 canonical event names); 域 3「支持 无（全目标）」行 ⊆ Flutter
            callback map keys in omnilang/src/codegen/flutter/utils.rs (v4 5.1 —
            台账「全目标支持」以编译器的 Flutter 映射事实为源，不再自证)
  RefOp   ↔ RefOpIR variants in crates/omnilang-core/src/ir/types.rs
            (RefOp 10 ops)
  token   ↔ parse_token_type match arms in
            crates/omnilang-core/src/parser/design.rs
            (13 types — parser-accepted lowercase names; v4 同重指向)

A whitelist word missing from the universe — or a universe entry without a
status, or an entry claiming 支持/部分 while outside the whitelist — fails
the gate. A semantic-set entry may legitimately be 未支持 (the universe
documents the full feature space, e.g. CSS-animatable properties beyond the
19); the diagnostic-behavior column records the current surface.

Usage:
  python scripts/verify-semantic-universe.py [--check|--inspect]
  (default)  gate mode — exit 1 on any mismatch
  --check    gate mode, explicit
  --inspect  audit mode — never exit nonzero
"""
import argparse
import os
import re
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
UNIVERSE_FILE = os.path.join(REPO_ROOT, "docs", "superpowers", "specs", "semantic-universe.md")
EVENT_TABLE_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "codegen", "shared", "event_names.rs"
)
ARRAY_VALIDATE_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "semantic", "validate_array_methods.rs"
)
REFOP_IR_FILE = os.path.join(REPO_ROOT, "crates", "omnilang-core", "src", "ir", "types.rs")
MOTION_ANIM_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "parser", "top_level", "motion.rs"
)
DESIGN_PARSER_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "parser", "design.rs"
)
BNF_SPEC_FILE = os.path.join(
    REPO_ROOT, "docs", "superpowers", "specs",
    "2026-07-31-semantic-level-dsl-syntax-spec.md",
)
FLUTTER_UTILS_FILE = os.path.join(REPO_ROOT, "omnilang", "src", "codegen", "flutter", "utils.rs")

# ── universe section contract ─────────────────────────────────────────────
# Domain titles must contain the expected keyword (in order 1..5).
DOMAIN_TITLES = {
    1: "可动画",
    2: "数组",
    3: "事件",
    4: "RefOp",
    5: "design token",
    6: "值绑定",  # v4 5.3 (domain HIGH-3): bind 域——无编译器白名单常量，无交叉断言
}
# Required sub-section headers per domain.
REQUIRED_SUBSECTIONS = ("语义域定义", "语义集条目", "已实现白名单")
# The semantic-set entry table must have columns with these names.
STATUS_COLUMN = "实现状态"
DIAG_COLUMN = "诊断行为"
STATUSES = {"支持", "部分", "未支持"}

# ── whitelist extraction (compiler ground truth) ─────────────────────────

def read_file(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


def event_canonical_names(text):
    """Canonical event names from the shared event table — every
    `... => Some("name")` match arm."""
    return sorted(set(re.findall(r'=>\s*Some\("([^"]+)"\)', text)))


def flutter_callback_names(text):
    """Canonical names the Flutter callback map recognizes — string keys of
    `flutter_callback_for_canonical` in flutter/utils.rs (v4 5.1: the fact
    source for universe「支持 无（全目标）」rows — 台账不再自证)."""
    m = re.search(r'fn flutter_callback_for_canonical\([^)]*\) -> String \{(.*?)\n\}', text, re.S)
    if not m:
        return None
    # All quoted string keys in the match arms are lowercase canonical names;
    # the callback outputs ("onPressed", …) are capitalized and not matched.
    return sorted(set(re.findall(r'"([a-z]+)"', m.group(1))))


def array_method_words(text):
    """Words of the `const ARRAY_METHODS: [&str; N] = ["a", "b", ...];`."""
    m = re.search(r'ARRAY_METHODS[^=]*=\s*\[(.*?)\]', text, re.S)
    return sorted(set(re.findall(r'"([^"]+)"', m.group(1)))) if m else None


def rust_str_array(text, const_name):
    """Words of a `const NAME: &[&str] = &["a", "b", ...];` array literal."""
    m = re.search(rf'{re.escape(const_name)}[^=]*=\s*&\[(.*?)\]', text, re.S)
    return sorted(set(re.findall(r'"([^"]+)"', m.group(1)))) if m else None


def refop_variants(text):
    """RefOpIR enum variant names (RefOp 10 ops)."""
    m = re.search(r'pub enum RefOpIR \{(.*?)\n\}', text, re.S)
    if not m:
        return None
    return sorted({v for v in re.findall(r'^    ([A-Z][A-Za-z]+)\b', m.group(1), re.M)})


def anim_props_from_parser(text):
    """SEMANTIC_ANIM_PROPS（parser/top_level/motion.rs:14）——motion 白名单
    的编译器事实源（v4 D-V4-9：原从 Superseded BNF production 提取，三文档
    可自洽同步漂移——systems MEDIUM-3）。"""
    return rust_str_array(text, "SEMANTIC_ANIM_PROPS")


def token_types_from_parser(text):
    """parse_token_type 的 match 臂引号名（parser/design.rs:251）——token
    类型白名单的编译器事实源（v4 同重指向；13 类型，parser 接受的 lowercase
    名——`"color" => Ok((TokenType::Color, ...))` 形状）。"""
    return sorted(set(re.findall(r'"([A-Za-z]+)"\s*=>\s*Ok\(\(TokenType::', text)))


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


# ── universe parsing ─────────────────────────────────────────────────────

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


def column_index(headers, name):
    for i, h in enumerate(headers):
        if h == name:
            return i
    return None


def first_column_rows(text):
    headers, rows = table_rows(text)
    if headers is None or rows is None:
        return None
    return [r[0] for r in rows]


def event_rows(text):
    """(canonical, status, diag) tuples of the 域 3 语义集条目 table (or None)."""
    sections = split_domains(text)
    if 3 not in sections:
        return None
    sub = subsection(sections[3][1], "语义集条目")
    if sub is None:
        return None
    headers, rows = table_rows(sub)
    if headers is None or rows is None:
        return None
    si = column_index(headers, STATUS_COLUMN)
    di = column_index(headers, DIAG_COLUMN)
    if si is None or di is None:
        return None
    return [(r[0], r[si], r[di]) for r in rows]


def check_event_claims(rows, mapped):
    """域 3「支持 无（全目标）」行必须出现在 Flutter 回调映射中——
    hover/toggle/longpress 曾宣称全目标支持但 web 两目标静默不触发
    （v4 review domain HIGH-1）；「同上」行继承上一行诊断（cut/paste 与
    copy 同一声明）。"""
    errors = []
    mapped = set(mapped or [])
    prev_diag = ""
    for canonical, status, diag in rows:
        if diag == "同上":
            diag = prev_diag
        if status == "支持" and "全目标" in diag and canonical not in mapped:
            errors.append(
                f"域3 {canonical}: 宣称「支持 无（全目标）」但 Flutter 回调映射缺失——"
                f"须改标「部分」或补映射（flutter/utils.rs callback map）"
            )
        prev_diag = diag
    return errors


# ── audit ────────────────────────────────────────────────────────────────

def audit():
    problems = []

    def fail(msg):
        problems.append(msg)

    if not os.path.exists(UNIVERSE_FILE):
        fail(f"semantic universe missing: {os.path.relpath(UNIVERSE_FILE, REPO_ROOT)}")
        return problems

    universe = read_file(UNIVERSE_FILE)
    sections = split_domains(universe)
    for n in range(1, 7):  # v4: 含域 6（值绑定，5.3 新建）——此前 range(1,6) 对域 6 空转
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
            if header == "语义域定义":
                if not sub.strip():
                    fail(f"domain {n} ({title}): '### {header}' is empty")
                continue
            if header == "语义集条目":
                headers, rows = table_rows(sub)
                if headers is None or rows is None or not rows:
                    fail(f"domain {n} ({title}): '### {header}' must be a well-formed table (header + separator + ≥1 row, no empty first column)")
                    continue
                si = column_index(headers, STATUS_COLUMN)
                di = column_index(headers, DIAG_COLUMN)
                if si is None or di is None:
                    fail(f"domain {n} ({title}): '### {header}' must have '{STATUS_COLUMN}' and '{DIAG_COLUMN}' columns (got {headers})")
                    continue
                entries = [r[0] for r in rows]
                if len(entries) != len(set(entries)):
                    fail(f"domain {n} ({title}): duplicate 语义集条目 {sorted({w for w in entries if entries.count(w) > 1})}")
                for row in rows:
                    if row[si] not in STATUSES:
                        fail(f"domain {n} ({title}): entry '{row[0]}' status '{row[si]}' not in {sorted(STATUSES)}")
                    if not row[di].strip():
                        fail(f"domain {n} ({title}): entry '{row[0]}' missing 诊断行为")
            elif header == "已实现白名单":
                words = first_column_rows(sub)
                if words is None or not words:
                    fail(f"domain {n} ({title}): '### {header}' must be a well-formed table (header + separator + ≥1 row, no empty first column)")
                    continue
                if len(words) != len(set(words)):
                    fail(f"domain {n} ({title}): duplicate whitelist entries {sorted({w for w in words if words.count(w) > 1})}")

    # ── per-domain compiler ground truth ────────────────────────────────
    def whitelist(n):
        """First-column words of the 已实现白名单 table of domain n (or None)."""
        if n not in sections:
            return None
        return first_column_rows(subsection(sections[n][1], "已实现白名单"))

    def entries(n):
        """(entry, status) pairs of the 语义集条目 table of domain n (or None)."""
        if n not in sections:
            return None
        sub = subsection(sections[n][1], "语义集条目")
        if sub is None:
            return None
        headers, rows = table_rows(sub)
        if headers is None or rows is None:
            return None
        si = column_index(headers, STATUS_COLUMN)
        if si is None:
            return None
        return [(r[0], r[si]) for r in rows]

    def assert_set(domain_label, got, expected, what, casefold=False):
        if got is None or expected is None:
            fail(f"domain {domain_label}: cannot cross-assert {what} (source extraction failed)")
            return
        norm = (lambda w: w.casefold()) if casefold else (lambda w: w)
        got_s, exp_s = sorted({norm(w) for w in got}), sorted({norm(w) for w in expected})
        if got_s != exp_s:
            fail(
                f"domain {domain_label}: {what} ≠ compiler whitelist\n"
                f"    doc-only:      {sorted(set(got_s) - set(exp_s))}\n"
                f"    compiler-only: {sorted(set(exp_s) - set(got_s))}"
            )

    def assert_matrix(domain_label, entries_list, whitelist_words, casefold=False):
        """Every universe entry has a status (checked above); the status
        matrix must be consistent with the whitelist:
          - whitelist ⊆ 语义集条目 (no implemented member missing from the universe)
          - {支持, 部分} ⊆ whitelist (a supported entry must be implemented)
          - 未支持 ∩ whitelist = ∅ (an implemented entry cannot be 未支持)
        """
        if entries_list is None or whitelist_words is None:
            fail(f"domain {domain_label}: cannot cross-assert status matrix (extraction failed)")
            return
        norm = (lambda w: w.casefold()) if casefold else (lambda w: w)
        entry_set = {norm(e) for e, _ in entries_list}
        wl_set = {norm(w) for w in whitelist_words}
        if not wl_set <= entry_set:
            fail(f"domain {domain_label}: whitelist not a subset of 语义集条目 (orphan whitelist members — every implemented entry must be a universe entry): {sorted(wl_set - entry_set)}")
        supported = {norm(e) for e, s in entries_list if s in ("支持", "部分")}
        if not supported <= wl_set:
            fail(f"domain {domain_label}: entries marked {STATUS_COLUMN} 支持/部分 but absent from 已实现白名单: {sorted(supported - wl_set)}")
        unsupported = {norm(e) for e, s in entries_list if s == "未支持"}
        overlap = unsupported & wl_set
        if overlap:
            fail(f"domain {domain_label}: entries marked 未支持 but present in 已实现白名单 (contradiction): {sorted(overlap)}")

    bnf = read_file(BNF_SPEC_FILE)

    def bnf_doc_subset(label, lhs, code_words):
        """「文档 ⊆ 代码」单向检查（v4 D-V4-9）——Superseded BNF production
        仍被机器引用但不再作事实源：其词汇必须是 parser 白名单的子集（文档
        不得宣称代码没有的词汇）；反向（代码 ⊆ 文档）由 whitelist 断言承担。"""
        bnf_words = bnf_production_words(bnf, lhs)
        if bnf_words is None:
            fail(f"domain {label}: cannot check BNF production '{lhs}' (extraction failed)")
            return
        extra = sorted(set(bnf_words) - set(code_words))
        if extra:
            fail(f"domain {label}: BNF '{lhs}' claims words absent from the parser whitelist (文档 ⊆ 代码 violated): {extra}")

    # motion (域 1): whitelist == parser SEMANTIC_ANIM_PROPS (19);
    # v4: BNF semantic_anim_prop production 降级为「文档 ⊆ 代码」单向检查
    wl1, en1 = whitelist(1), entries(1)
    motion_props = anim_props_from_parser(read_file(MOTION_ANIM_FILE))
    assert_set("motion", wl1, motion_props, "已实现白名单")
    assert_matrix("motion", en1, wl1)
    bnf_doc_subset("motion", "semantic_anim_prop", motion_props)

    # array (域 2): whitelist == ARRAY_METHODS (filter/map/count)
    wl2, en2 = whitelist(2), entries(2)
    assert_set("array", wl2, array_method_words(read_file(ARRAY_VALIDATE_FILE)), "已实现白名单")
    assert_matrix("array", en2, wl2)

    # events (域 3): whitelist == canonical event names (54)
    wl3, en3 = whitelist(3), entries(3)
    assert_set("事件", wl3, event_canonical_names(read_file(EVENT_TABLE_FILE)), "已实现白名单")
    assert_matrix("事件", en3, wl3)
    # events (域 3): 「支持 无（全目标）」行 ⊆ Flutter 回调映射（v4 5.1 事实断言——
    # hover/toggle/longpress 曾宣称全目标支持但 web 两目标静默不触发）
    rows3 = event_rows(universe)
    if rows3 is None:
        fail("事件: cannot cross-assert Flutter mapping (domain 3 语义集条目 extraction failed)")
    else:
        mapped = flutter_callback_names(read_file(FLUTTER_UTILS_FILE))
        if mapped is None:
            fail("事件: cannot cross-assert Flutter mapping (flutter/utils.rs callback map extraction failed)")
        else:
            problems.extend(check_event_claims(rows3, mapped))

    # RefOp (域 4): whitelist == RefOpIR variants (10, casefolded)
    wl4, en4 = whitelist(4), entries(4)
    assert_set("RefOp", wl4, refop_variants(read_file(REFOP_IR_FILE)), "已实现白名单", casefold=True)
    assert_matrix("RefOp", en4, wl4, casefold=True)

    # design token (域 5): whitelist == parser token type match arms (13);
    # v4: BNF token_type production 降级为「文档 ⊆ 代码」单向检查
    wl5, en5 = whitelist(5), entries(5)
    token_types = token_types_from_parser(read_file(DESIGN_PARSER_FILE))
    assert_set("design token", wl5, token_types, "已实现白名单")
    assert_matrix("design token", en5, wl5)
    bnf_doc_subset("design token", "token_type", token_types)

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
    print(f"\n=== semantic-universe gate: {problems and len(problems) or 0} problem(s) ===")
    for p in problems:
        print(f"FAIL {p}")
    if problems and not args.inspect:
        print("\nGATE FAILED: semantic set drifted from the implementation status / whitelists.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
