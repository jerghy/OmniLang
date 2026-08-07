# OmniView 遗留问题收口 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 收口三方对齐交付后的遗留问题——B2 编译器 bug（open 静默丢弃）修复 + deferred minors 9 项全修 + 游离 SDD 报告清理。

**Architecture:** B2 修复在 OmniView codegen（`omnilang/src/codegen/omniview/view/element.rs`）：open 增补进 `HTML_BOOLEAN_ATTRS`（12→13）与已知属性白名单（走 `el.open = !!` DOM property 分支），未知属性动态绑定在 fallback 分支生成 warning（裸名才警告，data-*/aria-* 前缀保持静默降级）。minors 横跨脚本（verify-semantic-universe.py 4 处）、bug 清单（1 处表述）、demo（ElementRefDemo 2 处）。游离副本移入 ref 计划 workspace。

**Tech Stack:** Rust codegen（omnilang crate）、Python 验证脚本、.omni demo、omnic CLI。

## Global Constraints

- B2 修复方向（用户拍板）：**增补 open + warning**——治标（open 可用）与治本（白名单外裸名动态绑定不再零诊断）组合
- React Doctor 报告的 poiesis 9 问题（并行会话 codeflow 工作面）：**不纳入本计划**（用户拍板留给并行会话）
- deferred minors：**8 项全修**（用户拍板）——12 项 minors 中 3 项已随 0bf89224/058a647a 修复（Task 3 ①drag 减法/②负数防御、Task 4 ①B2 笔误）；Task 2 ④（计数不校验）与 Task 4 ③（15 关键字数字）为同一表述问题的两面，合并为 M-5
- warning 只针对**非 data-/aria- 前缀**的未知属性动态绑定（裸未知属性名如 `open`；`data-tag={count}` 用户自定义通道保持既有静默降级）
- 不新增语义集/登记册改动（minors 中的表述修正是文档措辞，非登记内容变更）
- 门禁全绿标准：5 脚本 0 problem + `cargo test -p omnilang` 全绿 + `omnic build` learn-omniview 0 错误
- 脚本用 UTF-8 运行：`PYTHONIOENCODING=utf-8 python scripts/<name>.py`

---

### Task 1: B2 修复——open 增补 + 未知属性动态绑定 warning

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/element.rs`（HTML_BOOLEAN_ATTRS 常量 + 白名单 match + fallback 分支）
- Modify: `omnilang/src/codegen/omniview/tests.rs` 或 `tests_regression/`（找既有属性绑定测试位置，加 3 用例）

**Interfaces:**
- Produces: `HTML_BOOLEAN_ATTRS` 13 项（含 open）；`<details open={sig}>` 生成 `el.open = !!` effect；未知裸属性动态绑定生成 warning——Task 4 门禁验证依赖

**背景**（B2 实证链，Task 4 审计产出）：
- `open` 不在已知属性 match 分支（~70 项）也不在 `HTML_BOOLEAN_ATTRS`（12 项）→ 落入 `_ =>` fallback → `expr_to_static_str` 静态发射 `data-open="{isOpen}"` → 零 `__omni_effect`、零诊断 → `<details>` 运行时恒关闭（HTML 中 `data-open` 无意义）；React 目标正确透传
- 布尔属性必须走 DOM property（`el.checked = bool`）而非 setAttribute（`setAttribute("open", "false")` 中 "false" truthy 无法取消）

- [ ] **Step 1: 读目标文件确认既有结构**

Read `omnilang/src/codegen/omniview/view/element.rs`（重点：`HTML_BOOLEAN_ATTRS` 常量 ~:18、白名单 match ~:128-159、`_ =>` fallback ~:199-225）与 `tests.rs` 属性绑定测试模式（找 `data-nid` 或 `__omni_effect` 断言用例作模板）。

- [ ] **Step 2: HTML_BOOLEAN_ATTRS 增补 open（12→13）**

```rust
const HTML_BOOLEAN_ATTRS: &[&str] = &[
    "disabled",
    "checked",
    "required",
    "readonly",
    "selected",
    "autofocus",
    "multiple",
    "controls",
    "loop",
    "muted",
    "async",
    "defer",
    "open", // 2026-08-07 B2: <details> 布尔属性——此前落入 data-* fallback 静默丢弃
];
```

- [ ] **Step 3: 白名单 match 增补 "open"**

在既有 match 分支（`"id" | "placeholder" | ...`）合适位置加 `| "open"`（保持字母序大致：`"muted" | "open" | "poster"` 形态）。

- [ ] **Step 4: fallback 分支动态值 warning**

`_ =>` 分支开头（`let val_str = self.expr_to_static_str(value);` 之前）插入：

```rust
// Unknown attribute. Static values degrade to data-* passthrough
// (harmless — the emitted attribute name is data-<name>, not <name>).
// DYNAMIC bindings on a bare unknown name lose reactivity silently
// (B2: <details open={sig}> emitted static data-open, no __omni_effect);
// data-*/aria-* prefixed names are the user's own passthrough channel
// and keep the existing silent degradation.
if !matches!(value, ExprIR::LiteralExpr { .. })
    && !name.starts_with("data-")
    && !name.starts_with("aria-")
{
    self.warnings.push(format!(
        "unknown attribute '{}' with dynamic binding: OmniView emits it as static data-{} (not reactive); add the attribute to the known-attribute whitelist or use a known attribute",
        name, name
    ));
}
```

- [ ] **Step 5: 写失败测试（3 用例）**

在既有属性绑定测试区（tests.rs 或 tests_regression/，找 `open`/`data-` 相关测试的邻居位置）加：

```rust
#[test]
fn details_open_dynamic_binding_emits_el_open_property() {
    // <details open={isOpen}> → el.open = !!(...), no data-open attribute
    // （以既有 __omni_effect 断言模式书写，参考 data-nid/el.checked 用例）
}

#[test]
fn unknown_bare_attr_dynamic_binding_warns() {
    // <div foo={bar}> → warning "unknown attribute 'foo' with dynamic
    // binding" + data-foo 静态降级
}

#[test]
fn data_prefixed_attr_dynamic_binding_stays_silent() {
    // <div data-tag={count}> → 无 warning（用户自定义通道）
}
```

> 具体断言文本以既有测试模式为准（先读 tests.rs 找模板再写）；行为断言不得弱化：用例 1 断言产物含 `el.open` 且不含 `data-open`，用例 2 断言 warnings 非空。

- [ ] **Step 6: 运行测试验证 RED**

Run: `cargo test -p omnilang --lib codegen::omniview`（或既有测试模块路径——先 `cargo test -p omnilang --lib 2>&1 | grep -i "omniview"` 找模块名）
Expected: 新 3 用例 FAIL（open 仍走 fallback / 无 warning）

- [ ] **Step 7: 运行测试验证 GREEN + 全量回归**

Run: `cargo test -p omnilang --lib`
Expected: 全绿（既有断言若有对 HTML_BOOLEAN_ATTRS 集合的硬编码计数/集合测试——检查并同步为 13 项）

- [ ] **Step 8: 编译实证（替换旧二进制后再验）**

Run: `cargo build --release -p omnilang`（或项目既有 omnic 构建方式——见 memory `project-build.md` 构建约定：--release + PATH），然后用新二进制：
1. 临时文件 `C:\Users\XHT\.claude\jobs\cc104961\tmp\open-attr-test.omni`（内容：`import omniview; component OpenAttrTest { signal isOpen: Bool = false; view { <details open={isOpen}><summary>t</summary><p>body</p></details> } }`）→ `omnic build -t omniview` → 产物 grep `el.open` 存在且 `data-open` 不存在
2. 临时文件加 `<div foo={count}>`（`signal count: i32 = 0;`）→ build → 输出含 warning `unknown attribute 'foo'`

- [ ] **Step 9: Commit**

```bash
git add omnilang/src/codegen/omniview/view/element.rs omnilang/src/codegen/omniview/tests.rs
git commit -m "fix(codegen): OmniView open 布尔属性增补（B2——data-* fallback 静默丢弃）+ 未知裸属性动态绑定 warning（零诊断→诊断）"
```

> 若 tests_regression/ 有涉及 open/布尔属性的既有断言受影响，一并同步（同 commit）。

---

### Task 2: deferred minors 收口——脚本侧（verify-semantic-universe.py 4 项）

**Files:**
- Modify: `scripts/verify-semantic-universe.py`

**Interfaces:**
- Consumes: 无（独立）——Task 4 门禁验证依赖
- Produces: 脚本打磨后门禁仍 0 problem

**背景**：9 项 minors 中 4 项在脚本（ledger Task 2 minor ①②③⑤ + 表述类）。Task 3 ①/② 已随 0bf89224 修复。

- [ ] **Step 1: M-1 注释过期（:91）**

`# Domain titles must contain the expected keyword (in order 1..6).` → `in order 1..7`

- [ ] **Step 2: M-2 机制级存在性断言消息误导**

现状：条目状态检查与存在性检查解耦——「style 动态绑定/布尔属性」被标「未支持」时存在性检查仍执行并报「declared 支持 but...」。修复：存在性检查仅对状态 ∈ {支持, 部分} 的条目执行（未支持条目跳过存在性检查——未支持本身就是诚实契约）：

```python
        for name in ("style 动态绑定", "布尔属性"):
            if name not in mech:
                fail(f"directives: missing mechanism entry '{name}'")
                continue
            if mech[name] not in ("支持", "部分"):
                continue  # 未支持/部分外状态——存在性检查不适用（未支持即诚实契约）
        if mech.get("style 动态绑定") in ("支持", "部分"):
            ...existing existence checks...
        if mech.get("布尔属性") in ("支持", "部分"):
            ...existing boolean keyword check...
```

> 具体改法以实现者判断——语义要求：未支持状态的机制级条目不得触发「declared 支持」误导消息。

- [ ] **Step 3: M-3 boolean_attr_keywords 重复调用**

`boolean_attr_keywords(element_src)` 在条件与 fail 参数各调用一次 → 缓存变量（`keywords = boolean_attr_keywords(element_src)` 后用 `keywords`）。

- [ ] **Step 4: M-4 drag_words is None 死检查移除**

`lex_words is None` 分支不可达（drag_binding_words 永不返回 None；bind_words 的 None 检查已生效）——移除死分支或重构为 `if bind_words is None: fail(...)` + 拼接（保持提取失败显式 fail 语义，不静默放行）。

- [ ] **Step 5: M-5 登记册「15 关键字」表述对齐（同 commit 或独立小 commit）**

`docs/superpowers/specs/semantic-universe.md` 域 7 语义集条目「布尔属性」行诊断行为列：`parser 15 关键字 = peek_is_boolean_attr_keyword` 改为 `parser 布尔关键字集合 = peek_is_boolean_attr_keyword（当前 15 项——实现细节，以代码为准）`——数字作为当前事实而非契约（锁实现细节会反向漂移）。

- [ ] **Step 6: 门禁验证**

Run: `PYTHONIOENCODING=utf-8 python scripts/verify-semantic-universe.py --check`
Expected: 0 problem

- [ ] **Step 7: Commit**

```bash
git add scripts/verify-semantic-universe.py docs/superpowers/specs/semantic-universe.md
git commit -m "chore(audit): verify-semantic-universe deferred minors 收口——注释 1..7/机制级存在性断言按状态门控/重复调用缓存/死检查移除/15 关键字表述对齐"
```

---

### Task 3: deferred minors 收口——demo 与 bug 清单侧（3 项）

**Files:**
- Modify: `test/projects/learn-omniview/src/pages/ElementRefDemo.omni`（盒宽对齐 + 回显）
- Modify: `tasks/omniview-syntax-audit-bugs.md`（data-* 表述）

**Interfaces:**
- Consumes: 无——Task 4 门禁验证依赖
- Produces: demo 微调后 `verify-demo-coverage.py` 缺口仍 0 + omnic build 0 错误

- [ ] **Step 1: M-7 文件头盒宽对齐（ElementRefDemo.omni）**

文件头 ╔═╗ 块：新增 scroll-x 行（:12）69 列 vs 参考 67、改动 style 行（:7）71 列——微调空格使其与参考行对齐（以 :10 scroll-y 行为基准对齐列宽；既有行 ±1~5 漂移不动，只对齐本计划改动的两行）。

- [ ] **Step 2: M-8 scroll-x 按钮行追加状态回显**

scroll-x 按钮行（scrollXLeft/scrollX200 按钮区）追加状态 span（scrollPos 回显在 scroll-y 行——scroll-x 行自带显示）：

```omni
            <div class: LearnDesign.Row>
                <button variant: LearnDesign.Button(color=ghost, size=sm) @onPress={scrollXLeft}>scroll-x=0</button>
                <button variant: LearnDesign.Button(color=accent, size=sm) @onPress={scrollX200}>scroll-x=200</button>
                <span class: StatusText>X: {scrollTargetX}</span>
            </div>
```

> `{scrollTargetX}` 为 f64 插值——确认文件内既有 f64 插值写法（如 `{boxWidthPx}` 是 computed String；f64 直接插值 `{boxWidth}` 已有先例——看文件内实际用法对齐）。

- [ ] **Step 3: M-6 bug 清单「无 data-* 解析器」表述修正**

`tasks/omniview-syntax-audit-bugs.md` B2 行：`生成的运行时无 data-* 解析器（无 setAttribute/dataset 消费）` → `生成的运行时无通用 data-* 绑定解析器（setAttribute/dataset 零消费；data-for-key 为 @for 克隆专用）`

- [ ] **Step 4: 验证**

Run: `PYTHONIOENCODING=utf-8 python scripts/verify-demo-coverage.py`（缺口仍 0）+ `omnic build test/projects/learn-omniview/src/main.omni -t omniview -o <临时目录>`（0 错误）

- [ ] **Step 5: Commit**

```bash
git add test/projects/learn-omniview/src/pages/ElementRefDemo.omni tasks/omniview-syntax-audit-bugs.md
git commit -m "chore(demo): deferred minors 收口——文件头盒宽对齐 + scroll-x 状态回显 + bug 清单 data-* 表述修正"
```

---

### Task 4: 游离副本清理 + 门禁全绿交付

**Files:**
- Move: `tasks/task-5-report.md` → `.superpowers/sdd/2026-08-07-ref-high-frequency-direct-channel/task-5-report.md`
- Verify: 全部门禁 + cargo test

**Interfaces:**
- Consumes: Task 1-3 全部产物

- [ ] **Step 1: 核对游离副本与 ref workspace 既有报告**

先看 `.superpowers/sdd/2026-08-07-ref-high-frequency-direct-channel/task-5-report.md` 是否存在——若存在且内容相同则删除游离副本；若不同则对比后保留较完整者（游离副本来自上次会话复制，ref workspace 为原始）。以「tasks/ 目录只留跟踪交付物（bug 清单）」为终态。

- [ ] **Step 2: 移动/删除游离副本**

```bash
# 若 ref workspace 无同名文件：
mv tasks/task-5-report.md .superpowers/sdd/2026-08-07-ref-high-frequency-direct-channel/task-5-report.md
# 若已有：rm tasks/task-5-report.md
git status   # tasks/ 不再未跟踪（bug 清单已跟踪）
```

- [ ] **Step 3: 全部门禁回归**

```bash
PYTHONIOENCODING=utf-8 python scripts/verify-semantic-universe.py --check   # 0 problem
PYTHONIOENCODING=utf-8 python scripts/verify-demo-coverage.py               # 缺口 0
PYTHONIOENCODING=utf-8 python scripts/verify-vocabulary.py --check          # 0 problem
PYTHONIOENCODING=utf-8 python scripts/verify-syntax-lifecycle.py --check    # 0 problem
PYTHONIOENCODING=utf-8 python scripts/verify-docs.py --check                # 0 problem
cargo test -p omnilang --lib                                               # 全绿（B2 改动回归）
```

- [ ] **Step 4: 交付汇报**

汇报：B2 修复实证（el.open 产物 + warning 输出）、9 项 minors 处置表、游离副本清理结果、门禁汇总、提交列表、剩余遗留（若有）。

---

## Self-Review 记录

（实现者填：spec 覆盖核对 / 占位符扫描 / 类型一致性——执行前自查）
