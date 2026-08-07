# ref 高频操控——三目标直写通道 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 `@ref` + 信号 + `style: { expr }` 绑定的高频操控（拖拽等）在 React/Flutter 目标兑现与 OmniView 同级的高性能语义（ADR-071 编译器责任模式），分三阶段：React 直写通道 → @drag 意图化 → Flutter 表达式化。

**Architecture:** 语义契约（ADR-071 D2「style 属性恒等于表达式」）已定；本计划实现各目标编译器选路。Phase 1 在 React codegen 增加**信号域分类 pass**（D1 直写域/D2 写专用/B2 共享域），D1/D2 信号从 `useState` 改 `useRef` 存储 + mutation 点注入直写 apply，B2 走 `useEffect` 直写，style prop 剔除激活条目。Phase 2 把 `capturePointer` 意图化为 `@drag`（parser/AST/IR/三目标 codegen + demo 改造 + D3 演进）。Phase 3 把分类 pass 提为共享，Flutter 信号改 `ValueNotifier` + `ListenableBuilder` 包裹 + 属性映射表。

**Tech Stack:** Rust（omnilang codegen + omnilang-core parser/IR）、TypeScript/React 19、Dart/Flutter、bash 门禁（verify-react.sh）、cargo test 套件。

**设计文档:** `docs/design/ref-high-frequency-direct-channel.md`（语义契约、域分类、映射表、排期）

**验证基线（现状快照，2026-08-07）:** react 测试套件全绿（196/196）、compilation_test 7/7、verify-react GATE PASSED（tsc 72 errors 全豁免）、learn-omniview 生成产物 `ElementRefDemo.tsx` 中 `dragX/dragY/dragOriginX/dragOriginY` 为 `useState`（行 40-43）、`style={{ transform: ... }}` 在行 868、`dragBoxRef` 行 53。

---

## Task 0: 基线快照（实施前确认起点绿）

**Files:**
- Test: 无（运行既有门禁）

- [ ] **Step 1: 确认 React 测试套件基线**

Run: `cd D:/space/OmniLang && cargo test --lib codegen::react 2>&1 | tail -5`
Expected: 全部 PASS（react/tests.rs 内联单测）。

- [ ] **Step 2: 确认编译测试与门禁基线**

Run: `cargo test --test compilation_test && bash scripts/verify-react.sh`
Expected: compilation_test 7/7 PASS；verify-react 输出 `GATE PASSED`。

- [ ] **Step 3: 记录产物基线（审计锚点）**

Run: `grep -n "useState<number>\|dragBoxRef\|style={{" test/projects/learn-omniview/build/react-test-app/pages/ElementRefDemo.tsx | head -20`
Expected: 行 40-43 四个 drag useState；行 53 `dragBoxRef`；行 868 `style={{ transform: ... }}`。记录行号（后续 Task 5 断言对比用）。

---

## Phase 1: React 直写通道

### Task 1: 信号域分类 pass（D1/D2/B2）

**Files:**
- Create: `omnilang/src/codegen/react/component/direct_write.rs`
- Modify: `omnilang/src/codegen/react/component/mod.rs`（mod 声明）
- Test: `omnilang/src/codegen/react/tests.rs`（末尾追加）

- [ ] **Step 1: 写失败测试（分类 pass）**

在 `omnilang/src/codegen/react/tests.rs` 末尾追加（复用既有 `make_ir`，另加源码编译 helper）：

```rust
// ── Direct-write channel (Phase 1) ──────────────────────

/// Compile a single .omni source through the pipeline and return its
/// first component IR. Panics on pipeline errors — tests are contract.
fn compile_component_ir(source: &str) -> ComponentIR {
    let out = omnilang_core::run_pipeline(source);
    assert!(
        !out.has_errors,
        "pipeline errors: {:?}",
        out.diagnostics
    );
    let mut comps = out.ir.modules.get("main").unwrap().components.values();
    comps.next().unwrap().clone()
}

use super::component::direct_write::{classify_direct_write, DirectWriteSets};

#[test]
fn test_classify_direct_write_drag_demo() {
    // 拖拽三态：dragX/dragY 仅 style 消费（D1）；dragOrigin 仅 mutation RHS
    // 自读（D2）；dragging 有 class 绑定（B2 → 不在 D1/D2）。
    let comp = compile_component_ir(
        r#"
component Drag {
  signal dragging: Bool = false
  signal dragX: f64 = 0
  signal dragY: f64 = 0
  signal dragOriginX: f64 = 0
  signal dragOriginY: f64 = 0
  ref box: Element
  fn start(e) { dragging = true; dragOriginX = e.x; dragOriginY = e.y }
  fn move(e) { if dragging { dragX = e.x - dragOriginX; dragY = e.y - dragOriginY } }
  fn end(e) { dragging = false }
  view {
    <div @ref={box} @onPointerDown={start} @onPointerMove={move} @onPointerUp={end}
         style: { transform: "translate(" + dragX + "px, " + dragY + "px)" }
         class: { Active: dragging } />
  }
}
"#,
    );
    let sets = classify_direct_write(&comp);
    assert!(sets.d1.contains("dragX"), "dragX should be D1: {:?}", sets.d1);
    assert!(sets.d1.contains("dragY"), "dragY should be D1: {:?}", sets.d1);
    assert!(sets.d2.contains("dragOriginX"), "dragOriginX should be D2: {:?}", sets.d2);
    assert!(sets.d2.contains("dragOriginY"), "dragOriginY should be D2: {:?}", sets.d2);
    assert!(!sets.d1.contains("dragging"), "dragging must NOT be D1");
    assert!(!sets.d2.contains("dragging"), "dragging must NOT be D2");
    assert_eq!(sets.entries.len(), 1, "one active entry (transform)");
    assert_eq!(sets.entries[0].ref_name, "box");
    assert_eq!(sets.entries[0].prop, "transform");
}

#[test]
fn test_classify_direct_write_no_ref_is_inactive() {
    // 无 @ref → 不激活（激活条件 1）。
    let comp = compile_component_ir(
        r#"
component NoRef {
  signal x: f64 = 0
  view { <div style: { transform: "translate(" + x + "px, 0px)" } /> }
}
"#,
    );
    let sets = classify_direct_write(&comp);
    assert!(sets.entries.is_empty());
    assert!(sets.d1.is_empty() && sets.d2.is_empty());
}

#[test]
fn test_classify_direct_write_computed_stays_b2() {
    // computed 信号永远不进 D1/D2（保持 useMemo 渲染路径）。
    let comp = compile_component_ir(
        r#"
component SizeBox {
  signal w: f64 = 200
  computed wPx: String = w.to_string() + "px"
  ref sb: Element
  view { <div @ref={sb} style: { width: wPx } /> }
}
"#,
    );
    let sets = classify_direct_write(&comp);
    assert!(sets.d1.is_empty(), "computed must not be D1: {:?}", sets.d1);
    assert_eq!(sets.b2_entries.len(), 1, "computed-driven entry is B2");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --lib codegen::react::tests::test_classify_direct_write 2>&1 | tail -8`
Expected: `error[E0432]: unresolved import ... direct_write`（模块不存在）。

- [ ] **Step 3: 实现分类 pass**

Create `omnilang/src/codegen/react/component/direct_write.rs`：

```rust
//! Direct-write channel classification (ADR-071 decision 2, Phase 1).
//!
//! Classifies component signals consumed by dynamic `style: { expr }`
//! bindings on `@ref`'d elements (the "property always equals expr"
//! semantic) into storage domains:
//!
//! * D1 (direct-write): signal's only output reads are inside active
//!   entries → stored in `useRef`, mutations inject a direct DOM write.
//! * D2 (write-only): signal has no output reads (only mutation-RHS
//!   self-reads) → stored in `useRef`, no apply needed.
//! * B2 (shared): signal has render-path reads (text/class/prop/...)
//!   → stays `useState` + `useEffect([deps])` direct write.
//!
//! Computed signals and store fields NEVER enter D1/D2 (they must keep
//! their render-path derivation); entries reading them are B2.

use rustc_hash::FxHashSet;
use serde_json::Value;
use crate::ir::{ComponentIR, ViewTree};

/// One activated style entry: `{ref}Ref.current.style.{prop} = value`.
#[derive(Debug, Clone)]
pub(super) struct DirectWriteEntry {
    pub ref_name: String,
    pub prop: String,
    pub value: Value,
    pub read_signals: FxHashSet<String>,
}

/// Classification result consumed by the emission passes.
#[derive(Debug, Default)]
pub(super) struct DirectWriteSets {
    /// D1 signals: useRef storage + mutation-site apply injection.
    pub d1: FxHashSet<String>,
    /// D2 signals: useRef storage, no apply.
    pub d2: FxHashSet<String>,
    /// Activated (D1) entries — order follows view traversal.
    pub entries: Vec<DirectWriteEntry>,
    /// B2 entries: useEffect([deps]) direct writes.
    pub b2_entries: Vec<DirectWriteEntry>,
}

/// True when a style value is dynamic (contains $signal/$ident/$field/
/// $call/$concat meta-objects) — mirrors the shared/style.rs dispatch.
fn is_dynamic_style_value(val: &Value) -> bool {
    match val {
        Value::Object(obj) => {
            obj.contains_key("$signal")
                || obj.contains_key("$ident")
                || obj.contains_key("$field")
                || obj.contains_key("$call")
                || obj.contains_key("$concat")
        }
        _ => false,
    }
}

/// Collect every identifier name referenced by a style value JSON tree
/// ($signal / $ident leaf nodes).
fn collect_value_reads(val: &Value, acc: &mut FxHashSet<String>) {
    match val {
        Value::Object(obj) => {
            if let Some(sig) = obj.get("$signal").and_then(|v| v.as_str()) {
                acc.insert(sig.to_string());
                return;
            }
            if let Some(ident) = obj.get("$ident").and_then(|v| v.as_str()) {
                acc.insert(ident.to_string());
                return;
            }
            for v in obj.values() {
                collect_value_reads(v, acc);
            }
        }
        Value::Array(items) => {
            for v in items {
                collect_value_reads(v, acc);
            }
        }
        _ => {}
    }
}

/// Walk the view tree collecting elements (ref binding + dynamic style
/// entries). `in_for` guards activation condition 3 (@for + ref inactive).
fn collect_active_entries(node: &ViewTree, in_for: bool, out: &mut Vec<DirectWriteEntry>) {
    match node {
        ViewTree::Element(el) => {
            let nested_for = in_for || el.for_binding.is_some();
            if let Some(ref_name) = &el.ref_binding {
                if let Some(obj) = el
                    .inline_style
                    .as_ref()
                    .and_then(|v| v.as_object())
                {
                    for (prop, val) in obj {
                        if !nested_for && is_dynamic_style_value(val) {
                            let mut reads = FxHashSet::default();
                            collect_value_reads(val, &mut reads);
                            out.push(DirectWriteEntry {
                                ref_name: ref_name.clone(),
                                prop: prop.clone(),
                                value: val.clone(),
                                read_signals: reads,
                            });
                        }
                    }
                }
            }
            for child in &el.children {
                collect_active_entries(child, nested_for, out);
            }
        }
        ViewTree::Conditional(cond) => {
            for b in &cond.branches {
                collect_active_entries(&b.body, in_for, out);
            }
        }
        ViewTree::List(list) => {
            collect_active_entries(&list.body, true, out);
        }
        _ => {}
    }
}

/// Output reads of a signal OUTSIDE active entry values — the set of
/// identifiers referenced by render output (text interpolation, class
/// bindings, props, non-active style entries) plus view handler bodies
/// and called-fn bodies, EXCLUDING mutation-RHS self-reads.
///
/// Reuses the existing collectors (collect_view_read_signals / the
/// read-side of collect_signal_mutations); the active-entry reads are
/// subtracted afterwards.
fn collect_output_reads(comp: &ComponentIR, active_entries: &[DirectWriteEntry]) -> FxHashSet<String> {
    use super::super::shared_collectors::*;
    let mut reads = FxHashSet::default();
    if let Some(view) = &comp.view {
        collect_view_read_signals(view, &mut reads);
        collect_view_inline_handler_reads(view, &mut reads);
    }
    // Called-fn bodies (excluding mutation RHS — covered by the existing
    // collector's read side already; mutation RHS reads are subtracted via
    // the entry-value subtraction below only for ACTIVE entries, so fn
    // mutation RHS reads still land here: acceptable conservative
    // over-approximation — see Task 1 note in the design doc §4.2).
    for entry in active_entries {
        for s in &entry.read_signals {
            reads.remove(s);
        }
    }
    reads
}

/// Classify component signals into D1 / D2; everything else stays on the
/// existing useState path. Only `comp.signals` (not computeds) qualify.
pub(super) fn classify_direct_write(comp: &ComponentIR) -> DirectWriteSets {
    let mut sets = DirectWriteSets::default();
    if let Some(view) = &comp.view {
        collect_active_entries(view, false, &mut sets.entries);
    }
    if sets.entries.is_empty() {
        return sets;
    }

    // Signal names appearing in any active entry value.
    let mut entry_consumed: FxHashSet<String> = FxHashSet::default();
    for e in &sets.entries {
        entry_consumed.extend(e.read_signals.iter().cloned());
    }

    // Output reads outside active entries (view + handlers + fns).
    let output_reads = collect_output_reads(comp, &sets.entries);

    let signal_names: FxHashSet<String> =
        comp.signals.iter().map(|s| s.name.clone()).collect();

    for sig in &comp.signals {
        if !signal_names.contains(&sig.name) {
            continue;
        }
        let is_mutated = comp_mutated_signals(comp).contains(&sig.name);
        if !is_mutated {
            continue;
        }
        if entry_consumed.contains(&sig.name) && !output_reads.contains(&sig.name) {
            sets.d1.insert(sig.name.clone());
        } else if !entry_consumed.contains(&sig.name) && !output_reads.contains(&sig.name) {
            sets.d2.insert(sig.name.clone());
        }
    }

    // B2 entries: any entry reading at least one non-D1 signal.
    for e in sets.entries.iter() {
        let all_d1 = e
            .read_signals
            .iter()
            .all(|s| sets.d1.contains(s) || !comp.signals.iter().any(|cs| &cs.name == s));
        if !all_d1 {
            sets.b2_entries.push(e.clone());
        }
    }
    sets
}

/// Mutated component signal names (delegates to the existing collector).
fn comp_mutated_signals(comp: &ComponentIR) -> FxHashSet<String> {
    // collect_signal_mutations lives in state.rs and needs called_fns —
    // reuse the same wiring as mod.rs does for mutated_signals.
    let called = super::function::collect_called_local_fns(comp);
    super::state::collect_signal_mutations(comp, &called)
}
```

（注：`shared_collectors` 为下面 Step 5 的接线别名；`ViewTree::Fragment`/`List`/`Conditional`/`Element` 的字段名以 `crates/omnilang-core/src/ir/types.rs` 为准——若 `for_binding` 字段名不同，按实际更正，测试不变。）

Modify `omnilang/src/codegen/react/component/mod.rs`（模块声明区）：

```rust
pub(crate) mod direct_write;
```

（`pub(crate)` 而非私有 `mod`——`react/tests.rs`（`react` 的兄弟子模块）需要访问 `classify_direct_write`/`DirectWriteSets` 做产物断言。）

`direct_write.rs` 内对外 API 可见性同步：`pub(super) struct` → `pub(crate) struct`（`DirectWriteEntry`/`DirectWriteSets` 两个）、`pub(super) fn classify_direct_write` → `pub(crate) fn`。被 `mod.rs` 消费的 `gen_direct_write_storage`/`build_apply_map`/`gen_ref_style_effects` 保持 `pub(super)` 即可。

- [ ] **Step 4: 接线 shared_collectors 并让测试通过**

`direct_write.rs` 中 `collect_output_reads` 引用的 `super::super::shared_collectors` 不存在——改为直接引用既有函数（它们已 `pub(super)` 于 state.rs）。把该 use 行替换为：

```rust
use super::state::{
    collect_view_inline_handler_reads, collect_view_read_signals,
};
```

（`collect_view_inline_handler_reads` 定义于 mod.rs 而非 state.rs——若如此，改为 `use super::super::{collect_view_inline_handler_reads};`。以 grep 实际位置为准：`grep -rn "fn collect_view_inline_handler_reads" omnilang/src/codegen/react/`。）

Run: `cargo test --lib codegen::react::tests::test_classify_direct_write 2>&1 | tail -8`
Expected: 3 个测试全 PASS。若 `ViewTree`/`ElementIR` 字段名编译报错，按编译器提示修正字段访问（测试断言不变——测试是契约）。

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/component/direct_write.rs omnilang/src/codegen/react/component/mod.rs omnilang/src/codegen/react/tests.rs
git commit -m "feat(codegen): 信号域分类 pass（D1 直写域/D2 写专用/B2 共享）——React 直写通道分析基建"
```

### Task 2: D1/D2 存储发射（useRef 替代 useState）

**Files:**
- Modify: `omnilang/src/codegen/react/component/direct_write.rs`（新增发射函数）
- Modify: `omnilang/src/codegen/react/component/mod.rs`（hook 判定 + gen_state_hooks 过滤 + 调用点）
- Modify: `omnilang/src/codegen/react/component/state.rs`（gen_state_hooks 签名/过滤）
- Test: `omnilang/src/codegen/react/tests.rs`

- [ ] **Step 1: 写失败测试（产物断言）**

在 `omnilang/src/codegen/react/tests.rs` 追加：

```rust
#[test]
fn test_direct_write_storage_emits_use_ref() {
    let comp = compile_component_ir(
        r#"
component Drag {
  signal dragX: f64 = 0
  signal dragY: f64 = 0
  signal dragOriginX: f64 = 0
  signal dragging: Bool = false
  ref box: Element
  fn start(e) { dragging = true; dragOriginX = e.x }
  fn move(e) { dragX = e.x - dragOriginX; dragY = dragX }
  view {
    <div @ref={box} @onPointerDown={start} @onPointerMove={move}
         style: { transform: "translate(" + dragX + "px, " + dragY + "px)" }
         class: { Active: dragging } />
  }
}
"#,
    );
    let ir = make_ir(vec![("Drag".into(), comp)].into_iter().collect());
    let comp_ref = ir.modules.get("main").unwrap().components.get("Drag").unwrap();
    let component_map = build_component_map(&ir);
    let tsx = ReactCodegen::default().gen_component_tsx("Drag", comp_ref, &ir, &component_map);
    assert!(
        tsx.contains("const dragXRef = useRef<number>(0);"),
        "D1 storage must be useRef. Got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("const dragOriginXRef = useRef<number>(0);"),
        "D2 storage must be useRef. Got:\n{}",
        tsx
    );
    assert!(
        !tsx.contains("useState<number>(0)") || !tsx.contains("dragX"),
        "dragX must NOT be a useState. Got:\n{}",
        tsx
    );
    // dragging 有 class 消费 → 仍 useState
    assert!(
        tsx.contains("const [dragging, setDragging] = useState<boolean>(false);"),
        "dragging stays useState. Got:\n{}",
        tsx
    );
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --lib codegen::react::tests::test_direct_write_storage_emits_use_ref 2>&1 | tail -8`
Expected: FAIL——tsx 仍含 `const [dragX, setDragX] = useState<number>(0);`。

- [ ] **Step 3: 实现存储发射**

在 `direct_write.rs` 追加：

```rust
/// Emit `const nameRef = useRef<T>(initial);` for D1/D2 signals.
pub(super) fn gen_direct_write_storage(
    w: &mut crate::codegen::CodeWriter,
    comp: &ComponentIR,
    sets: &DirectWriteSets,
) {
    let direct: Vec<&str> = sets.d1.iter().chain(sets.d2.iter()).cloned().collect();
    let mut direct_sorted: Vec<&str> = direct;
    direct_sorted.sort();
    for name in direct_sorted {
        let sig = comp.signals.iter().find(|s| s.name == name).unwrap();
        let init = crate::codegen::react::expr::format_expr_ir_tsx(
            sig.initial_value.as_ref().unwrap_or(&crate::codegen::shared::zero_value()),
            &FxHashSet::default(),
        );
        w.emit_fmt(format_args!(
            "const {}Ref = useRef<{}>({});",
            name,
            crate::codegen::react::types::js_type_for(&sig.type_ann, comp),
            init
        ));
    }
    if !direct_sorted.is_empty() {
        w.emit("");
    }
}
```

（`js_type_for` 的实际函数名以 `react/types.rs` 为准；`initial_value` 为 None 时用 `null` 字面量 + `!` 断言不需要——保守用 `0` 不可行，因为类型不定。简化：None → 发射 `useRef<Type>(null as unknown as Type)`。实现时以 grep 到的现有 useState 初始值发射逻辑为准——`state.rs` 的 `gen_state_hooks` 里有同款翻译，直接提取复用。）

**关键联动修改**（`mod.rs`）：

1. `has_mutable_states`（行 172-175）扣除 D1/D2：

```rust
// v4 H2: 计入写专用信号（现会发射 useState）；排除 pruned 信号
// Phase 1: 再排除 D1/D2 直写域信号（useRef 存储，不走 useState）
let has_mutable_states = comp
    .signals
    .iter()
    .any(|s| {
        mutated_signals.contains(&s.name)
            && !pruned_signals.contains(&s.name)
            && !direct_write_sets.d1.contains(&s.name)
            && !direct_write_sets.d2.contains(&s.name)
    });
```

2. `gen_state_hooks` 调用处传入 `direct_write_sets`（d1 ∪ d2 集合），state.rs 的 gen_state_hooks 内跳过这些信号（在发射 useState 的判定处加 `&& !direct_write.contains(&state.name)`）。

3. `gen_ref_hooks`（行 802）之后调用存储发射：

```rust
self.gen_ref_hooks(&mut w, comp);
direct_write::gen_direct_write_storage(&mut w, comp, &direct_write_sets);
```

4. 计算 `direct_write_sets`：在 mod.rs 生成流程开头（pruned_signals 计算之后）调用 `classify_direct_write(comp)`，并作为参数传入需要的地方。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --lib codegen::react 2>&1 | tail -8`
Expected: 新测试 PASS，既有 react 测试无回归。

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/component/direct_write.rs omnilang/src/codegen/react/component/mod.rs omnilang/src/codegen/react/component/state.rs omnilang/src/codegen/react/tests.rs
git commit -m "feat(codegen): D1/D2 信号 useRef 存储发射 + useState 池联动（has_mutable_states/gen_state_hooks）"
```

### Task 3: mutation 发射 + apply 直写注入

**Files:**
- Modify: `omnilang/src/codegen/react/expr.rs`（ReactStmtFormatter 字段 + format_mutation_stmt + format_compound_mutation）
- Modify: `omnilang/src/codegen/react/component/direct_write.rs`（apply 语句构建）
- Test: `omnilang/src/codegen/react/tests.rs`

- [ ] **Step 1: 写失败测试（handler 产物断言）**

```rust
#[test]
fn test_direct_write_mutation_injects_apply() {
    let comp = compile_component_ir(
        r#"
component Drag {
  signal dragX: f64 = 0
  signal dragY: f64 = 0
  signal dragOriginX: f64 = 0
  ref box: Element
  fn move(e) { dragX = e.x - dragOriginX; dragY = e.x }
  view {
    <div @ref={box} @onPointerMove={move}
         style: { transform: "translate(" + dragX + "px, " + dragY + "px)" } />
  }
}
"#,
    );
    let ir = make_ir(vec![("Drag".into(), comp)].into_iter().collect());
    let comp_ref = ir.modules.get("main").unwrap().components.get("Drag").unwrap();
    let component_map = build_component_map(&ir);
    let tsx = ReactCodegen::default().gen_component_tsx("Drag", comp_ref, &ir, &component_map);
    assert!(
        tsx.contains("dragXRef.current = e.x - dragOriginXRef.current;"),
        "D1 mutation must write the ref. Got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("dragBoxRef.current?.style.transform ="),
        "apply direct write must be injected after D1 mutation. Got:\n{}",
        tsx
    );
    assert!(
        !tsx.contains("setDragX("),
        "no useState setter for D1 signals. Got:\n{}",
        tsx
    );
    // 值里读 D2 信号 → 必须解析为 Ref.current
    assert!(
        tsx.contains("dragOriginXRef.current"),
        "D2 signal reads resolve to Ref.current. Got:\n{}",
        tsx
    );
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --lib codegen::react::tests::test_direct_write_mutation_injects_apply 2>&1 | tail -8`
Expected: FAIL——产物含 `setDragX(...)`，无直写。

- [ ] **Step 3: 构建 apply 语句表**

在 `direct_write.rs` 追加（apply 值复用 `format_style_value_expr`，把 D1/D2 信号并入 ref_names 使 `$signal/$ident` 解析为 `{name}Ref.current`）：

```rust
use crate::codegen::shared::style::{format_style_prop_key, format_style_value_expr};

/// Build `signal → apply line` map. Each apply reads the CURRENT ref
/// values of all entry signals, so injecting after any mutation of any
/// entry signal keeps the DOM in sync (idempotent double-writes when a
/// handler mutates several signals are accepted — μs cost).
pub(super) fn build_apply_map(
    sets: &DirectWriteSets,
    ref_names: &FxHashSet<String>,
) -> FxHashMap<String, Vec<String>> {
    let mut refs = ref_names.clone();
    refs.extend(sets.d1.iter().cloned());
    refs.extend(sets.d2.iter().cloned());
    let mut map: FxHashMap<String, Vec<String>> = FxHashMap::default();
    for e in &sets.entries {
        let value_js = format_style_value_expr(&e.value, &refs);
        for sig in &e.read_signals {
            let line = format!(
                "{}Ref.current?.style.{} = {};",
                e.ref_name,
                format_style_prop_key(&e.prop),
                value_js
            );
            map.entry(sig.clone()).or_default().push(line);
        }
    }
    map
}
```

**CSS 自定义属性特例**：`format_style_prop_key` 对 `--*` 返回 `'--x'`（带引号），而 apply 是属性赋值——`el.style.'--x'` 非法。对 `--` 开头的 prop，apply 行改用 setProperty：

```rust
// 在 build_apply_map 中：
let line = if e.prop.starts_with("--") {
    format!(
        "{}Ref.current?.style.setProperty('{}', {});",
        e.ref_name, e.prop, value_js
    )
} else {
    format!(
        "{}Ref.current?.style.{} = {};",
        e.ref_name,
        format_style_prop_key(&e.prop),
        value_js
    )
};
```

**sanitize 决策**：apply 值含 `$concat`/`$call`/`$field` 时（动态拼接用户可控字符串），与 OmniView `__omni_sanitizeCss` 行为对齐，包裹 `sanitizeCss(...)`。`sanitizeCss` helper 由 `react/design.rs` 的 `emit_sanitize_css` 发射（模块级 helper）——grep 确认其可见性与调用点（`grep -n "emit_sanitize_css" omnilang/src/codegen/react/`），若为私有则提为 `pub(crate)`；当有需 sanitize 的 apply 时，在组件生成头部触发发射（与 design 的动态 token 触发同点）。动态值包裹实现（替换上面 value_js 的构造）：

```rust
let needs_sanitize = matches!(
    &e.value,
    Value::Object(obj)
        if obj.contains_key("$concat") || obj.contains_key("$call") || obj.contains_key("$field")
);
let value_js = if needs_sanitize {
    format!("sanitizeCss({})", format_style_value_expr(&e.value, &refs))
} else {
    format_style_value_expr(&e.value, &refs)
};
```

- [ ] **Step 4: 实现 mutation 发射分支**

`expr.rs`：`ReactStmtFormatter` 结构体增加字段（构造处 `generate_statement_tsx` 一并填充）：

```rust
/// D1/D2 signal names → `nameRef.current = ...` storage writes.
direct_write_signals: FxHashSet<String>,
/// signal → apply lines injected after its mutations.
direct_write_applies: FxHashMap<String, Vec<String>>,
```

`format_mutation_stmt` 的 `MutationIntent::DirectAssign` 分支，在 `else if self.signal_names.contains(...)` 之前插入：

```rust
} else if self.direct_write_signals.contains(target_str.as_str()) {
    let amount = value
        .as_ref()
        .map(|v| self.expr.format_expr(v))
        .unwrap_or_else(|| "0".into());
    w.emit_fmt(format_args!("{}Ref.current = {};", target_str, amount));
    if let Some(applies) = self.direct_write_applies.get(target_str.as_str()) {
        for line in applies {
            w.emit_fmt(format_args!("{}", line));
        }
    }
}
```

`format_compound_mutation`（Increment/Decrement 等宏）同样加 D1/D2 分支——在宏展开前拦截：

```rust
// direct_write 分支（compound 也写 ref + 注入 apply）
fn format_compound_mutation(&self, w: &mut CodeWriter, intent: &MutationIntent) {
    let target_str = compound_target_name(intent);
    if self.direct_write_signals.contains(target_str.as_str()) {
        // 目标形态: dragXRef.current += amt; + applies
        ...
    }
    // ... 原有宏路径
}
```

（compound 的 target 提取复用 `mutation_target_name` 既有 helper。）

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --lib codegen::react 2>&1 | tail -8`
Expected: 新测试 PASS，无回归。

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/react/expr.rs omnilang/src/codegen/react/component/direct_write.rs omnilang/src/codegen/react/tests.rs
git commit -m "feat(codegen): D1/D2 mutation 发射 useRef 写入 + apply 直写注入（含 -- 属性 setProperty 与 sanitize 特例）"
```

### Task 4: style prop 剔除 + B2 effect 直写

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs`（inline_style 发射过滤激活条目）
- Modify: `omnilang/src/codegen/react/component/direct_write.rs`（gen_ref_style_effects）
- Modify: `omnilang/src/codegen/react/component/mod.rs`（接线）
- Modify: `omnilang/src/codegen/react/component/state.rs`（emit_element_binding_effects 复用点）
- Test: `omnilang/src/codegen/react/tests.rs`

- [ ] **Step 1: 写失败测试（style prop 剔除 + B2 effect）**

```rust
#[test]
fn test_direct_write_style_prop_removed_and_b2_effect() {
    let comp = compile_component_ir(
        r#"
component Mix {
  signal dragX: f64 = 0
  signal boxWidthNum: f64 = 200
  computed boxWidthPx: String = boxWidthNum.to_string() + "px"
  ref box: Element
  ref sb: Element
  fn move(e) { dragX = e.x }
  view {
    <div @ref={box} @onPointerMove={move}
         style: { transform: "translate(" + dragX + "px, 0px)", width: "100px" } />
    <div @ref={sb} style: { width: boxWidthPx } />
    <span>{boxWidthNum}</span>
  }
}
"#,
    );
    let ir = make_ir(vec![("Mix".into(), comp)].into_iter().collect());
    let comp_ref = ir.modules.get("main").unwrap().components.get("Mix").unwrap();
    let component_map = build_component_map(&ir);
    let tsx = ReactCodegen::default().gen_component_tsx("Mix", comp_ref, &ir, &component_map);
    // 激活条目（transform）不进 style prop；静态条目（width）保留
    assert!(
        !tsx.contains("transform:"),
        "active entry must NOT appear in style prop. Got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("width: '100px'"),
        "static style entry stays in style prop. Got:\n{}",
        tsx
    );
    // B2：computed 驱动的条目 → useEffect 直写
    assert!(
        tsx.contains("sizeBoxRef.current?.style.width ="),
        "B2 entry emits effect direct write. Got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("useEffect(() =>"),
        "B2 write runs in useEffect. Got:\n{}",
        tsx
    );
    // D1 条目不进 effect（handler 注入已覆盖）
    assert!(
        !tsx.contains("useEffect(() => {") || !tsx.contains("dragBoxRef.current?.style.transform"),
        "D1 apply must NOT be in effect. Got:\n{}",
        tsx
    );
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --lib codegen::react::tests::test_direct_write_style_prop_removed_and_b2_effect 2>&1 | tail -8`
Expected: FAIL——transform 仍在 style prop，无 B2 effect。

- [ ] **Step 3: view.rs style prop 过滤**

`view.rs` 的 inline_style 发射（行 718-731 区域）：把激活条目（ref_name + prop 键集合）传入并跳过。函数签名增加参数（沿调用链从 `gen_view_node_tsx` 传入 `direct_write_keys: &FxHashSet<(String, String)>`——ref+prop 对）：

```rust
// Inline style
if let Some(ref style_json) = el.inline_style {
    if let Some(obj) = style_json.as_object() {
        let mut pairs: Vec<String> = Vec::new();
        for (prop, val) in obj {
            // Phase 1: 激活直写条目从 style prop 剔除（apply 在 mutation
            // 点/effect 直写，避免双路径覆盖竞态）
            if direct_write_keys.contains(&(el_ref.clone(), prop.clone())) {
                continue;
            }
            let js_prop = format_style_prop_key(prop);
            let js_val = format_style_value_expr(val, ref_names);
            pairs.push(format!("{}: {}", js_prop, js_val));
        }
        ...
    }
}
```

（`el_ref` 从 `el.ref_binding` 取；调用链：`mod.rs` 的 view 生成入口计算 `direct_write_keys` 传给 `view.rs`。）

- [ ] **Step 4: 实现 gen_ref_style_effects**

在 `direct_write.rs` 追加（复用 `state.rs:1162-1367 emit_element_binding_effects` 的 deps 计算模式）：

```rust
/// Emit `useEffect(() => { refRef.current?.style.prop = value; }, [deps])`
/// for B2 entries (state/computed/store-backed). Deps = entry read signals
/// that are NOT D1/D2 (D1/D2 are ref-stored, cannot be deps).
pub(super) fn gen_ref_style_effects(
    w: &mut crate::codegen::CodeWriter,
    comp: &ComponentIR,
    sets: &DirectWriteSets,
    ref_names: &FxHashSet<String>,
) {
    if sets.b2_entries.is_empty() {
        return;
    }
    let mut refs = ref_names.clone();
    refs.extend(sets.d1.iter().cloned());
    refs.extend(sets.d2.iter().cloned());
    for e in &sets.b2_entries {
        let value_js = format_style_value_expr(&e.value, &refs);
        let deps: Vec<String> = e
            .read_signals
            .iter()
            .filter(|s| !sets.d1.contains(*s) && !sets.d2.contains(*s))
            .cloned()
            .collect();
        w.emit_fmt(format_args!("useEffect(() => {{"));
        w.emit_fmt(format_args!(
            "  {}Ref.current?.style.{} = {};",
            e.ref_name,
            format_style_prop_key(&e.prop),
            value_js
        ));
        w.emit_fmt(format_args!("}}, [{}]);", deps.join(", ")));
        w.emit("");
    }
}
```

（store 字段读的 deps：若 entry 读 store 字段，`read_signals` 含 store 名——deps 需按既有 store slice 机制重命名。实现时复用 `emit_element_binding_effects` 的 deps 解析（`state.rs`），把本函数的 deps 计算替换为调用该既有逻辑；测试只断言形态，不锁实现细节。）

**接线**（`mod.rs`）：在 `gen_semantic_binding_effects`（行 867）之后：

```rust
direct_write::gen_ref_style_effects(&mut w, comp, &direct_write_sets, &ref_names);
```

同时确认 hook 选择已含 `useEffect`（B2 条目存在时若组件无既有 effect，需触发——`has_effects_or_lifecycles` 判定加 `|| !direct_write_sets.b2_entries.is_empty()`）。

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --lib codegen::react 2>&1 | tail -8`
Expected: 新测试 PASS，无回归。

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/react/view.rs omnilang/src/codegen/react/component/direct_write.rs omnilang/src/codegen/react/component/mod.rs omnilang/src/codegen/react/component/state.rs omnilang/src/codegen/react/tests.rs
git commit -m "feat(codegen): style prop 剔除激活条目 + B2 effect 直写（useEffect deps 复用语义绑定基建）"
```

### Task 5: 全量门禁 + 产物审计

**Files:**
- Test: `test/projects/learn-omniview/build/react-test-app/`（再生成）
- Test: `scripts/verify-react.sh`（运行）
- Test: `omnilang/tests/generated_code_validation.rs`（react 套件）

- [ ] **Step 1: 重新生成 learn-omniview React 产物**

Run:
```bash
cd test/projects/learn-omniview && ../../target/release/omnic build --target react -o build/react-test-app
```
（若 release 二进制过期：`cargo build --release --bin omnic` 先构建。）
Expected: 编译成功，无错误输出。

- [ ] **Step 2: 产物审计断言（4 项）**

Run:
```bash
f=build/react-test-app/pages/ElementRefDemo.tsx
grep -c "const \[dragX, setDragX\]\|const \[dragY, setDragY\]\|const \[dragOriginX" $f   # Expected: 0
grep -c "const dragXRef = useRef<number>(0)" $f                                          # Expected: ≥1
grep -n "dragBoxRef.current?.style.transform" $f                                          # Expected: onDragMove 内 ≥1
grep -n "style={{" $f | head -3                                                           # transform 不得出现
```
Expected: ① 无 drag useState；② 有 dragXRef useRef；③ 有 apply 直写；④ style prop 无 transform 条目。

- [ ] **Step 3: 运行门禁**

Run: `bash scripts/verify-react.sh`
Expected: `GATE PASSED`（tsc 72 errors 基线不上升——直写产物不得引入新错误类；若 D1 信号初始值翻译产生新 TS 错误，修 codegen 而非豁免）。

- [ ] **Step 4: 运行全量测试套件**

Run: `cargo test 2>&1 | tail -15`
Expected: react 196/196 + compilation_test 7/7 + 全部既有测试 PASS（含 generated_code_validation.rs 的 react 套件；flutter 套件应保持现状——D3 过滤不在此阶段动）。

- [ ] **Step 5: Commit**

```bash
git add test/projects/learn-omniview/build/react-test-app scripts/verify-react.exemptions
git commit -m "chore(verify): 直写通道产物再生 + 审计断言（dragX/dragY useState→useRef，style prop 剔除）+ 门禁通过"
```
（若 verify-react.exemptions 无变化则不加该文件。）

### Task 6: 浏览器实测 + 文档修订

**Files:**
- Modify: `docs/design/element-ref-system.md`（:388-393 注记修订）
- Modify: `docs/design/ref-high-frequency-direct-channel.md`（Phase 1 状态更新）
- Modify: `.architecture/decisions/adrs/ADR-071-ref-semantic-compiler-responsibility.md`（验收清单勾选）

- [ ] **Step 1: 浏览器实测拖拽（零重渲染断言）**

Run:
```bash
cd test/projects/learn-omniview/build/react-test-app && npm run dev
```
用 Playwright（或手动）打开页面 → ElementRef 页 → 拖拽 DragBox：
- 断言：拖拽精确跟随指针（与 OmniView 行为一致）
- 断言：React DevTools Profiler 录制拖拽期间**零组件渲染提交**（设置 `onDragMove` 内 `dragXRef.current = ...` 的变更不触发 render——若 Profiler 显示渲染，说明 D1 判定漏了消费点，回 Task 1 修正判定）

（Playwright 脚本可参考 `docs/superpowers/plans/` 既往 E2E 脚本或手动验证；零渲染断言是 Phase 1 的核心验收。）

- [ ] **Step 2: 修订 element-ref-system.md 注记**

把 `docs/design/element-ref-system.md` 行 388-393 的注记改为：

```markdown
高频写入路径是 **信号 + 声明式 style 绑定**（`style: { transform: ... }`）——`ref.style` 的写仍是编译错误，
迁移提示为 `use style:{...} binding with signals instead`。style 绑定在 OmniView 目标生成
`el.style.setProperty(...)` 直接 DOM 写入（零 reconciliation）；在 React 目标走**直写通道**（Phase 1，
`docs/design/ref-high-frequency-direct-channel.md`）：D1/D2 信号 useRef 存储 + mutation 点直写
（零 reconciliation，与 OmniView 同级），B2 共享信号 useEffect 直写——v2 时代「退化为 useState +
style={{...}} 整组件重渲染」的代价陈述已被直写通道撤销。
```

- [ ] **Step 3: 更新设计文档与 ADR 状态**

- `docs/design/ref-high-frequency-direct-channel.md`：§4 标注「Phase 1 已实施」+ 产物审计结果摘要
- `ADR-071` Validation 清单勾选前两项（React 无 useState 重渲染路径 / 浏览器实测同级流畅）

- [ ] **Step 4: Commit**

```bash
git add docs/design/element-ref-system.md docs/design/ref-high-frequency-direct-channel.md .architecture/decisions/adrs/ADR-071-ref-semantic-compiler-responsibility.md
git commit -m "docs: 直写通道实施完成——element-ref-system v2 代价陈述撤销 + ADR-071 验收勾选"
```

---

## Phase 2: @drag 意图化（capturePointer 语义上移）

> 语义草案见 `docs/design/ref-high-frequency-direct-channel.md` §5。正式决策在实施前由独立 ADR 评审（ADR-071 决策 3 要求）——**Task 7 Step 1 先落 ADR，评审通过后再动代码**。

### Task 7: @drag 语义 ADR + parser/AST/IR

**Files:**
- Create: `.architecture/decisions/adrs/ADR-072-drag-intent-semantics.md`
- Modify: `crates/omnilang-core/src/parser/ast/view.rs`（ElementProp 枚举）
- Modify: `crates/omnilang-core/src/parser/view/style.rs` 或 `directives.rs`（属性解析）
- Modify: `crates/omnilang-core/src/ir/types.rs`（ElementIR 新字段）
- Modify: `crates/omnilang-core/src/ir/lower/view.rs`（lowering）
- Modify: `crates/omnilang-core/src/semantic/`（验证）
- Test: `crates/omnilang-core/src/parser/` 既有测试模块 + `omnilang/tests/` 或 `design_parse_test.rs` 模式

- [ ] **Step 1: 撰写 @drag 独立 ADR 并通过评审**

Create `ADR-072-drag-intent-semantics.md`，内容按 ADR 模板（Context/Decision/Consequences/Validation），决策要点：

```markdown
# ADR-072: @drag 可拖拽意图语义（capturePointer 语义上移）

## Decision
1. 语言新增 @drag / @dragEnd 元素指令：「指针在元素上按下 → 捕获拖动 →
   拖动期间持续回调 @drag；抬起回调 @dragEnd」。事件数据
   `{ x, y, dx, dy, pointerId, startX, startY }`（dx/dy 为本次事件相对
   按下点的累计位移，数值型——供各目标地道映射）。
2. @drag 要求同一元素绑定 @ref（语义验证硬错误）。
3. capturePointer/releasePointer 降级为编译器内部实现细节：demo 源码
   不再显式书写；显式书写保留编译错误 + 迁移提示「use @drag instead」。
4. spec §10.3 设计 D3 从硬错误演进为目标差异注记。
```

- [ ] **Step 2: 写失败测试（parse + 诊断）**

在 parser 测试（`crates/omnilang-core/src/parser/` 内既有 tests 模块）追加：

```rust
#[test]
fn parses_drag_directive() {
    let src = r#"
component Drag {
  ref box: Element
  fn move(e) { }
  fn end(e) { }
  view { <div @ref={box} @drag={move} @dragEnd={end} /> }
}
"#;
    // 断言：无 parse 错误；元素含 Drag 属性（on_drag="move", on_drag_end="end"）
}

#[test]
fn drag_requires_ref_error() {
    // <div @drag={move} />（无 @ref）→ 语义错误「@drag requires @ref」
}

#[test]
fn drag_handler_undefined_error() {
    // @drag={missing}（无此 fn）→ 语义错误
}
```

- [ ] **Step 3: 实现 AST/parser/IR/lowering/语义验证**

**AST**（`parser/ast/view.rs` ElementProp 枚举）：

```rust
/// @drag={onDrag} @dragEnd={onDragEnd} — draggable-intent directive
/// (ADR-072). Compiler wires pointer capture internally per target.
Drag {
    on_drag: String,
    on_drag_end: Option<String>,
},
```

**Parser**：在元素属性解析循环（定位：`grep -n "is_event_like" crates/omnilang-core/src/parser/view/style.rs` 所在函数，事件属性按 `@onX` 前缀分派处）增加特例：

```rust
// @drag / @dragEnd — draggable intent (ADR-072). Name-based dispatch
// BEFORE the generic is_event_like path.
if name == "drag" || name == "dragEnd" {
    // 值必须为裸标识符（handler 名）
    let handler = parser.expect_identifier_name()?;
    if name == "drag" {
        props.push(ElementProp::Drag { on_drag: handler, on_drag_end: None });
    } else {
        // 合并到已存在的 Drag 属性
        if let Some(ElementProp::Drag { on_drag_end, .. }) = props.iter_mut().find(...) {
            *on_drag_end = Some(handler);
        } else { /* 无 @drag 只有 @dragEnd → 错误 */ }
    }
    continue;
}
```

**IR**（`ir/types.rs` ElementIR）：

```rust
/// @drag / @dragEnd handlers (ADR-072) — draggable intent; each target
/// compiles to its own capture/move wiring.
#[serde(default)]
pub drag_binding: Option<DragBindingIR>,
// ir/types.rs: pub struct DragBindingIR { pub on_drag: String, pub on_drag_end: Option<String> }
```

**Lowering**（`ir/lower/view.rs`）：`ElementProp::Drag` → `el.drag_binding`。

**语义验证**（`semantic/`）：① @drag 元素必须 `@ref`；② handler 必须存在于组件 fns；③ 事件数据签名校验（handler 参数接受 `{x,y,dx,dy,pointerId,startX,startY}` 对象）。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --lib parser 2>&1 | tail -6`（及语义验证测试）
Expected: 新测试 PASS，无回归。

- [ ] **Step 5: Commit**

```bash
git add .architecture/decisions/adrs/ADR-072-drag-intent-semantics.md crates/omnilang-core/src/parser/ast/view.rs crates/omnilang-core/src/parser/view/ crates/omnilang-core/src/ir/types.rs crates/omnilang-core/src/ir/lower/view.rs crates/omnilang-core/src/semantic/
git commit -m "feat(parser): @drag/@dragEnd 可拖拽意图语义（ADR-072）——AST/IR/lowering/语义验证"
```

### Task 8: OmniView codegen @drag

**Files:**
- Modify: `omnilang/src/codegen/omniview/`（事件发射 + ref 封装）
- Test: `omnilang/src/codegen/omniview/tests.rs`

- [ ] **Step 1: 写失败测试（产物断言）**

OmniView 目标产物形态（@drag 编译后 JS 应含）：

```js
// 内部接线（捕获 + 拖动路由）：
//   pointerdown → 记录起点 + setPointerCapture(pointerId)
//   pointermove → onDrag({ x, y, dx: x-startX, dy: y-startY, pointerId, startX, startY })
//   pointerup/cancel → releasePointer + onDragEnd(...)
```

测试断言：生成 JS 含 `setPointerCapture`、`releasePointer`、`onDragMove` 回调名、dx/dy 计算表达式。

- [ ] **Step 2: 运行确认失败** → `cargo test --lib codegen::omniview 2>&1 | tail -5`（FAIL：无 @drag 发射）

- [ ] **Step 3: 实现**

定位 OmniView 事件属性发射处（`grep -rn "addEventListener" omnilang/src/codegen/omniview/` → 事件接线模块）与 ref wrapper 生成处（`store.rs`/`ref` 相关）。`@drag` 发射为合成事件接线：

```rust
// 在事件接线生成中，对 el.drag_binding：
// 1. pointerdown: 记录起点（闭包变量 _dragStart）+ setPointerCapture
// 2. pointermove: 若有起点 → 计算 dx/dy → 调用 on_drag handler
// 3. pointerup/pointercancel: releasePointer + on_drag_end
```

- [ ] **Step 4: 运行通过 + Commit**

```bash
git add omnilang/src/codegen/omniview/ && git commit -m "feat(codegen): OmniView @drag 发射——capturePointer 内部化 + dx/dy 合成事件"
```

### Task 9: React codegen @drag

**Files:**
- Modify: `omnilang/src/codegen/react/`（事件发射 + 直写通道产物）
- Test: `omnilang/src/codegen/react/tests.rs`

- [ ] **Step 1: 写失败测试（产物断言）**

React 目标产物形态（@drag 编译后 TSX）：

```tsx
<div ref={dragBoxRef}
     onPointerDown={(e) => { e.currentTarget.setPointerCapture(e.pointerId); onDragStartRef.current = { x: e.clientX, y: e.clientY }; }}
     onPointerMove={(e) => { if (onDragStartRef.current) onDragMove({ x: e.clientX ?? 0, y: e.clientY ?? 0, dx: ..., dy: ..., pointerId: e.pointerId, startX: ..., startY: ... }) }}
     onPointerUp={...}
     onPointerCancel={...}>
```

断言：含 `setPointerCapture`；含 `onDragMove({ ... dx:`；**不含**源码级 `capturePointer` 名（意图已内部化）。

- [ ] **Step 2: 运行确认失败** → FAIL（无 @drag 发射）

- [ ] **Step 3: 实现**

定位 React 事件 prop 发射（`view.rs` 的 `@onX` → JSX onX 映射）与 handler 包装（`onPointerDown={(e) => { ... }}` 形态生成）。`@drag` 发射为上述四事件接线 + 起点 ref（`const dragStartRef = useRef<{x:number,y:number}|null>(null)`）。若 handler 是组件 fn（demo 现状），接线内调用 fn 并传合成数据对象——复用既有事件数据组装（`EventDataKind` 提取）。

- [ ] **Step 4: 运行通过 + Commit**

```bash
git add omnilang/src/codegen/react/ && git commit -m "feat(codegen): React @drag 发射——setPointerCapture 内部化 + 合成事件数据"
```

### Task 10: Flutter codegen @drag（GestureDetector）

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs`（widget 包装）
- Modify: `omnilang/src/codegen/flutter/ref.rs`（D3 诊断演进）
- Test: `omnilang/src/codegen/flutter/tests.rs`

- [ ] **Step 1: 写失败测试（产物断言）**

Flutter 目标产物形态（@drag 编译后 Dart）：

```dart
GestureDetector(
  onPanStart: (d) => { _dragStart = d.localPosition; onDragStart(...); },
  onPanUpdate: (d) => onDragMove({ 'x': ..., 'dx': d.localPosition.dx - _dragStart.dx, ... }),
  onPanEnd: (_) => onDragEnd(...),
  child: ...,
)
```

断言：含 `GestureDetector`、`onPanUpdate`；含 `onDragMove` 回调接线。

- [ ] **Step 2: 运行确认失败** → FAIL

- [ ] **Step 3: 实现**

定位 Flutter 元素 widget 生成（`view.rs` gen_view_node）与事件参数（`grep -rn "onTap\|onChanged\|GestureDetector" omnilang/src/codegen/flutter/`）。`@drag` → 元素包裹 `GestureDetector`（onPanStart/Update/End），事件数据组装按既有 EventDataKind 提取（x/y/dx/dy 用 `d.localPosition.dx` 等）。**`ref.rs:36` 的 `FLUTTER_POINTER_CAPTURE_ERROR` 保留**——显式书写 capturePointer 仍硬错误（防回潮），提示更新为「use @drag instead」。

- [ ] **Step 4: 运行通过 + Commit**

```bash
git add omnilang/src/codegen/flutter/ && git commit -m "feat(codegen): Flutter @drag 发射——GestureDetector onPan 接线 + D3 提示更新"
```

### Task 11: demo 改造 + D3 演进 + gate 更新

**Files:**
- Modify: `test/projects/learn-omniview/src/pages/ElementRefDemo.omni`
- Modify: `omnilang/tests/generated_code_validation.rs`（两处 D3 过滤移除）
- Modify: `scripts/verify-demo-coverage.py`（覆盖口径：capturePointer → @drag）
- Modify: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`（:204/:1156 D3 演进）

- [ ] **Step 1: demo 拖拽区改造**

`ElementRefDemo.omni` 拖拽区（行 435-447 区域）从显式 capturePointer 三事件改 @drag 意图：

```omni
fn onDragDown(e) {
    dragging = true;
    dragOriginX = e.x;
    dragOriginY = e.y;
}

fn onDragMove(e) {
    if dragging {
        dragX = e.x - dragOriginX;
        dragY = e.y - dragOriginY;
    }
}

fn onDragEnd(e) {
    dragging = false;
}
```

```omni
<div
    @ref={dragBox}
    @drag={onDragMove}
    @dragEnd={onDragEnd}
    style: { transform: "translate(" + dragX + "px, " + dragY + "px)" }
    class: { DragBox: true, DragBoxActive: dragging }
>
```

（onDragDown 的 capturePointer 行删除；教学区 pointerBox 的显式 capturePointer/releasePointer 改为 @drag 演示 + 目标差异注记。）

- [ ] **Step 2: gate D3 过滤移除**

`generated_code_validation.rs` 两处（行 704-711、1302-1309 区域）删除 `.filter(|d| !d.message.contains("Flutter 不支持指针捕获"))`，改为完整错误断言（demo 改造后应无该诊断）。

- [ ] **Step 3: 覆盖审计更新**

`verify-demo-coverage.py`：ref 域覆盖口径从「capturePointer/releasePointer 显式展示」改为「@drag 意图展示」；复跑确认缺口 0。

- [ ] **Step 4: 三目标全量验证**

```bash
cd test/projects/learn-omniview && ../../target/release/omnic build --target react -o build/react-test-app
../../target/release/omnic build --target flutter -o build/flutter-test-app   # 应 EXIT=0（无 D3 诊断）
cd ../.. && bash scripts/verify-react.sh && cargo test 2>&1 | tail -8
```
Expected: react 门禁 PASSED + flutter 编译成功（demo 不再含显式 capturePointer）+ 全量测试绿 + 浏览器实测拖拽三目标行为一致。

- [ ] **Step 5: Commit**

```bash
git add test/projects/learn-omniview/src/pages/ElementRefDemo.omni omnilang/tests/generated_code_validation.rs scripts/verify-demo-coverage.py docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md
git commit -m "feat(demo): ElementRefDemo 拖拽意图化（@drag）+ D3 演进为目标差异注记 + gate 过滤移除"
```

---

## Phase 3: Flutter inline_style 表达式化

> 依赖 Phase 2 完成（demo 已无显式 capturePointer，flutter 可编译）。

### Task 12: 分类 pass 提为共享 + Flutter 接入

**Files:**
- Modify: `omnilang/src/codegen/react/component/direct_write.rs` → 迁移到 `omnilang/src/codegen/shared/direct_write.rs`
- Modify: `omnilang/src/codegen/react/component/mod.rs`（引用路径）
- Test: `omnilang/src/codegen/react/tests.rs`（引用路径）+ 新 `omnilang/src/codegen/flutter/tests.rs` 用例

- [ ] **Step 1: 写失败测试（flutter 侧分类接入）**

`flutter/tests.rs` 追加（复用既有 `compile_flutter_full` helper）：

```rust
#[test]
fn test_flutter_classifies_drag_signals() {
    // 编译含 @ref + 动态 style 的组件，断言 D1/D2 集（与 react 同 pass）
}
```

- [ ] **Step 2: 运行确认失败** → FAIL（shared 模块不存在）

- [ ] **Step 3: 迁移**

把 `react/component/direct_write.rs` 整体移到 `codegen/shared/direct_write.rs`（`pub(super)` → `pub(crate)`；删除 react 专属依赖——`comp_mutated_signals` 委托的 `collect_signal_mutations` 在 react/component/state.rs，改为在 shared 内实现同名遍历或经参数注入）。React 侧 `mod.rs`/`expr.rs` 引用路径更新。既有 react 测试保持通过（测试不迁移，只改引用）。

- [ ] **Step 4: 运行通过 + Commit**

```bash
git add omnilang/src/codegen/shared/direct_write.rs omnilang/src/codegen/react/ omnilang/src/codegen/flutter/tests.rs
git commit -m "refactor(codegen): 信号域分类提为 shared/direct_write——React/Flutter 共享分析 pass"
```

### Task 13: Flutter D1/D2 存储（ValueNotifier）+ mutation 发射

**Files:**
- Modify: `omnilang/src/codegen/flutter/`（State 类字段生成）
- Modify: `omnilang/src/codegen/flutter/stmt.rs`（FlutterStmtFormatter 直写分支）
- Test: `omnilang/src/codegen/flutter/tests.rs`

- [ ] **Step 1: 写失败测试（产物断言）**

```rust
#[test]
fn test_flutter_direct_write_value_notifier() {
    // 编译拖拽组件 → 断言 Dart 产物：
    //   final _dragX = ValueNotifier<double>(0);   （D1 存储）
    //   onDragMove 内：_dragX.value = ...;         （mutation 直写，无 setState）
    //   无 setState(() { ... }) 包裹 dragX mutation
}
```

- [ ] **Step 2: 运行确认失败** → FAIL（信号仍为 State 字段 + setState）

- [ ] **Step 3: 实现**

1. **字段生成**：定位 State 类信号字段发射（`grep -rn "setState" omnilang/src/codegen/flutter/mod.rs` → State 字段区）。D1/D2 信号发射为：

```dart
final _dragX = ValueNotifier<double>(0);
```

（命名 `_` 前缀 + `.value` 读写。信号在其他位置的标识符解析同步改名——`flutter/expr.rs` 的信号标识符解析处加 `_name.value` 变体。）

2. **mutation 发射**（`stmt.rs` `FlutterStmtFormatter`）：在 `format_mutation_stmt` 开头短路 D1/D2（绕开 `setState` 前缀/后缀包装）：

```rust
// Phase 3: D1/D2 signals write the ValueNotifier directly — no setState,
// no whole-tree build.
if let MutationIntent::DirectAssign { target, value, .. } = intent {
    if self.direct_write_signals.contains(&target.name) {
        let amount = value.as_ref().map(|v| self.expr_fmt().format_expr(v)).unwrap_or_else(|| "0".into());
        w.emit_fmt(format_args!("_{}.value = {};", target.name, amount));
        return;
    }
}
```

（`direct_write_signals` 为 FlutterStmtFormatter 新字段，构造处填充 shared 分类结果。）

- [ ] **Step 4: 运行通过 + Commit**

```bash
git add omnilang/src/codegen/flutter/ && git commit -m "feat(codegen): Flutter D1/D2 信号 ValueNotifier 存储 + mutation 直写（零 setState）"
```

### Task 14: ListenableBuilder 包裹 + 属性映射表

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs`（inline_style 发射 + 元素包裹）
- Modify: `omnilang/src/codegen/shared/direct_write.rs`（style 值模式匹配）
- Test: `omnilang/src/codegen/flutter/tests.rs`

- [ ] **Step 1: 写失败测试（产物断言）**

```rust
#[test]
fn test_flutter_style_binding_listenable_builder() {
    // 编译拖拽组件 → 断言 Dart 产物：
    //   ListenableBuilder(
    //     listenable: Listenable.merge([_dragX, _dragY]),
    //     builder: (context, _) => Transform.translate(
    //       offset: Offset(_dragX.value, _dragY.value),
    //       ...
    //   )
    // 拖拽信号 mutation 不再 setState 也能重建位移（ValueNotifier 通知）
}

#[test]
fn test_flutter_unmappable_style_diagnostic() {
    // style: { letterSpacing: sig }（不可映射）→ 编译期诊断（非静默跳过）
}
```

- [ ] **Step 2: 运行确认失败** → FAIL（动态 style 静默跳过）

- [ ] **Step 3: 实现 style 值模式匹配（shared/direct_write.rs）**

```rust
/// Flutter-side style value pattern match — CSS-value shapes that map to
/// widget parameters (ADR-071 decision 2 / design §6.4, option B).
#[derive(Debug, PartialEq)]
pub(crate) enum StylePattern {
    /// `"translate(" + x + "px, " + y + "px)"` concat shape
    /// (5 段：["translate(", <x>, "px, ", <y>, "px)"]).
    Translate { x: Value, y: Value },
    /// Bare signal/number/string values for direct parameters.
    Direct(Value),
    /// Anything else — unmappable on Flutter (compile-time diagnostic).
    Unmappable,
}

pub(crate) fn match_style_pattern(val: &Value) -> StylePattern {
    if let Value::Object(obj) = val {
        if let Some(parts) = obj.get("$concat").and_then(|v| v.as_array()) {
            let n = parts.len();
            // "translate(" + x + "px, " + y + "px)" 五段形态
            if n == 5
                && parts[0] == &Value::String("translate(".into())
                && parts[2] == &Value::String("px, ".into())
                && parts[4] == &Value::String("px)".into())
            {
                return StylePattern::Translate {
                    x: parts[1].clone(),
                    y: parts[3].clone(),
                };
            }
        }
        // 非 translate 形态的动态值 → 单值表达式（按 CSS 属性映射参数）
        if obj.contains_key("$signal")
            || obj.contains_key("$ident")
            || obj.contains_key("$field")
            || obj.contains_key("$call")
            || obj.contains_key("$concat")
        {
            return StylePattern::Direct(val.clone());
        }
        return StylePattern::Unmappable;
    }
    match val {
        Value::Number(_) | Value::String(_) | Value::Bool(_) => {
            StylePattern::Direct(val.clone())
        }
        _ => StylePattern::Unmappable,
    }
}
```

**view.rs inline_style 改造**（替换现状 `as_str().unwrap_or("")` 静默跳过）：

```rust
// Phase 3: 动态 style 值 → ListenableBuilder 包裹 + 属性映射
//   match_style_pattern(val) {
//     Translate { x, y }  → Transform.translate(offset: Offset(<x>, <y>))
//     Direct(v)           → 参数映射（Opacity/SizedBox/TextStyle ...，按 CSS 属性）
//     Unmappable          → ctx.error(...) 编译期显式诊断（替代静默跳过）
//   }
// 元素有 ≥1 激活条目 → 包裹 ListenableBuilder(listenable: Listenable.merge(
//   [条目读信号的 _name ValueNotifier]), builder: (context, _) => <改造后子树>)
```

- [ ] **Step 4: 运行通过 + Commit**

```bash
git add omnilang/src/codegen/flutter/view.rs omnilang/src/codegen/shared/direct_write.rs omnilang/src/codegen/flutter/tests.rs
git commit -m "feat(codegen): Flutter 动态 style 表达式化——ListenableBuilder + Transform 映射 + 不可映射显式诊断"
```

### Task 15: gate 演进 + 全量验证 + 文档收尾

**Files:**
- Modify: `omnilang/tests/generated_code_validation.rs`（flutter 行为验证）
- Modify: `docs/design/ref-high-frequency-direct-channel.md`（Phase 3 状态）
- Modify: `.architecture/decisions/adrs/ADR-071-ref-semantic-compiler-responsibility.md`（验收勾选）
- Modify: `docs/design/element-ref-system.md`（Flutter 状态行）

- [ ] **Step 1: flutter gate 行为验证**

`generated_code_validation.rs` 的 flutter 部分（`compile_learn_omniview_flutter` / `learn_omniview_flutter_output_passes_dart_analyze`）：在既有编译断言之外新增行为断言：

```rust
// Phase 3: 拖拽位移正确渲染 —— 生成代码含 ListenableBuilder + ValueNotifier
let dart = std::fs::read_to_string(out.join("pages/ElementRefDemo.dart")).unwrap();
assert!(dart.contains("ListenableBuilder"), "flutter drag must use ListenableBuilder");
assert!(dart.contains("Listenable.merge"), "multi-signal entries merge notifiers");
assert!(dart.contains("Transform.translate"), "transform maps to Transform");
```

- [ ] **Step 2: 三目标全量验证**

```bash
bash scripts/verify-react.sh && cargo test 2>&1 | tail -8
python scripts/verify-demo-coverage.py
```
Expected: react 门禁 PASSED；全量测试绿（含 flutter gate 行为断言）；覆盖缺口 0；浏览器/模拟器实测 Flutter 拖拽位移正确渲染（`_dragX.value` 变更 → Transform 跟随，无整树 build——用 Flutter DevTools 检查 rebuild 范围）。

- [ ] **Step 3: 文档收尾**

- `ref-high-frequency-direct-channel.md`：§6 标注「Phase 3 已实施」+ 映射表实况
- `element-ref-system.md`：Flutter 状态行更新（动态 style 从「静默跳过」→「ListenableBuilder 表达式化」）
- `ADR-071` Validation 清单全勾选
- 记忆库更新（新会话 memory：三阶段完成实况 + 产物证据）

- [ ] **Step 4: Commit**

```bash
git add omnilang/tests/generated_code_validation.rs docs/ .architecture/decisions/adrs/ADR-071-ref-semantic-compiler-responsibility.md
git commit -m "docs: 三阶段实施完成——Flutter 行为 gate + ADR-071 验收全勾选 + 设计文档状态更新"
```

---

## Self-Review 记录

**1. Spec coverage（对照 `docs/design/ref-high-frequency-direct-channel.md`）：**
- §4 React 直写通道 → Task 1-6（判定/存储/mutation/B2/门禁/文档）✅
- §5 @drag 意图化 → Task 7-11（ADR/parser/三目标 codegen/demo/gate）✅
- §6 Flutter 表达式化 → Task 12-15（共享 pass/ValueNotifier/ListenableBuilder/gate）✅
- §7 排期验证矩阵 → 各 Task 的验证步骤内嵌 ✅
- §9 验收清单 → Task 6/11/15 的验收步骤对应 ✅

**2. 已知待实现时确认的点（非占位，是显式定位步骤）：**
- `ViewTree`/`ElementIR` 字段名（`for_binding`、`inline_style` 结构）——Task 1 Step 3/4 已标注「按编译器提示修正」
- `js_type_for`/`gen_state_hooks` 初始值翻译复用——Task 2 Step 3 已标注复用点
- `emit_sanitize_css` 可见性——Task 3 Step 3 含 grep 定位步骤
- `collect_view_inline_handler_reads` 定义位置——Task 1 Step 4 含 grep 定位步骤
- Flutter State 字段发射点/事件参数组装点——Task 13/14/10 含 grep 定位步骤

**3. 类型一致性：**
- `DirectWriteSets`（d1/d2/entries/b2_entries）在 Task 1 定义，Task 2/3/4/12/13 全部沿用
- `DirectWriteEntry`（ref_name/prop/value/read_signals）Task 1 定义，Task 3/4 消费
- `classify_direct_write(comp) -> DirectWriteSets` 签名跨任务一致
- apply 命名 `{ref}Ref.current?.style.{prop} = {value}` 在 Task 3 测试与实现一致
- `StylePattern` 在 Task 14 定义并被 Task 14/15 使用
