# OmniView 语法挖掘三方对齐 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 通过挖掘编译器事实源，完成语义集登记册（域 7 元素指令与绑定）、docs/omniview 文档、learn-omniview demo 三方对齐。

**Architecture:** 编译器是 ground truth（parser/semantic/IR/codegen 白名单）。以 5 个验证脚本为门禁网：先登记域 7（semantic-universe.md），再扩展脚本断言（verify-semantic-universe.py 域 7 交叉断言 + verify-demo-coverage.py 域 7 审计接入），随后文档核对补漏与 demo 补齐，最后门禁全绿交付。编译器 bug 只列清单不修（用户拍板：保守）。

**Tech Stack:** Python 验证脚本（verify-semantic-universe.py / verify-demo-coverage.py）、Markdown 文档（semantic-universe.md / docs/omniview 16 份）、OmniLang 展示项目（.omni 源码）、omnic CLI。

## Global Constraints

- 文档/demo/登记册**对齐编译器现状**；确认的编译器 bug 只记入 `tasks/omniview-syntax-audit-bugs.md`，不修
- 域 7 一次全收 **8 成员**：drag / dragEnd / ref / focus / scroll-x / scroll-y / style 动态绑定 / 布尔属性
- 语义集维护约定：新语法先登记 → 才可进 BNF/文档/白名单；`verify-semantic-universe.py` 双向断言使漂移即门禁失败
- 所有门禁通过标准：`0 problem`（verify-docs.py 允许 NOTE 级输出）
- demo 改动必须过 WASM 编译门禁（`omnic build` learn-omniview，OmniView 目标）
- 脚本用 UTF-8 运行（Windows GBK 控制台会崩）：`PYTHONIOENCODING=utf-8 python scripts/<name>.py`

---

### Task 1: 语义集登记册域 7 登记（含 spec 事实源修正）

**Files:**
- Modify: `docs/superpowers/specs/semantic-universe.md`（文件末尾追加域 7）
- Modify: `docs/superpowers/specs/2026-08-07-omniview-syntax-three-way-alignment-design.md:56`（属性三态事实源描述修正）

**Interfaces:**
- Produces: 域 7 三节结构（语义域定义 / 语义集条目 / 已实现白名单），节名与既有 6 域完全一致——Task 2 脚本断言依赖这些节名与表格格式

**背景（挖掘事实，非占位符）**：
- 属性三态事实源是**双源**：parser `peek_is_boolean_attr_keyword`（`crates/omnilang-core/src/parser/view/element.rs:38-59`，15 关键字：controls/disabled/readonly/required/checked/selected/autofocus/multiple/loop/muted/async/defer/hidden/autocomplete/autoplay）+ OmniView codegen `HTML_BOOLEAN_ATTRS`（`omnilang/src/codegen/omniview/view/element.rs:18`，12 项）。设计文档写的"19 项（omniview codegen）"是**事实错误**——19 项是 Poiesis TS 侧常量（`docs/superpowers/specs/2026-08-06-view-attributes-design.md` D4），非编译器事实源。本项目前须修正。
- @drag/@dragEnd 解析：`parse_event_prop`（element.rs:681 `event_name == "drag" || event_name == "dragEnd"`），语义验证：`validate_drag.rs`（6 验证项：需 @ref / handler 存在 / 参数 ≤1 / @dragEnd 不可单独 / 不可重复 / 顺序无关）
- 语义绑定全集：`SEMANTIC_BINDINGS` 常量（element.rs:89）= `["focus", "scroll-x", "scroll-y"]`，绑定值须裸 signal 引用（`{sig}` 或 `{Store.sig}`）
- @ref：`ref_binding`（ElementIR 字段）+ `RefDeclIR`（ir/types.rs）
- style 动态绑定直写通道：`omnilang/src/codegen/shared/direct_write.rs`（D1/D2/B2 信号域分类，三目标发射）

- [ ] **Step 1: 修正设计文档属性三态事实源描述**

`docs/superpowers/specs/2026-08-07-omniview-syntax-three-way-alignment-design.md` 域 7 成员表最后一行改为：

```markdown
| 属性三态 | `class="x"` 裸布尔 / `prop={expr}` 动态属性 | parser `peek_is_boolean_attr_keyword`（15 关键字，element.rs:38）+ OmniView codegen `HTML_BOOLEAN_ATTRS`（12 项）+ Poiesis TS `BOOLEAN_ATTRS`（19 项，编辑器侧） | 支持 |
```

- [ ] **Step 2: 在 semantic-universe.md 文件末尾追加域 7**

```markdown
---

## 域 7: 元素指令与绑定（directives）

### 语义域定义

元素属性层语义域——**声明在元素上的指令与绑定**（元素可拖拽 / 元素可被引用 / 声明式 DOM 副作用 / 动态样式 / 属性形态），与事件（域 3）和命令式 RefOp（域 4）互补：指令表达意图（`@drag` 拖拽意图——指针捕获由编译器按目标选路）、绑定表达声明（`@ref` 引用、`focus=`/`scroll-x=`/`scroll-y=` 语义绑定驱动原生 DOM 副作用）、样式与属性表达形态（`style: { prop: sig }` 动态条目经直写通道落目标、裸布尔属性三态）。语义词汇 = 指令/绑定名本体（`drag`/`ref`/`focus` 等——`@` 前缀与 `=` 后缀是 DSL 表面形态）。全集封闭按本表（2026-08-07 登记）：未列元素属性层功能视为未登记，扩充需设计评审 + 登记册 + 三目标验证（@drag 入册先例，ADR-073）。

### 语义集条目

| 语义集条目 | 语义 | 实现状态 | 诊断行为 |
|-----------|------|---------|---------|
| drag | 拖拽意图指令（指针捕获编译器选路：React setPointerCapture / OmniView capturePointer 组合 / Flutter GestureDetector pan） | 支持 | 无（三目标）；**语义验证硬错误**：需同元素 @ref、handler 必须为组件函数名（禁止内联箭头/调用）、参数 ≤1、@dragEnd 不可单独出现、二者不可重复（validate_drag.rs 6 验证项） |
| dragEnd | 拖拽结束回调（指针抬起/取消） | 支持 | 同上 |
| ref | 元素引用绑定（`@ref={name}` 或 `ref: name`，等价） | 支持 | 无（三目标；@ref 声明见域 4 RefOp 关联） |
| focus | 声明式焦点绑定（`focus={sig}`，true → .focus() / false → .blur()） | 支持 | 无（三目标）；绑定值须裸 signal 引用（`{sig}` 或 `{Store.sig}`），其他表达式为普通 prop |
| scroll-x | 声明式水平滚动绑定 | 支持 | 同上（SEMANTIC_BINDINGS 常量成员） |
| scroll-y | 声明式垂直滚动绑定 | 支持 | 同上 |
| style 动态绑定 | `style: { prop: sig }` 动态条目——信号驱动样式经直写通道（D1/D2 域分类）落目标：React useRef+DOM 直写（零重渲染）/ Flutter ValueNotifier+ListenableBuilder（零 setState 整树重建）/ OmniView el.style.setProperty | 支持 | 无（三目标）；**Flutter 编译期诊断**：不可映射值（opacity/color 等）error_at 显式报错；数值 dimension（width/height/padding/margin）React/OmniView 自动 px 模板 |
| 布尔属性 | 裸布尔属性形态（`disabled`、`checked` 等 15 关键字，`<input checked />` 与 `checked={sig}` 均合法） | 支持 | 无（三目标）；parser 15 关键字 = peek_is_boolean_attr_keyword；OmniView codegen HTML_BOOLEAN_ATTRS 12 项共享 |

### 已实现白名单

| 条目 | 编译器事实源 |
|------|-------------|
| drag | `parse_event_prop` drag 分派（`crates/omnilang-core/src/parser/view/element.rs` ~:681）+ `validate_drag.rs` + `DragBindingIR`（ir/types.rs） |
| dragEnd | 同上（merge 步骤 push_element_prop 成对合并） |
| ref | ElementIR `ref_binding` + `RefDeclIR`（ir/types.rs） |
| focus | `SEMANTIC_BINDINGS` 常量（parser/view/element.rs:89） |
| scroll-x | 同上 |
| scroll-y | 同上 |
```

> 注：**style 动态绑定** 与 **布尔属性** 为机制级条目（非词法级），白名单交叉断言以存在性检查承担（见 verify-semantic-universe.py 域 7 断言 Task 2）；本白名单表只列 6 个词法级条目。

- [ ] **Step 3: 跑 verify-semantic-universe.py 确认登记册结构无误**

Run: `PYTHONIOENCODING=utf-8 python scripts/verify-semantic-universe.py --check`
Expected: `semantic-universe gate: 0 problem(s)`——域 7 尚不在脚本断言范围（range(1,7)），本步只验证既有 6 域未被破坏

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/semantic-universe.md docs/superpowers/specs/2026-08-07-omniview-syntax-three-way-alignment-design.md
git commit -m "docs(universe): 域 7 元素指令与绑定登记——8 成员（drag/dragEnd/ref/focus/scroll-x/scroll-y/style 动态绑定/布尔属性）+ spec 属性三态事实源修正"
```

---

### Task 2: verify-semantic-universe.py 域 7 断言扩展

**Files:**
- Modify: `scripts/verify-semantic-universe.py`

**Interfaces:**
- Consumes: Task 1 域 7 三节结构（`## 域 7: 元素指令与绑定（directives）` 标题、`语义域定义`/`语义集条目`/`已实现白名单` 三节、8 条目表、6 词白名单表）
- Produces: 域 7 断言函数 `domain7_checks(problems)`——Task 3 不依赖，但门禁链后续任务依赖其通过

**背景**：脚本现有 `DOMAIN_TITLES = {1..6}`、`for n in range(1, 7)`、每域 `assert_set`（白名单 == 编译器词集合）+ `assert_matrix`（条目状态矩阵 ↔ 白名单）。域 7 有 6 个词法级条目（走通用断言）+ 2 个机制级条目（布尔属性/style 动态绑定，走存在性断言——因其语义集条目名非词法级，无法进词集合比对）。

- [ ] **Step 1: 扩展 DOMAIN_TITLES 与主循环**

文件顶部 `DOMAIN_TITLES = {...}` 加一行：

```python
    7: "元素指令",  # 2026-08-07: directives 域——drag/dragEnd/ref/语义绑定/style 动态绑定/布尔属性
```

主循环 `for n in range(1, 7):` 改为：

```python
    for n in range(1, 8):  # 2026-08-07: 含域 7（元素指令与绑定）
```

- [ ] **Step 2: 新增事实源提取函数（放在既有提取函数区，如 `flutter_callback_names` 之后）**

```python
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
    (`Some("disabled") | Some("checked") | …`)。"""
    return sorted(set(re.findall(r'Some\("([a-z]+)"\)', text)))
```

- [ ] **Step 3: 新增域 7 断言（放在 `bnf_doc_subset("motion", ...)` 之前、既有 6 域断言之后——按文档顺序）**

```python
    # directives (域 7): 词法级条目走通用矩阵断言；机制级条目（布尔属性 /
    # style 动态绑定）走存在性断言——其语义集条目名非词法级，不进词集合比对
    ELEMENT_PARSER_FILE = os.path.join(
        REPO_ROOT, "crates", "omnilang-core", "src", "parser", "view", "element.rs"
    )
    DIRECT_WRITE_FILE = os.path.join(
        REPO_ROOT, "omnilang", "src", "codegen", "shared", "direct_write.rs"
    )
    wl7, en7 = whitelist(7), entries(7)
    element_src = read_file(ELEMENT_PARSER_FILE)
    lex_words = drag_binding_words(element_src) + semantic_binding_words(element_src) + ["ref"]
    if lex_words is None:
        fail("directives: cannot cross-assert 已实现白名单 (source extraction failed)")
    else:
        assert_set("directives", wl7, lex_words, "已实现白名单")
        lex_entries = [(e, s) for e, s in (en7 or []) if e not in ("style 动态绑定", "布尔属性")]
        assert_matrix("directives", lex_entries, wl7)
    # 机制级条目存在性断言
    if en7 is None:
        fail("directives: cannot cross-assert mechanism entries (语义集条目 extraction failed)")
    else:
        mech = {e: s for e, s in en7 if e in ("style 动态绑定", "布尔属性")}
        for name in ("style 动态绑定", "布尔属性"):
            if name not in mech:
                fail(f"directives: missing mechanism entry '{name}'")
                continue
            if mech[name] not in ("支持", "部分"):
                fail(f"directives: mechanism entry '{name}' must be 支持/部分 (implemented), got '{mech[name]}'")
        if os.path.exists(DIRECT_WRITE_FILE):
            dw = read_file(DIRECT_WRITE_FILE)
            if "classify_direct_write" not in dw and "DirectWriteSets" not in dw:
                fail("directives: style 动态绑定 declared 支持 but direct_write.rs lacks D1/D2 classification")
        else:
            fail("directives: style 动态绑定 declared 支持 but codegen/shared/direct_write.rs missing")
        if boolean_attr_keywords(element_src) is None or not boolean_attr_keywords(element_src):
            fail("directives: 布尔属性 declared 支持 but peek_is_boolean_attr_keyword has no keywords")
```

> 注意：`ELEMENT_PARSER_FILE`/`DIRECT_WRITE_FILE` 路径常量放在函数级会重复定义——建议放模块级常量区（与 `MOTION_ANIM_FILE` 等并列）。实现时如遇 `read_file` 已存在则直接复用。

- [ ] **Step 4: 跑脚本验证 RED（Task 1 的登记册若与事实源不一致则报错）**

Run: `PYTHONIOENCODING=utf-8 python scripts/verify-semantic-universe.py --check`
Expected: `0 problem(s)`——若报差异，按报错修 Task 1 登记册内容（对齐编译器事实源，**不要**改脚本断言去迁就登记册）

- [ ] **Step 5: Commit**

```bash
git add scripts/verify-semantic-universe.py
git commit -m "test(universe): 域 7 断言扩展——DOMAIN_TITLES/range(1,8) + drag/语义绑定/布尔属性事实源提取 + 机制级条目存在性断言"
```

---

### Task 3: verify-demo-coverage.py 域 7 审计接入 + EXEMPT 更新

**Files:**
- Modify: `scripts/verify-demo-coverage.py`

**Interfaces:**
- Consumes: Task 1 域 7 条目名（drag/dragEnd/ref/focus/scroll-x/scroll-y/style 动态绑定/布尔属性）
- Produces: 域 7 demo 覆盖审计（Task 5 用它驱动 demo 补齐）

**背景**：脚本 `usage(domain, name)` 按域分派正则匹配 demo 文本；域 7 需新增分支。EXEMPT 集含 `("4", "capturePointer")`/`("4", "releasePointer")`——@drag 入册后豁免理由更新。

- [ ] **Step 1: usage 函数新增域 7 分支（在既有分支后、`return 0` 前）**

```python
    if domain == "7":
        if name == "drag":
            return len(re.findall(r"@drag\s*=", demo_text)) - len(re.findall(r"@drag-horizontal\s*=|@drag-vertical\s*=", demo_text))
        if name == "dragEnd":
            return len(re.findall(r"@dragEnd\s*=", demo_text))
        if name == "ref":
            return len(re.findall(r"@ref\s*=|(?<![\w@])ref:\s*", demo_text))
        if name in ("focus", "scroll-x", "scroll-y"):
            return len(re.findall(rf"{re.escape(name)}\s*=\s*{{", demo_text))
        if name == "style 动态绑定":
            return len(re.findall(r"style:\s*\{", demo_text))
        if name == "布尔属性":
            return len(re.findall(
                r"\b(?:controls|disabled|readonly|required|checked|selected|autofocus|multiple|loop|muted|async|defer|hidden|autocomplete|autoplay)\s*=",
                demo_text,
            ))
        return 0
```

> `@drag` 计数排除 `@drag-horizontal`/`@drag-vertical`（scrubbing 触发器，域 1 motion 语义，避免误计）。

- [ ] **Step 2: EXEMPT 更新**

```python
EXEMPT = {
    ("3", "resize"),  # window 级事件——元素绑定不合法（React 无元素 onResize）；DSL 无 window 绑定表面
    ("3", "animationstart"), ("3", "animationend"), ("3", "animationiteration"),
    # 动画族——触发需宿主 CSS @keyframes（DSL 无）；绑定可写但展示为「需宿主动画」说明面板
    ("4", "capturePointer"), ("4", "releasePointer"),
    # ADR-073 决策 3：显式 capturePointer 降级为编译器内部细节，demo 源码不再书写——
    # 指针捕获意图由 ElementRefDemo 的 @drag/@dragEnd 绑定承担（域 7 已登记，意图形态
    # 由 @drag 承担同一演示语义）
}
```

- [ ] **Step 3: 跑脚本看域 7 缺口**

Run: `PYTHONIOENCODING=utf-8 python scripts/verify-demo-coverage.py`
Expected: 输出含 `── 域 7: 支持/部分 8 成员`，未展示成员列出（预计：scroll-x、布尔属性、style 动态绑定视 demo 现状）——缺口清单是 Task 5 的输入

- [ ] **Step 4: Commit**

```bash
git add scripts/verify-demo-coverage.py
git commit -m "test(demo): verify-demo-coverage 域 7 审计接入——drag/dragEnd/ref/语义绑定/布尔属性正则 + EXEMPT 豁免理由更新"
```

---

### Task 4: 文档核对补漏

**Files:**
- Verify: `docs/omniview/` 16 份（重点 4 份，其余快速核对）
- Modify: 核对发现的文档失实处（对齐编译器事实源）

**Interfaces:**
- Consumes: 挖掘事实（Task 1 背景节）+ Task 2/3 脚本断言结果
- Produces: 文档失实修正；`tasks/omniview-syntax-audit-bugs.md` 编译器 bug 清单（如发现）

**核对清单**（已知覆盖良好项只确认不重写）：

| 文档 | 核对项 | 预期 |
|------|--------|------|
| 06-templates.md | 布尔属性示例含 `<details open={isOpen}>`——parser 15 关键字**无 open** | **差异疑点**：Step 2 验证 |
| 06-templates.md | 语义绑定节（focus/scroll-x/scroll-y 全集） | 已覆盖（§79-168）确认 |
| 07-events-and-bindings.md | @drag/@dragEnd 意图指令节 | 已覆盖（d4d1d0e6）确认 |
| 09-styling.md | 直写通道行为 | 已覆盖（d4d1d0e6）确认 |
| 05-components.md | @ref/语义绑定/高频拖拽 | 已覆盖确认 |
| 15-grammar-view.md | @drag BNF | 已覆盖（d4d1d0e6）确认 |
| 00-cheatsheet.md | @drag/直写条目 | 已覆盖确认 |
| 其余 9 份 | 快速核对（grep 关键语法点是否存在明显失实） | 无改动预期 |

- [ ] **Step 1: 验证 `<details open={isOpen}>` 是否可编译**

写最小测试文件 `$CLAUDE_JOB_DIR/tmp/open-attr-test.omni`（背景会话临时目录，勿放仓库）：

```omni
import omniview;
component OpenAttrTest {
    signal isOpen: Bool = false;
    view {
        <details open={isOpen}><summary>t</summary><p>body</p></details>
    }
}
```

Run: `omnic check <tmp>/open-attr-test.omni`
- 若编译通过 → `open` 经其他路径合法（记入差异清单：文档示例合法）→ 无需改文档
- 若编译失败（`open` 被解析为普通 prop / 未识别）→ **文档对齐编译器**：06-templates.md 布尔属性示例删除 `<details open={isOpen}>` 行（或改用 `disabled` 示例），并把「文档宣称但编译器不支持 open 布尔」记入 `tasks/omniview-syntax-audit-bugs.md` 待批（不修编译器）

- [ ] **Step 2: 快速核对其余 9 份（如无失实则无文件改动）**

对每份文档执行：`grep -n "<语法点>" docs/omniview/0X-*.md` 抽查 2-3 个关键语法点与编译器事实比对（语法点清单：@if/@for、bind:、emit、@motion 触发器、@store、variant、@themes、import/export）。发现失实 → 修文档对齐编译器；编译器行为与文档矛盾 → 记 bug 清单。

- [ ] **Step 3: 若 Step 1 编译失败——修改 06-templates.md**

删除布尔属性示例中 `<details open={isOpen}>...</details>` 行，其余保留（具体替换文本以 Step 1 实际结果为准；若编译通过跳过本步）。

- [ ] **Step 4: 建立 bug 清单文件（无论 Step 1 结果）**

创建 `tasks/omniview-syntax-audit-bugs.md`：

```markdown
# OmniView 语法审计——编译器 bug 清单（待批，不修）

**原则**：本次审计只列不改（用户拍板：文档对齐编译器保守处置）。修复另排期。

| # | 语法点 | 编译器事实 | 文档宣称 | 差异 | 建议 |
|---|--------|-----------|---------|------|------|
| B1 | `open` 布尔属性 | parser 15 关键字无 open | 06-templates.md 布尔属性示例使用 `<details open={isOpen}>` | （Step 1 实测结果填写：编译通过则关闭，失败则保留） | 待批 |
```

- [ ] **Step 5: Commit**

```bash
git add docs/omniview/ tasks/omniview-syntax-audit-bugs.md
git commit -m "docs(omniview): 语法核对补漏——<details open> 差异处置 + 编译器 bug 清单建立"
```

---

### Task 5: demo 补齐（域 7 成员展示）

**Files:**
- Modify: `test/projects/learn-omniview/src/`（按 Task 3 审计结果）
- Modify（候选）: `test/projects/learn-omniview/src/components/ViewAttributesDemo.omni`（布尔属性）
- Modify（候选）: `test/projects/learn-omniview/src/components/EventShorthand.omni` 或新增滚动绑定处（scroll-x）

**Interfaces:**
- Consumes: Task 3 域 7 缺口清单（预计缺口：scroll-x、布尔属性）
- Produces: 域 7 全成员 demo 展示，`verify-demo-coverage.py` 缺口归零

**背景**（既有展示，勿重复添加）：@drag/@dragEnd/@ref/focus={sig}/scroll-y={sig}/style 动态绑定——ElementRefDemo.omni 已全部展示（Task 3 Step 3 会确认）。

- [ ] **Step 1: 按 Task 3 审计缺口逐个补展示**

**scroll-x 缺口**（预期）：在 ElementRefDemo.omni 滚动控制 section 补 scroll-x 绑定演示。在现有 `scrollTarget` 信号旁新增 `signal scrollTargetX: f64 = 0;`，滚动区 div 加 `scroll-x={scrollTargetX}`，按钮区加一行按钮（对齐既有 scrollToTop/scrollToBottom 模式）：

```omni
    // ── 滚动控制：响应式 scroll-x 绑定（水平滚动区） ──
    fn scrollXLeft() {
        scrollTargetX = 0;
        scrollPos = "scroll-x=0";
    }
    fn scrollX200() {
        scrollTargetX = 200;
        scrollPos = "scroll-x=200";
    }
```

view 滚动区（`<div scroll-y={scrollTarget} @ref={scrollArea} class: ScrollBox>` 之后）追加一个水平滚动演示区：

```omni
            <div scroll-x={scrollTargetX} class: ScrollBoxX>
                <p class: ScrollItemX>Item A — 水平滚动区（scroll-x 绑定）</p>
                <p class: ScrollItemX>Item B — 使用 scroll-x 绑定跳转</p>
                <p class: ScrollItemX>Item C — 水平滚动演示</p>
                <p class: ScrollItemX>Item D — scroll-x={scrollTargetX}</p>
                <p class: ScrollItemX>Item E — 水平滚动演示结束</p>
            </div>
            <div class: LearnDesign.Row>
                <button variant: LearnDesign.Button(color=ghost, size=sm) @onPress={scrollXLeft}>scroll-x=0</button>
                <button variant: LearnDesign.Button(color=accent, size=sm) @onPress={scrollX200}>scroll-x=200</button>
            </div>
```

style 块追加（配合水平滚动区——子元素 inline-block 以撑开宽度）：

```omni
        .ScrollBoxX { width: 100%; height: 64px; overflow-x: auto; overflow-y: hidden; background: var(--bgSecondary); border-radius: 8px; display: flex; gap: 8px; padding: 8px; }
        .ScrollBoxX p { flex: 0 0 auto; white-space: nowrap; font-size: var(--fontSizeXs); color: var(--textSecondary); margin: 0; }
```

> 若 `--bgSecondary`/`--fontSizeXs` 等 token 在 learn-omniview 不存在，改用文件内既有 token（ElementRefDemo 已有 .ScrollBox 使用 `var(--fontSizeXs)`——以其实际 token 为准）。

**布尔属性缺口**（预期）：ViewAttributesDemo.omni 追加裸布尔属性演示（对齐其 class:"string" 演示风格）：

```omni
            <div class: DemoCard>
                <h4 class: Label>裸布尔属性（三态：裸布尔 / {expr} / 缺省）</h4>
                <input type="checkbox" checked disabled={!isChecked} />
                <span>checked 裸布尔 · disabled={!isChecked} 表达式</span>
                <button disabled>disabled 裸布尔</button>
                <p class: info-text>裸布尔属性形态：<code>checked</code> 与 <code>disabled</code> 不带值即 true</p>
            </div>
```

> 具体内容以 Task 3 审计实际缺口为准；若布尔属性已有展示则跳过。

- [ ] **Step 2: WASM 编译门禁**

Run: `omnic build test/projects/learn-omniview/src/main.omni -t omniview`（或项目既有构建脚本；见 test/projects/learn-omniview 的 build 目录/脚本）
Expected: 编译成功，0 错误（learn-omniview 为多文件项目，若 CLI 不支持多文件则用项目既有 build.py 模式）

- [ ] **Step 3: 审计重跑确认缺口归零**

Run: `PYTHONIOENCODING=utf-8 python scripts/verify-demo-coverage.py`
Expected: `合计缺口（支持/部分未展示）: 0`（域 7 8 成员全部展示）

- [ ] **Step 4: 提交（含上次会话未提交的 ElementRefDemo/Overview 直写注释改动——保留并入）**

```bash
git add test/projects/learn-omniview/src/
git commit -m "feat(demo): learn-omniview 域 7 展示补齐——scroll-x 语义绑定 + 裸布尔属性 + 直写通道注释"
```

---

### Task 6: 门禁全绿与交付

**Files:**
- Create: 交付汇报（无文件，会话输出）

**Interfaces:**
- Consumes: Task 1-5 全部产物

- [ ] **Step 1: 全部门禁回归**

```bash
PYTHONIOENCODING=utf-8 python scripts/verify-semantic-universe.py --check   # 0 problem
PYTHONIOENCODING=utf-8 python scripts/verify-demo-coverage.py               # 缺口 0
PYTHONIOENCODING=utf-8 python scripts/verify-vocabulary.py --check          # 0 problem
PYTHONIOENCODING=utf-8 python scripts/verify-syntax-lifecycle.py --check    # 0 problem
PYTHONIOENCODING=utf-8 python scripts/verify-docs.py --check                # 0 problem（NOTE 可接受）
```

- [ ] **Step 2: 核对差异清单（挖掘产出）**

汇总核对结论：6 域脚本断言项（无需人工复核）+ 域 7 挖掘项（8 成员全部登记）+ 文档核对项（Task 4 清单）→ 确认无遗漏。

- [ ] **Step 3: 检查 bug 清单完整性**

确认 `tasks/omniview-syntax-audit-bugs.md` 记录了全部"文档宣称 ≠ 编译器行为"差异；若为零则文件保留（表头 + 说明，无 B 行）。

- [ ] **Step 4: 交付汇报**

汇报内容：差异清单、门禁结果、bug 清单位置、提交列表（3-4 个 commit）、遗留项（若有）。

---

## Self-Review 记录

（实现者填：spec 覆盖核对 / 占位符扫描 / 类型一致性——执行前自查）
