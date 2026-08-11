#!/usr/bin/env python3
"""Vocabulary-registry gate — design P1 recurrence mechanism (v3 plan Task 7.1).

docs/superpowers/specs/vocabulary-registry.md is the closed-vocabulary
registry of the semantic DSL: six closed vocabulary domains (events / a11y
role / motion properties / RefOps / design-token types / form validation
rules). Every entry must answer 「词汇的语义域是什么」with four required fields:

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
  form rule   ↔ VALIDATE_RULES in
                crates/omnilang-core/src/semantic/validate_form_load.rs
                (6 规则名 — validate 值域闭集; Task 7 新增域 6)

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
VALIDATE_FORM_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "semantic", "validate_form_load.rs"
)
# 2026-08-11 Task 9（批次 2 导航路由）：域 7 导航词汇锚点文件——解析器
# （parser/top_level/route.rs，@route/@layout/@query 语法 + V2/V5 校验）+ 内建
# 函数登记（parser/expr/mod.rs BUILTIN_FUNCTIONS 内含 navigate/back）。
ROUTE_PARSER_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "parser", "top_level", "route.rs"
)
EXPR_MOD_FILE = os.path.join(REPO_ROOT, "crates", "omnilang-core", "src", "parser", "expr", "mod.rs")
# 2026-08-11 批次 3 Task 1（登记先行）：域 8 响应式词汇锚点文件——语义层
# （analyse.rs validate_breakpoints 查表）+ design IR（ir/design.rs breakpoints
# 常量表）+ 三目标 codegen（React useMediaQuery / Flutter MediaQuery.sizeOf /
# OmniView matchMedia）。
ANALYSE_FILE = os.path.join(REPO_ROOT, "crates", "omnilang-core", "src", "analyse.rs")
DESIGN_IR_FILE = os.path.join(REPO_ROOT, "crates", "omnilang-core", "src", "ir", "design.rs")
REACT_COMPONENT_FILE = os.path.join(
    REPO_ROOT, "omnilang", "src", "codegen", "react", "component", "mod.rs"
)
FLUTTER_MOD_FILE = os.path.join(REPO_ROOT, "omnilang", "src", "codegen", "flutter", "mod.rs")
FLUTTER_REF_FILE = os.path.join(REPO_ROOT, "omnilang", "src", "codegen", "flutter", "ref.rs")
OMNIVIEW_EXPR_FILE = os.path.join(REPO_ROOT, "omnilang", "src", "codegen", "omniview", "expr.rs")

# ── registry section contract ────────────────────────────────────────────
# Domain titles must contain the expected keyword (in order 1..6).
DOMAIN_TITLES = {
    1: "事件",
    2: "a11y",
    3: "motion",
    4: "RefOp",
    5: "design token",
    6: "表单校验规则",  # 2026-08-09 Task 7: @form validate 规则域——VALIDATE_RULES 锚点
    7: "导航",  # 2026-08-10 批次 2 Task 1: 导航词汇域——@route/@layout/@query/navigate/back（登记先行 ADR-074）
    8: "响应式",  # 2026-08-11 批次 3 Task 1: 响应式词汇域——bp/breakpoint（登记先行 ADR-074；域 7 已被导航占用，响应式取域 8，两文档编号体系独立）
    10: "反馈模态",  # 2026-08-09 Task 0: 反馈模态词汇域——@modal/showToast/@skeleton/@errorBoundary（批次 1，登记先行 ADR-074）
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
    parts = re.split(r'^## 域 (\d+): ([^\n]+)', text, flags=re.M)
    sections = {}
    for i in range(1, len(parts), 3):
        sections[int(parts[i])] = (parts[i + 1], parts[i + 2])
    return sections


def subsection(body, header):
    m = re.search(rf'^### {re.escape(header)}\s*\n(.*?)(?=^### |\Z)', body, re.M | re.S)
    return m.group(1) if m else None


def table_cell_rows(text):
    """Cell lists of the first markdown table in `text` (header + separator
    rows excluded). Malformed tables return None."""
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
    return rows


def first_column_rows(text):
    """First-column cell values of the first markdown table in `text`."""
    rows = table_cell_rows(text)
    return None if rows is None else [cells[0] for cells in rows]


def excluded_word_pairs(text):
    """(word, owner) pairs of the 排除词汇 table — first-column excluded words
    tied to their owning vocabulary via the 正确语义词汇 column. Needed for
    域 10: 排除词汇 first-column entries are individual borrow words (dialog/
    toast/…), not vocabulary names — per-vocab borrow-word assertions map
    through the owner column."""
    rows = table_cell_rows(text)
    if rows is None or not rows or len(rows[0]) < 2:
        return None
    return [(cells[0], cells[-1]) for cells in rows]


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
    for n in sorted(DOMAIN_TITLES):  # 2026-08-09 Task 7: 含域 6（表单校验规则）；Task 0: 含域 10（反馈模态）；批次 2/3: 含域 7（导航）/域 8（响应式）——sorted(DOMAIN_TITLES) 自维护
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

    # form rule (域 6, 2026-08-09 Task 7): registry vocabulary == VALIDATE_RULES
    # （validate_form_load.rs:10，6 规则名闭集——@form validate 值域）
    assert_set("表单校验规则", vocab(6), rust_str_array(read_file(VALIDATE_FORM_FILE), "VALIDATE_RULES"), "语义词汇")

    # borrow-word rule across all domains: 排除词汇 must be disjoint from 语义词汇
    for n, label in ((1, "事件"), (2, "a11y"), (3, "motion"), (4, "RefOp"), (5, "design token"), (6, "表单校验规则")):
        v, x = vocab(n), excluded(n)
        if v is not None and x is not None:
            overlap = sorted(set(v) & set(x))
            if overlap:
                fail(f"domain {n} ({label}): 排除词汇 overlaps 语义词汇 (借词违规): {overlap}")

    # 反馈模态 (域 10, 2026-08-09 Task 0): 机制级条目（批次 1 登记先行 ADR-074——
    # 语法 Task 1-6 落地前无编译器白名单常量，交叉断言以存在性检查承担，照
    # semantic-universe 域 10 锚点方案）。语义词汇为精确四元闭集；排除词汇
    # 按 正确语义词汇 归属列映射到所属词汇（排除词汇表首列是逐借词而非词汇名）。
    v10 = vocab(10)
    if v10 is not None:
        expected_vocab = ["@modal", "showToast", "@skeleton", "@errorBoundary"]
        if sorted(v10) != sorted(expected_vocab):
            fail(
                f"domain 10 (反馈模态): 语义词汇 ≠ 批次 1 登记词汇\n"
                f"    registry-only:   {sorted(set(v10) - set(expected_vocab))}\n"
                f"    spec-only:       {sorted(set(expected_vocab) - set(v10))}"
            )
    if 10 in sections:
        pairs = excluded_word_pairs(subsection(sections[10][1], "排除词汇"))
        if pairs is None:
            fail("domain 10 (反馈模态): '### 排除词汇' must be a well-formed table (header + separator + ≥1 row, no empty first column)")
        else:
            expected = {
                "@modal": ["dialog", "showModal", "showDialog", "Modal", "ModalDialog"],
                "showToast": ["toast", "SnackBar", "Toast", "Sonner", "ElMessage"],
                "@skeleton": ["skeleton", "shimmer", "Shimmer"],
                "@errorBoundary": ["ErrorBoundary", "error-boundary"],
            }
            owner_map = {}
            for word, owner in pairs:
                owner_map.setdefault(owner, []).append(word)
            # owner 列拼写守卫（2026-08-09 硬化）：owner 键须与登记词汇精确一致——
            # 否则拼错的 owner（如 @modalX）会让下方逐词断言静默跳过该行排除词。
            if set(owner_map) != set(expected):
                fail(
                    f"domain 10 (反馈模态): 排除词汇 owner 列 ≠ 登记词汇（spec §5.2）\n"
                    f"    registry-only:   {sorted(set(owner_map) - set(expected))}\n"
                    f"    spec-only:       {sorted(set(expected) - set(owner_map))}"
                )
            else:
                for vocab_word, words in expected.items():
                    got = sorted(owner_map.get(vocab_word, []))
                    if got != sorted(words):
                        fail(
                            f"domain 10 (反馈模态): {vocab_word} 的排除词汇 ≠ 批次 1 登记（spec §5.2）\n"
                            f"    registry-only:   {sorted(set(got) - set(words))}\n"
                            f"    spec-only:       {sorted(set(words) - set(got))}"
                        )
            # 借词违规由上方通用 loop 承担——但该 loop 只覆盖域 1-6，域 10
            # 语义词汇（@modal 等）与排除词汇（dialog 等）天然不相交，显式断言以防登记回退。
            if v10 is not None:
                overlap = sorted(set(v10) & set(p[0] for p in pairs))
                if overlap:
                    fail(f"domain 10 (反馈模态): 排除词汇 overlaps 语义词汇 (借词违规): {overlap}")

    # 导航 (域 7, 2026-08-10 批次 2 Task 1): 机制级词汇（组件级指令 @route/@layout/
    # @query + 内建语义函数 navigate/back——语法未落地无编译器白名单常量，交叉断言
    # 以存在性检查承担，照域 10 先例）。语义词汇为精确五元闭集（spec §5.2）；排除
    # 词汇按 正确语义词汇 归属列映射到所属词汇（排除词汇表首列是逐借词而非词汇名）。
    v7 = vocab(7)
    if v7 is not None:
        expected_vocab = ["@route", "@layout", "@query", "navigate", "back"]
        if sorted(v7) != sorted(expected_vocab):
            fail(
                f"domain 7 (导航): 语义词汇 ≠ 批次 2 登记词汇（spec §5.2）\n"
                f"    registry-only:   {sorted(set(v7) - set(expected_vocab))}\n"
                f"    spec-only:       {sorted(set(expected_vocab) - set(v7))}"
            )
    if 7 in sections:
        pairs = excluded_word_pairs(subsection(sections[7][1], "排除词汇"))
        if pairs is None:
            fail("domain 7 (导航): '### 排除词汇' must be a well-formed table (header + separator + ≥1 row, no empty first column)")
        else:
            expected = {
                "@route": ["path", "route", "Routes", "GoRoute"],
                "@layout": ["Outlet", "ShellRoute", "Layout"],
                "@query": ["query", "searchParams"],
                "navigate": ["push", "go", "useNavigate", "Navigate"],
                "back": ["pop", "back()"],
            }
            owner_map = {}
            for word, owner in pairs:
                owner_map.setdefault(owner, []).append(word)
            # owner 列拼写守卫（照域 10 硬化先例）：owner 键须与登记词汇精确一致——
            # 否则拼错的 owner（如 @routeX）会让下方逐词断言静默跳过该行排除词。
            if set(owner_map) != set(expected):
                fail(
                    f"domain 7 (导航): 排除词汇 owner 列 ≠ 登记词汇（spec §5.2）\n"
                    f"    registry-only:   {sorted(set(owner_map) - set(expected))}\n"
                    f"    spec-only:       {sorted(set(expected) - set(owner_map))}"
                )
            else:
                for vocab_word, words in expected.items():
                    got = sorted(owner_map.get(vocab_word, []))
                    if got != sorted(words):
                        fail(
                            f"domain 7 (导航): {vocab_word} 的排除词汇 ≠ 批次 2 登记（spec §5.2）\n"
                            f"    registry-only:   {sorted(set(got) - set(words))}\n"
                            f"    spec-only:       {sorted(set(words) - set(got))}"
                        )
            # 借词违规：域 7 语义词汇与排除词汇天然不相交（back() 字面 ≠ back），
            # 显式断言以防登记回退。
            if v7 is not None:
                overlap = sorted(set(v7) & set(p[0] for p in pairs))
                if overlap:
                    fail(f"domain 7 (导航): 排除词汇 overlaps 语义词汇 (借词违规): {overlap}")
            # 词汇锚点断言（Task 9, 照 bnf_doc_subset 先例 :324——「文档 ⊆ 代码」
            # 存在性检查）：域 7 是机制级词汇（无词法级白名单常量），@ 前缀词
            # 无法用引号词正则提取，改为对编译器事实源做关键符号 grep。锚点文件 =
            # parser/top_level/route.rs（@route/@layout/@query 语法 + V2/V5 校验）
            # + parser/expr/mod.rs（BUILTIN_FUNCTIONS 登记 navigate/back）。
            route_parser_src = read_file(ROUTE_PARSER_FILE)
            expr_src = read_file(EXPR_MOD_FILE)
            for vocab_word, haystack, needles in (
                ("@route", route_parser_src, ["parse_route_decl"]),
                ("@layout", route_parser_src, ["parse_layout_decl"]),
                ("@query", route_parser_src, ["parse_query_decl"]),
                ("navigate", expr_src, ['"navigate"']),
                ("back", expr_src, ['"back"']),
            ):
                for needle in needles:
                    if needle not in haystack:
                        fail(f"domain 7 (导航): 词汇 '{vocab_word}' 编译器锚点 '{needle}' 缺失")

    # 导航排除词汇全局禁借断言（Task 9 测试点）：域 7 的 15 个目标端词
    # （path/route/Routes/GoRoute/Outlet/ShellRoute/Layout/query/searchParams/push/
    # go/useNavigate/Navigate/pop/back()）不得成为任何域的语义词汇——域 1-6 由上方
    # 通用 loop 承担（域内自查）、域 7/10 有局部断言，此处做跨域强化（遍历全部登记
    # 域）。排除词汇集合自维护：直接从域 7 排除词汇表（pairs）提取而非硬编码，
    # 但下方仍显式断言 15 词闭集以防登记回退。
    if 7 in sections and pairs is not None:
        nav_excluded = {word for word, owner in pairs}
        expected_excluded = {"path", "route", "Routes", "GoRoute", "Outlet", "ShellRoute",
                             "Layout", "query", "searchParams", "push", "go",
                             "useNavigate", "Navigate", "pop", "back()"}
        if nav_excluded != expected_excluded:
            fail(
                f"domain 7 (导航): 排除词汇全集 ≠ Task 9 登记 15 词（spec §5.2）\n"
                f"    registry-only:   {sorted(nav_excluded - expected_excluded)}\n"
                f"    spec-only:       {sorted(expected_excluded - nav_excluded)}"
            )
        all_vocab = set()
        for n in sorted(sections):
            v = vocab(n)
            if v is not None:
                all_vocab |= set(v)
        overlap = sorted(all_vocab & nav_excluded)
        if overlap:
            fail(f"全部域: 导航排除词汇成为语义词汇 (借词违规): {overlap}")

    # 响应式 (域 8, 2026-08-11 批次 3 Task 1 登记先行): 机制级词汇（bp 内建断点
    # 条件表达式 + breakpoint 断点声明关键字——语法未落地无编译器白名单常量，交叉
    # 断言以存在性检查承担，照域 10/7 先例）。语义词汇为精确二元闭集（spec §5.3）；
    # 排除词汇含 useMediaQuery（React 借词禁——语义词汇不得为合成事件名/旗标名/
    # 库 API 名）。四字段（来源域/语义词汇/目标映射/排除词汇）由通用 REQUIRED_
    # SUBSECTIONS 循环承担。词汇锚点 = analyse.rs（validate_breakpoints）+ 三目标
    # codegen（useMediaQuery/MediaQuery.sizeOf/matchMedia）——T5-T8 落地后转绿，
    # T1 阶段为 pending-red（文档化预期，同 semantic-universe 域 12）。
    v8 = vocab(8)
    if v8 is not None:
        expected_vocab = ["bp", "breakpoint"]
        if sorted(v8) != sorted(expected_vocab):
            fail(
                f"domain 8 (响应式): 语义词汇 ≠ 批次 3 登记词汇（spec §5.3）\n"
                f"    registry-only:   {sorted(set(v8) - set(expected_vocab))}\n"
                f"    spec-only:       {sorted(set(expected_vocab) - set(v8))}"
            )
    if 8 in sections:
        ex8 = excluded(8)
        if ex8 is None:
            fail("domain 8 (响应式): '### 排除词汇' must be a well-formed table (header + separator + ≥1 row, no empty first column)")
        else:
            if "useMediaQuery" not in ex8:
                fail("domain 8 (响应式): 排除词汇应包含 useMediaQuery（React 借词禁）")
            if v8 is not None:
                overlap = sorted(set(v8) & set(ex8))
                if overlap:
                    fail(f"domain 8 (响应式): 排除词汇 overlaps 语义词汇 (借词违规): {overlap}")
        # 词汇锚点断言（照域 7 先例——机制级词汇无词法级白名单常量，对编译器
        # 事实源做关键符号 grep）。bp 为表达式函数形态（语义层 validate_breakpoints
        # 查表 + 三目标 codegen 分派）；breakpoint 为声明关键字（parser design 块 +
        # DesignIR.breakpoints 常量表）。
        analyse_src = read_file(ANALYSE_FILE)
        design_parser_src = read_file(DESIGN_PARSER_FILE)
        design_ir_src = read_file(DESIGN_IR_FILE)
        react_src = read_file(REACT_COMPONENT_FILE)
        flutter_ref_src = read_file(FLUTTER_REF_FILE)
        omni_expr_src = read_file(OMNIVIEW_EXPR_FILE)
        for vocab_word, haystack, needles in (
            ("bp", analyse_src, ["validate_breakpoints"]),
            ("bp", react_src, ["useMediaQuery"]),
            ("bp", flutter_ref_src, ["MediaQuery.sizeOf"]),
            ("bp", omni_expr_src, ["matchMedia"]),
            ("breakpoint", design_parser_src, ["breakpoint"]),
            ("breakpoint", design_ir_src, ["breakpoints"]),
        ):
            for needle in needles:
                if needle not in haystack:
                    fail(f"domain 8 (响应式): 词汇 '{vocab_word}' 编译器锚点 '{needle}' 缺失")
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
