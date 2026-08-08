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
  motion      ↔ SEMANTIC_ANIM_PROPS in
                crates/omnilang-core/src/parser/top_level/motion.rs
                (19 属性白名单 — v4 D-V4-9: parser 常量是事实源；原 BNF
                semantic_anim_prop production 降级为「文档 ⊆ 代码」单向检查)
  RefOp       ↔ RefOpIR variants in crates/omnilang-core/src/ir/types.rs
                (RefOp 10 ops)
  token type  ↔ parse_token_type match arms in
                crates/omnilang-core/src/parser/design.rs
                (13 类型 — parser-accepted lowercase names; v4 同重指向)

A word added to any compiler whitelist forces the registry to learn it and
vice versa — the registry can never drift from the whitelists, and a platform
flag name can never slip in as semantic vocabulary without failing the
borrow-word check (排除词汇 must be disjoint from 语义词汇 and, for a11y,
must equal the legacy-flag list exactly).

Usage:
  python scripts/verify-vocabulary.py [--check|--inspect]
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
# a11y-only: the Flutter-mappable role subset and the live value domain,
# each cross-asserted against its validator constant.
A11Y_EXTRA_SUBSECTIONS = {
    "role 子集": "FLUTTER_ROLE_SUBSET",
    "live 值域": "LIVE_VALUES",
}

# ── whitelist extraction (compiler ground truth) ─────────────────────────

def read_file(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


def event_canonical_names(text):
    """Canonical event names from the shared event table — the
    `... => Some("name")` match arms of `canonical_event_name`'s body
    (4.2 事件白名单).

    FN-SCOPED（防回归教训，2026-08-08 Task 1）: whole-file 正则会同时命中
    `legacy_spelling` 的 match 臂（"onPress" => Some("onClick") 等 3 个）——
    旧名是硬错误迁移提示，不是白名单成员，误捕会让门禁报出幽灵 whitelist-only
    diff。必须先定位 `pub fn canonical_event_name` 函数体再提取（先例：
    verify-semantic-universe.py:120-129）。提取失败返回 None——门禁显式失败，
    不静默放行。"""
    m = re.search(r'pub fn canonical_event_name.*?\{(.*?)\n\}', text, re.S)
    if not m:
        return None
    return sorted(set(re.findall(r'=>\s*Some\("([^"]+)"\)', m.group(1))))


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
    for cells in rows:
        if not cells[0]:
            return None
    return [cells[0] for cells in rows]


# ── audit ────────────────────────────────────────────────────────────────

def audit():
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
                fail(f"domain {n} ({title}): '### {header}' must be a well-formed table (header + separator + ≥1 row, no empty first column)")
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

    def semantic_names(n):
        """First-column words of the 语义事件名（DSL 表面） table of domain n
        (or None when the domain is missing or the table is malformed)."""
        if n not in sections:
            return None
        return first_column_rows(subsection(sections[n][1], "语义事件名（DSL 表面）") or "")

    # events (4.2): registry vocabulary == shared event table canonical names
    ev = vocab(1)
    if ev is not None:
        assert_set("事件", ev, event_canonical_names(read_file(EVENT_TABLE_FILE)), "语义词汇")
    # 语义事件名（DSL 表面）== SEMANTIC_EVENT_NAMES（14 名，spec 2026-08-08 §3.1
    # 首批语义事件）双向断言——新增语义事件必须同 commit 在登记册入册。
    # 发现 A（EVENT_NAME_SUGGESTIONS 含 4 个自身硬错误名 onClick/onTap/onMouseEnter/
    # onMouseLeave——建议可能把用户引到硬错误拼写）defer 阶段 4——此处不加
    # suggestions 防御断言（当前会红：建议表含自身硬错误名）。
    ev_sem = semantic_names(1)
    if ev_sem is not None:
        assert_set("事件", ev_sem, rust_str_array(read_file(EVENT_TABLE_FILE), "SEMANTIC_EVENT_NAMES"), "语义事件名")
    elif 1 in sections:
        fail("domain 事件: '### 语义事件名（DSL 表面）' missing or malformed (header + separator + ≥1 row, no empty first column)")
    ex_ev = excluded(1)
    if ev is not None and ex_ev is not None:
        overlap = sorted(set(ev) & set(ex_ev))
        if overlap:
            fail(f"domain 事件: 排除词汇 overlaps 语义词汇 (借词违规): {overlap}")

    def bnf_doc_subset(label, lhs, code_words):
        """「文档 ⊆ 代码」单向检查（v4 D-V4-9）——Superseded BNF production
        仍被机器引用但不再作事实源：其词汇必须是 parser 白名单的子集（文档
        不得宣称代码没有的词汇）；反向（代码 ⊆ 文档）由 registry 断言承担。"""
        bnf_words = bnf_production_words(bnf, lhs)
        if bnf_words is None:
            fail(f"domain {label}: cannot check BNF production '{lhs}' (extraction failed)")
            return
        extra = sorted(set(bnf_words) - set(code_words))
        if extra:
            fail(f"domain {label}: BNF '{lhs}' claims words absent from the parser whitelist (文档 ⊆ 代码 violated): {extra}")

    bnf = read_file(BNF_SPEC_FILE)

    # a11y (4.3): primitives == a11y_entry BNF; role 子集 == FLUTTER_ROLE_SUBSET;
    # live 值域 == LIVE_VALUES; 排除词汇 == legacy flag names
    if 2 in sections:
        title, body = sections[2]
        a11y_src = read_file(A11Y_VALIDATE_FILE)
        assert_set("a11y", vocab(2), bnf_production_words(bnf, "a11y_entry"), "语义词汇")
        for header, const_name in A11Y_EXTRA_SUBSECTIONS.items():
            sub = first_column_rows(subsection(body, header))
            if sub is None:
                fail(f"domain 2 ({title}): missing '### {header}' table")
            else:
                assert_set("a11y", sub, rust_str_array(a11y_src, const_name), header)
        legacy = legacy_flag_roles(a11y_src)
        ex_a11y = excluded(2)
        if ex_a11y is not None:
            if legacy is None:
                fail("domain a11y: cannot cross-assert 排除词汇 (LEGACY_FLAG_ROLES extraction failed)")
            else:
                assert_set("a11y", ex_a11y, [name for name, _ in legacy], "排除词汇")

    # motion: registry vocabulary == parser SEMANTIC_ANIM_PROPS (19 属性白名单);
    # v4: BNF semantic_anim_prop production 降级为「文档 ⊆ 代码」单向检查
    motion_props = anim_props_from_parser(read_file(MOTION_ANIM_FILE))
    assert_set("motion", vocab(3), motion_props, "语义词汇")
    bnf_doc_subset("motion", "semantic_anim_prop", motion_props)

    # RefOp: registry vocabulary (casefolded) == RefOpIR variants (10 ops)
    refop = vocab(4)
    if refop is not None:
        variants = refop_variants(read_file(REFOP_IR_FILE))
        if variants is None:
            fail("domain RefOp: cannot cross-assert 语义词汇 (RefOpIR extraction failed)")
        else:
            assert_set("RefOp", [w.casefold() for w in refop], [v.casefold() for v in variants], "语义词汇")

    # design token: registry vocabulary == parser token type match arms (13 类型);
    # v4: BNF token_type production 降级为「文档 ⊆ 代码」单向检查
    token_types = token_types_from_parser(read_file(DESIGN_PARSER_FILE))
    assert_set("design token", vocab(5), token_types, "语义词汇")
    bnf_doc_subset("design token", "token_type", token_types)

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
    args = ap.parse_args()

    problems = audit()
    print(f"\n=== vocabulary-registry gate: {problems and len(problems) or 0} problem(s) ===")
    for p in problems:
        print(f"FAIL {p}")
    if problems and not args.inspect:
        print("\nGATE FAILED: registry/vocabulary drifted from the compiler whitelists.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
