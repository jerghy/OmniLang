#!/usr/bin/env python3
"""Semantic-universe gate — design P3 mechanism (v3 plan Task 7.2).

docs/superpowers/specs/semantic-universe.md is the semantic full set: the
FRONTEND FEATURE UNIVERSE (independent of implementation progress), split
from the implemented set. Nine semantic domains (animatable properties /
array operations / events / RefOps / design-token types / value bindings /
directives / form validation / async data three-states). Every entry must
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
  directives ↔ drag/dragEnd dispatch + SEMANTIC_BINDINGS +
            peek_is_boolean_attr_keyword in
            crates/omnilang-core/src/parser/view/element.rs
            (6 词法级条目; 机制级条目 style 动态绑定/布尔属性 走存在性断言)
  form     ↔ VALIDATE_RULES in
            crates/omnilang-core/src/semantic/validate_form_load.rs
            (6 规则名白名单); 机制级条目 @form/formErrors/formValid/
            formSubmitting 走存在性断言（ADR-074 先例——跨三目标锚点）
  data     ↔ 无词法级白名单常量——@load/loading/error/data/retry 以
            编译器代码锚点存在性断言（域 6 值绑定先例）; v0 边界（仅
            Array 返回）记录于诊断行为列

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
ELEMENT_PARSER_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "parser", "view", "element.rs"
)
DIRECT_WRITE_FILE = os.path.join(
    REPO_ROOT, "omnilang", "src", "codegen", "shared", "direct_write.rs"
)
BNF_SPEC_FILE = os.path.join(
    REPO_ROOT, "docs", "superpowers", "specs",
    "2026-07-31-semantic-level-dsl-syntax-spec.md",
)
FLUTTER_UTILS_FILE = os.path.join(REPO_ROOT, "omnilang", "src", "codegen", "flutter", "utils.rs")
VALIDATE_FORM_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "semantic", "validate_form_load.rs"
)
LOWER_COMPONENT_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "ir", "lower", "component.rs"
)
REACT_COMPONENT_FILE = os.path.join(
    REPO_ROOT, "omnilang", "src", "codegen", "react", "component", "mod.rs"
)
FLUTTER_MOD_FILE = os.path.join(REPO_ROOT, "omnilang", "src", "codegen", "flutter", "mod.rs")
OMNIVIEW_STORE_FILE = os.path.join(REPO_ROOT, "omnilang", "src", "codegen", "omniview", "store.rs")
# 2026-08-10 Task 7（批次 1 反馈模态全量断言）：域 10 编译器事实源锚点文件——
# 语义验证器 + React/OmniView codegen（Flutter 锚点暂缺，Task 5 待落地）
FEEDBACK_MODAL_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "semantic", "validate_feedback_modal.rs"
)
OMNIVIEW_MODAL_FILE = os.path.join(
    REPO_ROOT, "omnilang", "src", "codegen", "omniview", "view", "modal.rs"
)
OMNIVIEW_ELEMENT_FILE = os.path.join(
    REPO_ROOT, "omnilang", "src", "codegen", "omniview", "view", "element.rs"
)
OMNIVIEW_EXPR_FILE = os.path.join(REPO_ROOT, "omnilang", "src", "codegen", "omniview", "expr.rs")
# 2026-08-11 Task 9（批次 2 导航路由全量断言）：域 11 编译器事实源锚点文件——
# 解析器（parser/top_level/route.rs）+ IR lowering（ir/lower/route.rs）+ 内建
# 函数登记（parser/expr/mod.rs）+ 语义验证器（semantic/validate_route_navigation.rs）
# + 三目标 codegen（react/flutter/omniview router.rs）。锚点账本见 audit() 域 11 块。
ROUTE_PARSER_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "parser", "top_level", "route.rs"
)
LOWER_ROUTE_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "ir", "lower", "route.rs"
)
EXPR_MOD_FILE = os.path.join(REPO_ROOT, "crates", "omnilang-core", "src", "parser", "expr", "mod.rs")
VALIDATE_ROUTE_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "semantic", "validate_route_navigation.rs"
)
REACT_ROUTER_FILE = os.path.join(REPO_ROOT, "omnilang", "src", "codegen", "react", "router.rs")
FLUTTER_ROUTER_FILE = os.path.join(REPO_ROOT, "omnilang", "src", "codegen", "flutter", "router.rs")
OMNIVIEW_ROUTER_FILE = os.path.join(REPO_ROOT, "omnilang", "src", "codegen", "omniview", "router.rs")
# 2026-08-11 批次 3 Task 1（登记先行 ADR-074）：域 12 响应式编译器事实源锚点文件——
# design IR（ir/design.rs breakpoints 常量表）+ style 解析器（parser/view/style.rs
# AtMedia/@media 块）+ 分析管线（analyse.rs validate_breakpoints/validate_style_selectors）
# + 三目标 codegen 锚点（React useMediaQuery helper / Flutter MediaQuery.sizeOf /
# OmniView matchMedia 表达式）。锚点账本见 audit() 域 12 块；T2-T8 落地前为
# pending-red（文档化预期，禁以提前实现语法转绿）。
DESIGN_IR_FILE = os.path.join(REPO_ROOT, "crates", "omnilang-core", "src", "ir", "design.rs")
ANALYSE_FILE = os.path.join(REPO_ROOT, "crates", "omnilang-core", "src", "analyse.rs")
STYLE_PARSER_FILE = os.path.join(
    REPO_ROOT, "crates", "omnilang-core", "src", "parser", "view", "style.rs"
)
# 深色模式生成物（@themes → 3-layer CSS）真实生成点：shared/design.rs 的
# emit_design_css（react/component/mod.rs 仅 prefers-reduced-motion，无主题生成物）
SHARED_DESIGN_FILE = os.path.join(REPO_ROOT, "omnilang", "src", "codegen", "shared", "design.rs")
# MediaQuery.sizeOf 表达式分派在 flutter/ref.rs（mod.rs 是组件/样式输出层）
FLUTTER_REF_FILE = os.path.join(REPO_ROOT, "omnilang", "src", "codegen", "flutter", "ref.rs")

# ── universe section contract ─────────────────────────────────────────────
# Domain titles must contain the expected keyword (in order 1..9).
DOMAIN_TITLES = {
    1: "可动画",
    2: "数组",
    3: "事件",
    4: "RefOp",
    5: "design token",
    6: "值绑定",  # v4 5.3 (domain HIGH-3): bind 域——无编译器白名单常量，无交叉断言
    7: "元素指令",  # 2026-08-07: directives 域——drag/dragEnd/ref/语义绑定/style 动态绑定/布尔属性
    8: "表单",  # 2026-08-09: form 域——@form/validate 规则/内建信号（Task 7，阶段 2 域 A）
    9: "数据三态",  # 2026-08-09: @load 域——loading/error/data/retry（Task 7，阶段 2 域 A）
    10: "反馈模态",  # 2026-08-09 Task 0: 反馈模态域——showToast/@modal/@skeleton/@errorBoundary（批次 1，登记先行 ADR-074）
    11: "导航",  # 2026-08-10 批次 2 Task 1: 导航域——@route/@layout/@query/navigate/back（登记先行 ADR-074）
    12: "响应式",  # 2026-08-11 批次 3 Task 1: 响应式域——breakpoint 声明/bp() 条件/@media 块/@container 块/深色模式（登记先行 ADR-074）
    # 域 14 为单表留生态登记行（无三子节结构）——不走通用循环，见 audit() 域 14 专项块
}
# Required sub-section headers per domain.
REQUIRED_SUBSECTIONS = ("语义域定义", "语义集条目", "已实现白名单")
# The semantic-set entry table must have columns with these names.
STATUS_COLUMN = "实现状态"
DIAG_COLUMN = "诊断行为"
# 2026-08-09 Task 0: 第四状态「留生态」——基线 §8 状态值域扩展（ADR-074 登记先行，
# 域 10 tooltip 条目 = 留生态：设计意图成立但未落地，不列入已实现白名单）。
STATUSES = {"支持", "部分", "未支持", "留生态"}
# Task 9: 域 3 事件行的语义化分类列——语义化（首批 14 语义事件）/ 透传（其余）。
# 断言锚点 = SEMANTIC_EVENT_NAMES 常量（ADR-074 rust_str_array 提取模式），
# 脚本不硬编码 14/40——语义化集合随编译器常量漂移时门禁失败。
SEMANTIC_CLASS_COLUMN = "语义化"
SEMANTIC_CLASSES = {"语义化", "透传"}

# ── whitelist extraction (compiler ground truth) ─────────────────────────

def read_file(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


def event_canonical_names(text):
    """Canonical event names from the shared event table — the
    `=> Some("name")` match arms of `canonical_event_name`'s body.
    Scoped to the fn body: `legacy_spelling`'s arms ("onPress" => Some("onClick"),
    Task 2/3) are legacy-name mappings — those names are HARD-REJECTED, not
    implemented whitelist members, so they must not be extracted (登记册零失实)."""
    m = re.search(r'pub fn canonical_event_name.*?\{(.*?)\n\}', text, re.S)
    if not m:
        return None
    return sorted(set(re.findall(r'=>\s*Some\("([^"]+)"\)', m.group(1))))


def canonical_alias_map(text):
    """canonical_event_name 别名表（event_names.rs）——每个 lowercase
    `"stem" | … => Some("canonical")` 匹配臂的 stem → canonical（Task 9:
    解析 SEMANTIC_EVENT_NAMES 的 onXxx → canonical；legacy_spelling 的
    camelCase 臂 "onPress" 等不匹配 lowercase 模式，天然排除）。"""
    return {
        stem: canonical
        for chain, canonical in re.findall(
            r'((?:"[a-z]+"\s*\|\s*)*"[a-z]+")\s*=>\s*Some\("([a-z]+)"\)', text
        )
        for stem in re.findall(r'"([a-z]+)"', chain)
    }


def semantic_event_canonicals(text):
    """首批语义事件（SEMANTIC_EVENT_NAMES 常量）经 canonical_event_name
    别名表解析出的 canonical 集合——域 3「语义化」分类的编译器事实锚点
    （ADR-074 先例：断言锚点 = 编译器常量，脚本不硬编码 14/40）。
    提取失败返回 None（门禁必须失败，不静默放行）。"""
    names = rust_str_array(text, "SEMANTIC_EVENT_NAMES")
    if names is None:
        return None
    alias = canonical_alias_map(text)
    if not alias:
        return None
    resolved = set()
    for n in names:
        stem = n[2:].lower() if n.startswith("on") else n.lower()
        canonical = alias.get(stem)
        if canonical is None:
            return None
        resolved.add(canonical)
    return resolved


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


# ── 域 7 事实源提取（元素指令与绑定） ─────────────────────────────

def drag_binding_words(text):
    """@drag/@dragEnd 分派词——parse_event_prop 的 drag 分派分支
    (`event_name == "drag" || event_name == "dragEnd"`)。"""
    return sorted(set(re.findall(r'event_name == "(drag|dragEnd)"', text)))


def semantic_binding_words(text):
    """语义绑定词——SEMANTIC_BINDINGS 常量字符串成员
    (`const SEMANTIC_BINDINGS: &[&str] = &["focus", "scroll-x", "scroll-y"]`)。"""
    m = re.search(r'SEMANTIC_BINDINGS: &\[&str\] = &\[([^\]]+)\]', text)
    if not m:
        return None
    return sorted(set(re.findall(r'"([^"]+)"', m.group(1))))


def boolean_attr_keywords(text):
    """裸布尔属性关键字——peek_is_boolean_attr_keyword 的 matches! 分支
    (`Some("disabled") | Some("checked") | …`)。限定函数体以避免捕获
    测试模块中的 `Some("end")` 等断言字符串。"""
    m = re.search(
        r'fn peek_is_boolean_attr_keyword.*?matches!\((.*?)\)\s*\}', text, re.S
    )
    if not m:
        return None
    return sorted(set(re.findall(r'Some\("([a-z]+)"\)', m.group(1))))


def known_attr_words(text):
    """已知属性白名单 match arm（element.rs emit_element_props）内的引号名。

    限定范围：`"id" | "placeholder"` 起至 `"capture" => {` 止——白名单是 match
    arm 非数组常量，无法用 rust_str_array 提取（该 arm 是 match 首臂，无前导 `|`）。"""
    m = re.search(r'"id" \| "placeholder".*?"capture" => \{', text, re.S)
    if not m:
        return set()
    return set(re.findall(r'"([a-z-]+)"', m.group(0)))


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
    parts = re.split(r'^## 域 (\d+): ([^\n]+)', text, flags=re.M)
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


def event_classification(text):
    """域 3 语义集条目 (name, 语义化值) 对——Task 9 事件行分类列
    （语义化/透传）。提取失败（缺列/缺小节/表畸形）返回 None。"""
    sections = split_domains(text)
    if 3 not in sections:
        return None
    sub = subsection(sections[3][1], "语义集条目")
    if sub is None:
        return None
    headers, rows = table_rows(sub)
    if headers is None or rows is None:
        return None
    ci = column_index(headers, SEMANTIC_CLASS_COLUMN)
    if ci is None:
        return None
    return [(r[0], r[ci]) for r in rows]


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
    for n in sorted(DOMAIN_TITLES):  # v4: 含域 6（值绑定，5.3 新建）——此前 range(1,6) 对域 6 空转；2026-08-07: 含域 7（元素指令与绑定）；2026-08-09: 含域 8（表单）/域 9（数据三态，Task 7）/域 10（反馈模态，Task 0）——sorted(DOMAIN_TITLES) 自维护
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
                    # 2026-08-09 Task 0: 域 10 状态列带括号注记（「支持（v0：无手动关闭）」
                    # 「留生态（基线 §8 状态值域扩展）」）——按「（」前基词校验，域 1-9 全为
                    # 裸状态不受影响；括注内容由 per-domain 断言/文档本体承担。
                    base = row[si].split("（", 1)[0].strip()
                    if base not in STATUSES:
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
    # events (域 3, Task 9): 事件行语义化分类——「语义化」列 ∈ {语义化, 透传}；
    # 语义化集合 == SEMANTIC_EVENT_NAMES 常量经 canonical_event_name 别名表
    # 解析的 canonical 集合（ADR-074：断言锚点 = 编译器常量，不硬编码 14/40）；
    # 透传集合 == 全集 − 语义化（54 行须全分类：14 语义化 + 40 透传无漂移）
    cls3 = event_classification(universe)
    if cls3 is None:
        fail(
            f"事件: domain 3 '### 语义集条目' must have '{SEMANTIC_CLASS_COLUMN}' column "
            f"with values in {sorted(SEMANTIC_CLASSES)} (Task 9 分类列缺失)"
        )
    else:
        invalid = sorted({(n, c) for n, c in cls3 if c not in SEMANTIC_CLASSES})
        if invalid:
            fail(f"事件: invalid {SEMANTIC_CLASS_COLUMN} values {invalid} (must be {sorted(SEMANTIC_CLASSES)})")
        semantic_rows = {n for n, c in cls3 if c == "语义化"}
        expected_semantic = semantic_event_canonicals(read_file(EVENT_TABLE_FILE))
        if expected_semantic is None:
            fail("事件: cannot resolve SEMANTIC_EVENT_NAMES through canonical_event_name (extraction failed)")
        else:
            if semantic_rows != expected_semantic:
                fail(
                    f"事件: {SEMANTIC_CLASS_COLUMN} 语义化集合 ≠ SEMANTIC_EVENT_NAMES 常量解析的 canonical 集合\n"
                    f"    doc-only:      {sorted(semantic_rows - expected_semantic)}\n"
                    f"    compiler-only: {sorted(expected_semantic - semantic_rows)}"
                )
            all_names = {n for n, _ in cls3}
            passthrough_rows = {n for n, c in cls3 if c == "透传"}
            if passthrough_rows != all_names - semantic_rows:
                fail(
                    f"事件: {SEMANTIC_CLASS_COLUMN} 透传集合 ≠ 全集 − 语义化 "
                    f"（{len(all_names)} 行须全分类：语义化 + 透传 = 全集）"
                )

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

    # directives (域 7): 词法级条目走通用矩阵断言；机制级条目（布尔属性 /
    # style 动态绑定）走存在性断言——其语义集条目名非词法级，不进词集合比对
    wl7, en7 = whitelist(7), entries(7)
    element_src = read_file(ELEMENT_PARSER_FILE)
    drag_words = drag_binding_words(element_src)
    bind_words = semantic_binding_words(element_src)
    if bind_words is None:
        # drag_binding_words 永不返回 None（parse_event_prop drag 分派分支恒可匹配）——
        # 仅 bind_words（SEMANTIC_BINDINGS 常量提取）可能提取失败；显式 fail，不静默放行
        fail("directives: cannot cross-assert 已实现白名单 (source extraction failed)")
    else:
        lex_words = drag_words + bind_words + ["ref"]
        assert_set("directives", wl7, lex_words, "已实现白名单")
        if en7 is None:
            fail("directives: cannot cross-assert 语义集条目 (extraction failed)")
        else:
            lex_entries = [(e, s) for e, s in en7 if e not in ("style 动态绑定", "布尔属性")]
            assert_matrix("directives", lex_entries, wl7)
    # 机制级条目存在性断言——仅对 支持/部分 状态执行（未支持即诚实契约，
    # 未支持条目不得触发「declared 支持」误导消息；条目状态合法性已由
    # 通用 STATUSES 检查承担）
    if en7 is None:
        fail("directives: cannot cross-assert mechanism entries (语义集条目 extraction failed)")
    else:
        mech = {e: s for e, s in en7 if e in ("style 动态绑定", "布尔属性")}
        for name in ("style 动态绑定", "布尔属性"):
            if name not in mech:
                fail(f"directives: missing mechanism entry '{name}'")
        if mech.get("style 动态绑定") in ("支持", "部分"):
            if os.path.exists(DIRECT_WRITE_FILE):
                dw = read_file(DIRECT_WRITE_FILE)
                if "classify_direct_write" not in dw and "DirectWriteSets" not in dw:
                    fail("directives: style 动态绑定 declared 支持 but direct_write.rs lacks D1/D2 classification")
            else:
                fail("directives: style 动态绑定 declared 支持 but codegen/shared/direct_write.rs missing")
        if mech.get("布尔属性") in ("支持", "部分"):
            keywords = boolean_attr_keywords(element_src)
            if keywords is None or not keywords:
                fail("directives: 布尔属性 declared 支持 but peek_is_boolean_attr_keyword has no keywords")
            # 2026-08-10 升级（评审 MEDIUM-2）：存在性断言不足以捕获双源漂移——
            # parser 关键字 ⊆ codegen 布尔集 ∪ 已知属性白名单，否则新关键字可
            # 静默落入 data-* fallback（B2 bug 类：hidden 曾因此不工作）。
            omni_elem_src = read_file(OMNIVIEW_ELEMENT_FILE)
            codegen_bool = set(rust_str_array(omni_elem_src, "HTML_BOOLEAN_ATTRS") or [])
            missing = set(keywords) - codegen_bool - known_attr_words(omni_elem_src)
            if missing:
                fail(
                    "directives: parser 布尔关键字不在 codegen HTML_BOOLEAN_ATTRS ∪ 已知属性白名单: "
                    + ", ".join(sorted(missing))
                )

    # form (域 8, 2026-08-09 Task 7): 词法级规则名 == VALIDATE_RULES 双向断言；
    # 机制级条目（@form/formErrors/formValid/formSubmitting）走存在性断言
    # （ADR-074 先例——跨三目标锚点，无单一定义的词法白名单常量）
    wl8, en8 = whitelist(8), entries(8)
    validate_words = rust_str_array(read_file(VALIDATE_FORM_FILE), "VALIDATE_RULES")
    assert_set("form", wl8, validate_words, "已实现白名单")
    if en8 is None:
        fail("form: cannot cross-assert 语义集条目 (extraction failed)")
    else:
        lex_entries8 = [
            (e, s) for e, s in en8 if e not in ("@form", "formErrors", "formValid", "formSubmitting")
        ]
        assert_matrix("form", lex_entries8, wl8)
        lower_src = read_file(LOWER_COMPONENT_FILE)
        react_src = read_file(REACT_COMPONENT_FILE)
        flutter_src = read_file(FLUTTER_MOD_FILE)
        omniview_src = read_file(OMNIVIEW_STORE_FILE)
        for name, needles in (
            ("@form", [("collect_form_intent", lower_src)]),
            ("formErrors", [("setFormErrors", react_src), ("formErrors", flutter_src), ("_formErrors", omniview_src)]),
            ("formValid", [("formValid", react_src), ("formValid", flutter_src), ("_formValid", omniview_src)]),
            ("formSubmitting", [("formSubmitting", react_src), ("formSubmitting", flutter_src), ("_formSubmitting", omniview_src)]),
        ):
            if dict(en8).get(name) in ("支持", "部分"):
                for needle, haystack in needles:
                    if needle not in haystack:
                        fail(f"form: '{name}' 声明 支持/部分 但编译器锚点 '{needle}' 缺失")
            elif name not in dict(en8):
                fail(f"form: 缺失语义集条目 '{name}'")

    # data three-states (域 9, 2026-08-09 Task 7): 无词法级白名单常量——
    # @load/loading/error/data/retry 以编译器代码锚点存在性断言（域 6 值绑定先例）；
    # v0 边界（仅 Array 返回）记录于诊断行为列
    wl9, en9 = whitelist(9), entries(9)
    if en9 is None:
        fail("数据三态: cannot cross-assert 语义集条目 (extraction failed)")
    else:
        assert_matrix("数据三态", en9, wl9)
        lower_src = read_file(LOWER_COMPONENT_FILE)
        react_src = read_file(REACT_COMPONENT_FILE)
        flutter_src = read_file(FLUTTER_MOD_FILE)
        omniview_src = read_file(OMNIVIEW_STORE_FILE)
        for name, needles in (
            ("@load", [("load_intent", lower_src)]),
            ("loading", [("setLoading", react_src), ("_loading", omniview_src)]),
            ("error", [("setError", react_src), ("_error", omniview_src)]),
            ("data", [("setData", react_src), ("_data", omniview_src)]),
            ("retry", [("ref.invalidate", flutter_src), ("_retry", omniview_src)]),
        ):
            if dict(en9).get(name) in ("支持", "部分"):
                for needle, haystack in needles:
                    if needle not in haystack:
                        fail(f"数据三态: '{name}' 声明 支持/部分 但编译器锚点 '{needle}' 缺失")
            elif name not in dict(en9):
                fail(f"数据三态: 缺失语义集条目 '{name}'")

    # feedback & modal (域 10, 2026-08-09 Task 0 登记 → 2026-08-10 Task 7 全量断言):
    # showToast/@modal/焦点陷阱/@skeleton/@errorBoundary/tooltip——机制级条目居多
    # （指令名/内建函数名，无词法级白名单常量），本域语法已随 Task 1-6 落地，故断言
    # 两层：① 条目 6 行存在 + tooltip 状态 = 留生态（基线 §8 状态值域第四值，未落地
    # 故不进已实现白名单）+ 已实现白名单 3 行存在（Task 0 存在性检查，保持）；
    # ② 「支持/部分」条目逐项编译器代码锚点（照域 8/9 锚点账本模式）——语义验证器 +
    # IR 变体 + React/OmniView codegen（Flutter 锚点暂缺：Task 5 Flutter codegen 待
    # 落地，不设 Flutter 针）
    wl10, en10 = whitelist(10), entries(10)
    if en10 is None:
        fail("反馈模态: cannot cross-assert 语义集条目 (extraction failed)")
    else:
        en10_map = dict(en10)
        for name in ("showToast", "@modal", "焦点陷阱", "@skeleton", "@errorBoundary", "tooltip"):
            if name not in en10_map:
                fail(f"反馈模态: 缺失语义集条目 '{name}'")
        if en10_map.get("tooltip", "").split("（", 1)[0].strip() != "留生态":
            fail(f"反馈模态: tooltip 实现状态应 = 留生态（基线 §8 状态值域扩展），got '{en10_map.get('tooltip')}'")
        # 锚点账本（②层，2026-08-10 Task 7）：支持/部分 条目 → 编译器事实源代码锚点。
        # 事实源文件：语义验证器 validate_feedback_modal.rs（V1-V14 消息源）、IR 变体
        # （ir/types.rs：BuiltinToastCall/ModalIntentIR/SkeletonBindingIR/
        # ErrorBoundaryBindingIR）、React codegen（ToastProvider/showModal()/
        # ErrorBoundary/omni-skeleton）、OmniView codegen（ensureToastContainer/
        # omni-modal-mask/Focus trap/omni-skeleton/omni-error-boundary）。tooltip 为
        # 留生态（未落地）——不设锚点，由下方 in ("支持", "部分") 守卫自动跳过。
        feedback_src = read_file(FEEDBACK_MODAL_FILE)
        ir_src = read_file(REFOP_IR_FILE)
        react_src = read_file(REACT_COMPONENT_FILE)
        omni_modal_src = read_file(OMNIVIEW_MODAL_FILE)
        omni_elem_src = read_file(OMNIVIEW_ELEMENT_FILE)
        omni_expr_src = read_file(OMNIVIEW_EXPR_FILE)
        for name, needles in (
            ("showToast", [("BuiltinToastCall", ir_src), ("ToastProvider", react_src),
                           ("ensureToastContainer", omni_expr_src)]),
            ("@modal", [("validate_feedback_modal", feedback_src), ("ModalIntentIR", ir_src),
                        ("showModal", react_src), ("omni-modal-mask", omni_modal_src)]),
            ("焦点陷阱", [("Focus trap", omni_modal_src)]),
            ("@skeleton", [("SkeletonBindingIR", ir_src), ("omni-skeleton", react_src),
                           ("omni-skeleton", omni_elem_src)]),
            ("@errorBoundary", [("ErrorBoundaryBindingIR", ir_src), ("ErrorBoundary", react_src),
                                ("omni-error-boundary", omni_elem_src)]),
        ):
            if en10_map.get(name) in ("支持", "部分"):
                for needle, haystack in needles:
                    if needle not in haystack:
                        fail(f"反馈模态: '{name}' 声明 支持/部分 但编译器锚点 '{needle}' 缺失")
    if wl10 is None:
        fail("反馈模态: cannot cross-assert 已实现白名单 (extraction failed)")
    else:
        for name in ("dialog 元素", "showToast 调用", "@modal/@skeleton/@errorBoundary"):
            if name not in wl10:
                fail(f"反馈模态: 缺失已实现白名单行 '{name}'")
        if "tooltip" in wl10:
            fail("反馈模态: tooltip 为留生态条目（未落地），不得出现在已实现白名单")

    # navigation (域 11, 2026-08-10 批次 2 Task 1 登记 → Task 9 回填): 机制级条目
    # （组件级指令 @route/@layout/@query + 内建函数 navigate/back + 路径参数注入/
    # guard/lazy——无词法级白名单常量，语法已随批次 2 Task 2-8 落地）。本任务断言
    # 两层（照域 8/9 锚点账本先例 + 域 10 ①层存在性先例）：① 语义集条目 8 行存在
    # + 实现状态 = 支持 + 已实现白名单 6 行存在；② 「支持/部分」条目逐项编译器
    # 代码锚点账本——解析器（parse_route_decl/parse_layout_decl/parse_query_decl/
    # parse_path_params）、IR lowering（collect_route_intent/collect_layout_intent/
    # collect_query_params）、内建函数登记（BUILTIN_FUNCTIONS 内 navigate/back）、
    # 语义验证器（ROUTE_VALIDATE_RULES/NAVIGATE_RESERVED）、三目标 codegen
    # （createBrowserRouter/lazy_for、GoRoute、__omni_routes）。
    wl11, en11 = whitelist(11), entries(11)
    if en11 is None:
        fail("导航: cannot cross-assert 语义集条目 (extraction failed)")
    else:
        en11_map = dict(en11)
        for name in ("@route 页面声明", "路径参数 prop 注入", "@query 查询参数", "guard 守卫",
                     "@layout 嵌套布局", "navigate 内建函数", "back 内建函数", "lazy 懒加载"):
            if name not in en11_map:
                fail(f"导航: 缺失语义集条目 '{name}'")
            elif en11_map[name].split("（", 1)[0].strip() != "支持":
                fail(f"导航: '{name}' 实现状态应 = 支持（批次 2 已落地），got '{en11_map[name]}'")
        # 保留名无重断言（Task 9 测试点 test_registry_no_reserved_name_collision）：
        # navigate/back（V13 保留名）∩ 既有编译器保留名 = ∅。既有保留名事实源 =
        # 内建语义函数 showToast（validate_feedback_modal.rs V8 保留名判定）+ 表单
        # 数据三态内建信号（formErrors/formValid/formSubmitting/loading/error/data/
        # retry，V-F8 @form/@load 保留名判定）。
        existing_reserved = {"showToast", "formErrors", "formValid", "formSubmitting",
                             "loading", "error", "data", "retry"}
        nav_reserved = {"navigate", "back"}
        overlap = sorted(nav_reserved & existing_reserved)
        if overlap:
            fail(f"导航: 保留名 navigate/back 与既有保留名表冲突: {overlap}")
        # 锚点账本（②层）：「支持/部分」条目 → 编译器事实源代码锚点（照域 8/9
        # needle-ledger 模式——针在 haystack 中的存在性检查）。
        route_parser_src = read_file(ROUTE_PARSER_FILE)
        lower_route_src = read_file(LOWER_ROUTE_FILE)
        expr_src = read_file(EXPR_MOD_FILE)
        validate_route_src = read_file(VALIDATE_ROUTE_FILE)
        react_router_src = read_file(REACT_ROUTER_FILE)
        flutter_router_src = read_file(FLUTTER_ROUTER_FILE)
        omniview_router_src = read_file(OMNIVIEW_ROUTER_FILE)
        for name, needles in (
            ("@route 页面声明", [("parse_route_decl", route_parser_src),
                                 ("collect_route_intent", lower_route_src)]),
            ("路径参数 prop 注入", [("parse_path_params", route_parser_src),
                                    ("params", lower_route_src)]),
            ("@query 查询参数", [("parse_query_decl", route_parser_src),
                                 ("collect_query_params", lower_route_src)]),
            ("guard 守卫", [("guard", route_parser_src),
                            ("ROUTE_VALIDATE_RULES", validate_route_src)]),
            ("@layout 嵌套布局", [("parse_layout_decl", route_parser_src),
                                  ("collect_layout_intent", lower_route_src)]),
            ("navigate 内建函数", [('"navigate"', expr_src),
                                   ("createBrowserRouter", react_router_src),
                                   ("GoRoute", flutter_router_src),
                                   ("__omni_routes", omniview_router_src)]),
            ("back 内建函数", [('"back"', expr_src),
                               ("NAVIGATE_RESERVED", validate_route_src)]),
            ("lazy 懒加载", [("lazy", route_parser_src),
                             ("lazy_for", react_router_src)]),
        ):
            if en11_map.get(name) in ("支持", "部分"):
                for needle, haystack in needles:
                    if needle not in haystack:
                        fail(f"导航: '{name}' 声明 {en11_map[name]} 但编译器锚点 '{needle}' 缺失")
    if wl11 is None:
        fail("导航: cannot cross-assert 已实现白名单 (extraction failed)")
    else:
        for name in ("@route/@layout/@query 解析", "guard/lazy/redirect 校验",
                     "navigate/back 语义函数", "React 路由接线", "Flutter GoRouter 接线",
                     "OmniView hash 路由"):
            if name not in wl11:
                fail(f"导航: 缺失已实现白名单行 '{name}'")

    # responsive (域 12, 2026-08-11 批次 3 Task 1 登记先行): 机制级条目居多
    # （breakpoint 声明关键字 + bp() 表达式函数形态——无词法级白名单常量，语法随
    # T2-T8 落地）。断言两层：① 语义集条目 6 行存在（含「选择器诊断」——spec/plan
    # 白名单表含此行但语义集条目缺失，执行时修正补齐，assert_matrix 防孤儿白名单）
    # + 白名单 5 行存在；② 「支持/部分」条目逐项编译器代码锚点账本（照域 8/9 先例）。
    # ⚠️ T1 阶段锚点未落地 = pending-red（文档化预期，计划 T1 Step 5 注记）；T5/T6/T8
    # 落地后转绿。禁以提前实现语法转绿。
    wl12, en12 = whitelist(12), entries(12)
    if en12 is None:
        fail("响应式: cannot cross-assert 语义集条目 (extraction failed)")
    else:
        en12_map = dict(en12)
        for name in ("breakpoint 声明", "bp() 条件", "选择器诊断", "@media 块", "@container 块", "深色模式"):
            if name not in en12_map:
                fail(f"响应式: 缺失语义集条目 '{name}'")
        design_parser_src = read_file(DESIGN_PARSER_FILE)
        design_ir_src = read_file(DESIGN_IR_FILE)
        analyse_src = read_file(ANALYSE_FILE)
        style_parser_src = read_file(STYLE_PARSER_FILE)
        lower_comp_src = read_file(LOWER_COMPONENT_FILE)
        react_src = read_file(REACT_COMPONENT_FILE)
        flutter_src = read_file(FLUTTER_MOD_FILE)
        flutter_ref_src = read_file(FLUTTER_REF_FILE)
        omni_expr_src = read_file(OMNIVIEW_EXPR_FILE)
        for name, needles in (
            ("breakpoint 声明", [("breakpoint", design_parser_src), ("breakpoints", design_ir_src)]),
            ("bp() 条件", [("validate_breakpoints", analyse_src), ("useMediaQuery", react_src),
                          ("MediaQuery.sizeOf", flutter_ref_src), ("matchMedia", omni_expr_src)]),
            ("选择器诊断", [("validate_style_selectors", analyse_src), ("unsupported_selector", lower_comp_src)]),
            ("@media 块", [("AtMedia", style_parser_src), ("@media", react_src),
                          ("media_blocks", flutter_src)]),
            ("深色模式", [("prefers-color-scheme", read_file(SHARED_DESIGN_FILE))]),
        ):
            # 状态列带括号注记（「支持（批次 3）」）——按「（」前基词判定（域 14 基词
            # 校验同模式；域 10 锚点断言用裸 in ("支持","部分") 会静默跳过，此处不做）。
            if en12_map.get(name, "").split("（", 1)[0].strip() in ("支持", "部分"):
                for needle, haystack in needles:
                    if needle not in haystack:
                        fail(f"响应式: '{name}' 声明 {en12_map[name]} 但编译器锚点 '{needle}' 缺失")
    if wl12 is None:
        fail("响应式: cannot cross-assert 已实现白名单 (extraction failed)")
    else:
        for name in ("breakpoint 声明", "bp() 条件", "@media 块", "选择器诊断", "深色模式"):
            if name not in wl12:
                fail(f"响应式: 缺失已实现白名单行 '{name}'")

    # styling (域 14, 2026-08-11 批次 3 Task 1): 单表留生态登记行（无三子节结构——
    # 不走通用 REQUIRED_SUBSECTIONS 循环，专项校验）。状态值域 ∈ STATUSES（含
    # 第四值「留生态」，基线 §8 状态值域扩展）；三行已知状态断言防登记回退。
    # 显式存在性守卫（评审 MEDIUM-2 修正）——整节被删时必须 fail，不静默跳过。
    if 14 not in sections:
        fail("domain 14 missing (expected '## 域 14: …样式…'——留生态登记行，批次 3 T1)")
    else:
        headers14, rows14 = table_rows(sections[14][1])
        if headers14 is None or rows14 is None:
            fail("域 14: 留生态登记表必须为良构表格（header + separator + ≥1 行，首列非空）")
        else:
            si14 = column_index(headers14, STATUS_COLUMN)
            if si14 is None:
                fail(f"域 14: 表格必须含 '{STATUS_COLUMN}' 列（got {headers14}）")
            else:
                for row in rows14:
                    base = row[si14].split("（", 1)[0].strip()
                    if base not in STATUSES:
                        fail(f"域 14: 条目 '{row[0]}' 状态 '{row[si14]}' 不在 {sorted(STATUSES)}")
                en14_map = {r[0]: r[si14] for r in rows14}
                for name, expected in (
                    ("复杂选择器（兄弟/子/属性/带参伪类/伪元素/选择器列表）", "留生态"),
                    ("@container 查询块", "留生态（候选）"),
                    ("@media 块", "部分（域 12 同载）"),  # spec §5.2 原文（含跨域注记）
                ):
                    if name not in en14_map:
                        fail(f"域 14: 缺失登记行 '{name}'")
                    elif en14_map[name] != expected:
                        fail(f"域 14: '{name}' 状态应 = '{expected}'，got '{en14_map[name]}'")

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
