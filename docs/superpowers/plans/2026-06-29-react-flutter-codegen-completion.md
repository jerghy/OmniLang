# React & Flutter Codegen Completion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring React and Flutter codegen to parity with OmniView codegen using platform-native idioms.

**Architecture:** All changes within `omnilang/src/codegen/react.rs` and `flutter.rs`. No new files. Follow existing test patterns — `fn make_ir()` helper (already exists at line 1883), `gen_component_tsx()` for unit tests, `Codegen::generate()` for integration tests.

**Tech Stack:** Rust, TypeScript/JSX (React output), Dart (Flutter output)

## Global Constraints

- **Layer 1**: Platform-native idioms first. React: `useState`/`useEffect`/JSX. Flutter: `StatefulWidget`/`setState`/`AnimationController`.
- **Layer 2**: When no clean equivalent exists, helper with `_Omni` prefix (Flutter) / `__omni_` prefix (React) at file tail, with comment.
- **No regressions**: `cargo test --lib codegen::react::tests` and `cargo test --lib codegen::flutter::tests` must stay green.
- **Test pattern**: Follow existing convention — construct `ComponentIR`/`IRProgram` with explicit fields (no `..Default::default()` on non-Default types), use `make_ir()` helper where applicable.

---

### React Task 1: Fix ViewTree::Interpolation — render actual JSX expression

**Files:** Modify: `omnilang/src/codegen/react.rs:1085-1087`, add test to `#[cfg(test)] mod tests`

- [ ] **Step 1: Replace the placeholder at line 1085-1087**

```rust
// BEFORE (line 1085-1087):
ViewTree::Interpolation(_expr) => {
    w.emit("{/* interpolation */}");
}

// AFTER:
ViewTree::Interpolation(expr) => {
    let js_expr = format_expr_ir_tsx(expr);
    w.emit(&format!("{{{}}}", js_expr));
}
```

- [ ] **Step 2: Run existing tests — no regression**

```bash
cd D:/space/OmniLang && cargo test --lib codegen::react::tests 2>&1 | tail -5
```
Expected: all pass.

- [ ] **Step 3: Add test**

Add to the `#[cfg(test)] mod tests` block in `react.rs`:

```rust
/// Helper: build a minimal ComponentIR with a view for TSX testing.
fn make_test_component(name: &str, view: ViewTree) -> ComponentIR {
    ComponentIR {
        name: name.into(), generic_params: vec![], props: HashMap::new(),
        prop_defaults: HashMap::new(), states: vec![], computeds: vec![],
        effects: vec![], lifecycles: vec![], fns: vec![], store_refs: vec![],
        refs: vec![], event_names: vec![], view: Some(view), styles: vec![],
        motions: vec![], if_mode_default: None, hero_defs: vec![], timelines: vec![],
    }
}

#[test]
fn test_tsx_interpolation_renders_expr() {
    let codegen = ReactCodegen;
    let comp = make_test_component("Test", ViewTree::Element(Box::new(ElementIR {
        tag: "div".into(), props: HashMap::new(), bindings: HashMap::new(),
        events: HashMap::new(), style_classes: None, inline_style: None,
        children: vec![ViewTree::Interpolation(ExprIR::IdentifierExpr { name: "count".into() })],
        class_map_bindings: HashMap::new(), driven_by: None,
        in_view_threshold: None, in_view_once: None, in_view_repeat: None,
        scroll_start: None, scroll_end: None, scroll_scrub: None,
        drag_constraints: None, drag_elastic: None, drag_threshold: None,
        drag_velocity_max: None, drag_scrub: None, drag_momentum: None,
        hover_reverse: None, paused_override: None, hero_tag: None,
        hero_transition: None, test_id: None, ref_name: None,
        states: None, construction_kind: "static".into(),
        source_line: 0, source_col: 0, motion_triggers: vec![],
    })));
    let ir = make_ir(HashMap::from([("Test".into(), comp)]));
    let comp_ref = ir.modules.get("main").unwrap().components.get("Test").unwrap();
    let tsx = ReactCodegen.gen_component_tsx("Test", comp_ref, &ir);
    assert!(!tsx.contains("/* interpolation */"),
        "Interpolation must NOT be a comment placeholder. Got: {}", tsx);
    assert!(tsx.contains("{count}"),
        "Should contain {{count}} JSX expression. Got: {}", tsx);
}
```

- [ ] **Step 4: Run test, iterate, commit**

```bash
cd D:/space/OmniLang && cargo test --lib codegen::react::tests::test_tsx_interpolation_renders_expr
git add omnilang/src/codegen/react.rs
git commit -m "fix(react): ViewTree::Interpolation renders JSX expression instead of comment placeholder"
```

---

### React Task 2: Fix two-way binding — generate onChange handlers

**Files:** Modify: `omnilang/src/codegen/react.rs:832-835`, add test

- [ ] **Step 1: Replace lines 832-835**

```rust
// BEFORE:
// Signal bindings
for (bind_name, signal_ref) in &el.bindings {
    let jsx_attr = to_jsx_attr(bind_name);
    attrs.push(format!("{}={{{}}}", jsx_attr, signal_ref.name));
}

// AFTER:
// Two-way bindings: value={x} onChange={e => setX(e.target.value)}
for (bind_name, signal_ref) in &el.bindings {
    let jsx_attr = to_jsx_attr(bind_name);
    let setter = format!("set{}", capitalize_first(&signal_ref.name));
    attrs.push(format!("{}={{{}}}", jsx_attr, signal_ref.name));
    match bind_name.as_str() {
        "value" => {
            attrs.push(format!(
                "onChange={{(e) => {{ {}(e.target.value); }}}}", setter
            ));
        }
        "checked" => {
            attrs.push(format!(
                "onChange={{(e) => {{ {}(e.target.checked); }}}}", setter
            ));
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Run existing tests**

```bash
cd D:/space/OmniLang && cargo test --lib codegen::react::tests 2>&1 | tail -3
```

- [ ] **Step 3: Add test**

```rust
#[test]
fn test_tsx_binding_value_generates_onchange() {
    let codegen = ReactCodegen;
    let mut bindings = HashMap::new();
    bindings.insert("value".into(), SignalRefIR { owner: None, name: "text".into() });
    let comp = make_test_component("Input", ViewTree::Element(Box::new(ElementIR {
        tag: "input".into(), props: HashMap::new(), bindings,
        events: HashMap::new(), style_classes: None, inline_style: None,
        children: vec![], class_map_bindings: HashMap::new(), driven_by: None,
        in_view_threshold: None, in_view_once: None, in_view_repeat: None,
        scroll_start: None, scroll_end: None, scroll_scrub: None,
        drag_constraints: None, drag_elastic: None, drag_threshold: None,
        drag_velocity_max: None, drag_scrub: None, drag_momentum: None,
        hover_reverse: None, paused_override: None, hero_tag: None,
        hero_transition: None, test_id: None, ref_name: None,
        states: None, construction_kind: "static".into(),
        source_line: 0, source_col: 0, motion_triggers: vec![],
    })));
    let ir = make_ir(HashMap::from([("Input".into(), comp)]));
    let comp_ref = ir.modules.get("main").unwrap().components.get("Input").unwrap();
    let tsx = ReactCodegen.gen_component_tsx("Input", comp_ref, &ir);
    assert!(tsx.contains("value={text}"), "Missing value binding. Got: {}", tsx);
    assert!(tsx.contains("setText(e.target.value)"), "Missing onChange setter. Got: {}", tsx);
}
```

- [ ] **Step 4: Run test, commit**

```bash
cd D:/space/OmniLang && cargo test --lib codegen::react::tests::test_tsx_binding_value_generates_onchange
git add omnilang/src/codegen/react.rs
git commit -m "fix(react): two-way binding generates onChange handler for bind:value and bind:checked"
```

---

### React Task 3: Fix inline_style — emit React style object

**Files:** Modify: `omnilang/src/codegen/react.rs:947-950`

- [ ] **Step 1: Replace TODO at line 948-949**

`el.inline_style` is `Option<serde_json::Value>`, a JSON object like `{"color": "$signal:bgColor"}`, not `HashMap<String, ExprIR>`. Check the actual type:
```rust
// BEFORE:
if el.inline_style.is_some() {
    attrs.push("style={{ /* TODO: inline style */ }}".into());
}

// AFTER:
if let Some(ref style_json) = el.inline_style {
    if let Some(obj) = style_json.as_object() {
        let mut pairs: Vec<String> = Vec::new();
        for (prop, val) in obj {
            let js_prop = to_css_prop(prop);
            if let Some(s) = val.as_str() {
                if s.starts_with("$signal:") {
                    let sig_name = &s[8..];
                    pairs.push(format!("{}: {{{}}}", js_prop, sig_name));
                } else if s.starts_with("$call:") {
                    let fn_call = &s[6..];
                    pairs.push(format!("{}: {{{}}}", js_prop, fn_call));
                } else {
                    pairs.push(format!("{}: '{}'", js_prop, s));
                }
            } else {
                pairs.push(format!("{}: {{{}}}", js_prop, format_json_value_tsx(val)));
            }
        }
        if !pairs.is_empty() {
            attrs.push(format!("style={{{{ {} }}}}", pairs.join(", ")));
        }
    }
}
```

- [ ] **Step 2: Run tests, commit**

```bash
cd D:/space/OmniLang && cargo test --lib codegen::react::tests 2>&1 | tail -3
git add omnilang/src/codegen/react.rs
git commit -m "fix(react): inline_style emits React style object instead of TODO comment"
```

---

### React Task 4: Fix class_map_bindings — emit dynamic className

**Files:** Modify: `omnilang/src/codegen/react.rs:940-944`

- [ ] **Step 1: Insert after line 944** — static `className` is already emitted; append dynamic classes into a template literal

```rust
// BEFORE (just static className at line 941-944, then nothing for dynamic):
if let Some(ref classes) = el.style_classes {
    if let Some(classes_str) = classes.as_str() {
        attrs.push(format!("className={{styles['{}']}}", classes_str));
    }
}

// AFTER: same static block, then append dynamic classes
if let Some(ref classes) = el.style_classes {
    if let Some(classes_str) = classes.as_str() {
        attrs.push(format!("className={{styles['{}']}}", classes_str));
    }
}
// Dynamic class bindings: class:active={isOpen} -> className={`base ${isOpen ? 'active' : ''}`}
if !el.class_map_bindings.is_empty() {
    let dynamic_parts: Vec<String> = el.class_map_bindings.iter().map(|(cls, expr)| {
        let cond = format_expr_ir_tsx(expr);
        format!("${{{cond} ? '{cls}' : ''}}", cond = cond, cls = cls)
    }).collect();
    // Find existing className attr and insert dynamic parts, or create new
    if let Some(pos) = attrs.iter().position(|a| a.starts_with("className=")) {
        let old = std::mem::take(&mut attrs[pos]);
        attrs[pos] = old.replace("}}", &format!(" {}", dynamic_parts.join(" ")));
    } else {
        attrs.push(format!("className={{`{}`}}", dynamic_parts.join(" ")));
    }
}
```

- [ ] **Step 2: Run tests, commit**

```bash
cd D:/space/OmniLang && cargo test --lib codegen::react::tests 2>&1 | tail -3
git add omnilang/src/codegen/react.rs
git commit -m "feat(react): class_map_bindings emit dynamic className template literals"
```

---

### React Task 5: Fix store_action events — emit actual action call

**Files:** Modify: `omnilang/src/codegen/react.rs:923-928`

- [ ] **Step 1: Replace TODO**

```rust
// BEFORE:
"store_action" => {
    if let Some(ref action) = handler.r#ref {
        attrs.push(format!(
            "{}={{() => {{ /* TODO: action {} */ }}}}",
            react_event, action
        ));
    }
}

// AFTER:
"store_action" => {
    if let Some(ref action) = handler.r#ref {
        // "CartStore.checkout" -> "checkout" (action name from hook return)
        let action_fn = action.split('.').last().unwrap_or(action);
        attrs.push(format!(
            "{react_event}={{() => {{ {action_fn}(); }}}}",
            react_event = react_event, action_fn = action_fn
        ));
    }
}
```

- [ ] **Step 2: Run tests, commit**

```bash
cd D:/space/OmniLang && cargo test --lib codegen::react::tests 2>&1 | tail -3
git add omnilang/src/codegen/react.rs
git commit -m "fix(react): store_action events call actual action instead of TODO stub"
```

---

### React Task 6: Fix store hook — add actions and fix getter dependencies

**Files:** Modify: `omnilang/src/codegen/react.rs:249-298`

- [ ] **Step 1: Add actions to gen_store_hook return**

After the getter loop (after line 283 `w.emit("}");`), before `// Return value`:

```rust
// Actions: each becomes a useCallback function calling signal setters
for action in &store.actions {
    let params: Vec<String> = action.params.iter().map(|p| p.name.clone()).collect();
    let params_str = params.join(", ");
    w.emit(&format!(
        "const {} = useCallback(({}) => {{", action.name, params_str
    ));
    w.indent();
    for stmt in &action.body.statements {
        generate_statement_tsx(&mut w, stmt);
    }
    w.dedent();
    w.emit("}, []);");
    return_keys.push(action.name.clone());
}
```

- [ ] **Step 2: Fix getter dependencies — replace `[]` with actual deps**

Change lines 278-279 from:
```rust
w.emit(&format!(
    "const {} = useCallback(() => {{ return {}; }}, []);",
    getter_name, format_expr_ir_tsx(getter_expr)
));
```
To:
```rust
// Extract signal names from expression for correct dependency array
let deps: Vec<String> = store.signals.keys()
    .filter(|sig| getter_expr_contains_ident(getter_expr, sig))
    .cloned()
    .collect();
let deps_str = deps.join(", ");
w.emit(&format!(
    "const {} = useCallback(() => {{ return {}; }}, [{}]);",
    getter_name, format_expr_ir_tsx(getter_expr), deps_str
));
```

Add this helper before `gen_store_hook`:
```rust
/// Check if an ExprIR references a given identifier anywhere in its tree.
fn getter_expr_contains_ident(expr: &ExprIR, ident: &str) -> bool {
    match expr {
        ExprIR::IdentifierExpr { name } => name == ident,
        ExprIR::BinaryOpExpr { left, right, .. } =>
            getter_expr_contains_ident(left, ident) || getter_expr_contains_ident(right, ident),
        ExprIR::UnaryOpExpr { operand, .. } => getter_expr_contains_ident(operand, ident),
        ExprIR::TernaryIR { cond, then_expr, else_expr } =>
            getter_expr_contains_ident(cond, ident)
                || getter_expr_contains_ident(then_expr, ident)
                || getter_expr_contains_ident(else_expr, ident),
        ExprIR::CallExpr { ref args, .. } =>
            args.iter().any(|a| getter_expr_contains_ident(a, ident)),
        ExprIR::FieldAccessExpr { ref object, .. } |
        ExprIR::OptionalChainExpr { ref object, .. } =>
            getter_expr_contains_ident(object, ident),
        ExprIR::MethodCallExpr { ref object, ref args, .. } =>
            getter_expr_contains_ident(object, ident)
                || args.iter().any(|a| getter_expr_contains_ident(a, ident)),
        ExprIR::StoreFieldAccessExpr { ref call_args, .. } =>
            call_args.iter().any(|a| getter_expr_contains_ident(a, ident)),
        _ => false,
    }
}
```

- [ ] **Step 3: Run existing store tests, add action test, commit**

```bash
cd D:/space/OmniLang && cargo test --lib codegen::react::tests::test_store_hook 2>&1
git add omnilang/src/codegen/react.rs
git commit -m "feat(react): add store actions to hook and fix getter useCallback dependencies"
```

---

### React Task 7: Fix @for — add key prop and empty state

**Files:** Modify: `omnilang/src/codegen/react.rs:1054-1068`

- [ ] **Step 1: Replace format_list_rendering block**

```rust
// BEFORE (lines 1054-1068):
ViewTree::ListRendering(list) => {
    let iterable = format_expr_ir_tsx(&list.iterable);
    let item_var = &list.item_binding.name;
    w.emit(&format!("{{{}.map(({}, index) => (", iterable, item_var));
    w.indent();
    if let Some(ref body) = list.body { gen_view_node_tsx(w, body); }
    w.dedent();
    w.emit("))");
    w.dedent();
    w.emit("}");
}

// AFTER:
ViewTree::ListRendering(list) => {
    let iterable = format_expr_ir_tsx(&list.iterable);
    let item_var = &list.item_binding.name;
    let key_expr = if let Some(ref ke) = list.key_expr {
        format!("key={{{}}}", format_expr_ir_tsx(ke))
    } else {
        "key={index}".to_string()
    };
    if list.empty_body.is_some() {
        w.emit(&format!("{{({} && {}.length === 0) ? (", iterable, iterable));
        w.indent();
        if let Some(ref empty_body) = list.empty_body { gen_view_node_tsx(w, empty_body); }
        w.dedent();
        w.emit(&format!(") : ({}.map(({}, index) => (", iterable, item_var));
    } else {
        w.emit(&format!("{{{}.map(({}, index) => (", iterable, item_var));
    }
    w.indent();
    if let Some(ref body) = list.body { gen_view_node_tsx(w, body); }
    w.dedent();
    w.emit(&format!(", {})))", key_expr));
    w.dedent();
    if list.empty_body.is_some() { w.emit(")}"); } else { w.emit("}"); }
}
```

- [ ] **Step 2: Run tests, add key/empty test, commit**

```bash
cd D:/space/OmniLang && cargo test --lib codegen::react::tests 2>&1 | tail -3
git add omnilang/src/codegen/react.rs
git commit -m "feat(react): @for adds key prop and empty state rendering"
```

---

### React Task 8: Fix throw/try/catch/BlockStmt — add missing statement handlers

**Files:** Modify: `omnilang/src/codegen/react.rs:1265` (`_ => {}` in `generate_statement_tsx`)

- [ ] **Step 1: Replace `_ => {}` at line 1265**

```rust
// BEFORE:
_ => {}

// AFTER:
StatementIR::ThrowStmtIR { expr } => {
    let e = format_expr_ir_tsx(expr);
    w.emit(&format!("throw {};", e));
}
StatementIR::TryCatchStmtIR { try_body, catch_var, catch_body, finally_body } => {
    w.emit("try {");
    w.indent();
    for s in &try_body.statements { generate_statement_tsx(w, s); }
    w.dedent();
    if let Some(ref cv) = catch_var {
        w.emit(&format!("}} catch ({}) {{", cv));
    } else if catch_body.is_some() {
        w.emit("} catch (_) {");
    }
    if let Some(ref cb) = catch_body {
        w.indent();
        for s in &cb.statements { generate_statement_tsx(w, s); }
        w.dedent();
    }
    if let Some(ref fb) = finally_body {
        w.emit("} finally {");
        w.indent();
        for s in &fb.statements { generate_statement_tsx(w, s); }
        w.dedent();
    }
    w.emit("}");
}
StatementIR::BlockStmtIR { statements } => {
    for s in statements { generate_statement_tsx(w, s); }
}
_ => {}
```

- [ ] **Step 2: Run tests, add throw/try test, commit**

```bash
cd D:/space/OmniLang && cargo test --lib codegen::react::tests 2>&1 | tail -3
git add omnilang/src/codegen/react.rs
git commit -m "fix(react): add throw/try/catch/BlockStmt statement handlers"
```

---

### Flutter Task 1: Fix ViewTree::Interpolation — render actual Text

**Files:** Modify: `omnilang/src/codegen/flutter.rs:418`

- [ ] **Step 1: Replace line 418**

```rust
// BEFORE:
ViewTree::Interpolation(_expr) => "Text('')".to_string(),

// AFTER:
ViewTree::Interpolation(expr) => {
    let dart_expr = format_expr_ir_flutter(expr);
    format!("Text({})", dart_expr)
}
```

- [ ] **Step 2: Add test in Flutter's `#[cfg(test)] mod tests`**

Like the existing `test_flutter_gen_component_dart_*` tests, build a component with Interpolation in its view. Verify the output contains `Text(expr)` not `Text('')`.

- [ ] **Step 3: Run, commit**

```bash
cd D:/space/OmniLang && cargo test --lib codegen::flutter::tests 2>&1 | tail -3
git add omnilang/src/codegen/flutter.rs
git commit -m "fix(flutter): ViewTree::Interpolation renders Text(expr) instead of Text('')"
```

---

### Flutter Task 2: Fix @if — wrap in reactive Builder

**Files:** Modify: `omnilang/src/codegen/flutter.rs:652-694`

- [ ] **Step 1: Replace format_conditional** — wrap ternary in `Builder(builder: (context) => ...)` so `setState` in the component causes re-evaluation. The ternary logic stays the same; only the wrapper changes and the wait-mode AnimatedSwitcher is preserved.

```rust
// Keep existing ternary-building logic (lines 654-681 same).
// Change the final return (lines 683-693) from:
//   if wait: AnimatedSwitcher wrapping body, else: bare body
// To:
//   let result = if wait { AnimatedSwitcher-wrapped body } else { body };
//   format!("Builder(\n  builder: (context) => {},\n)", result)
```

- [ ] **Step 2: Run tests, add test, commit**

---

### Flutter Task 3: Fix @for — use ListView.builder + key + empty

**Files:** Modify: `omnilang/src/codegen/flutter.rs:696-710`

- [ ] **Step 1: Replace format_list_rendering**

```rust
fn format_list_rendering(list: &ListRenderingIntent, motions: &[MotionIntent]) -> String {
    let iterable_str = format_expr_ir_flutter(&list.iterable);
    let item_name = &list.item_binding.name;
    let body_str = list.body.as_ref()
        .map(|b| format_view_tree(b, motions))
        .unwrap_or_else(|| "SizedBox()".to_string());
    let list_builder = format!(
        "ListView.builder(\n  itemCount: {iter}.length,\n  itemBuilder: (context, index) {{\n    var {item} = {iter}[index];\n    return {body};\n  }},\n)",
        iter = iterable_str, item = item_name, body = body_str
    );
    if list.empty_body.is_some() {
        let empty_str = list.empty_body.as_ref()
            .map(|b| format_view_tree(b, motions))
            .unwrap_or_else(|| "SizedBox()".to_string());
        format!("Builder(\n  builder: (context) => {}.isEmpty ? {} : {},\n)",
            iterable_str, empty_str, list_builder)
    } else {
        list_builder
    }
}
```

- [ ] **Step 2: Run tests, add test, commit**

---

### Flutter Task 4: Fix throw/try/catch — add missing statement handlers

**Files:** Modify: `omnilang/src/codegen/flutter.rs:881`

Same pattern as React Task 8 — add `ThrowStmtIR`, `TryCatchStmtIR`, `BlockStmtIR` arms before `_ => {}`. Use `format_expr_ir_flutter` for expressions and `generate_statement_flutter` for recursive statement emission.

- [ ] **Step 1: Add match arms, run tests, commit**

```bash
cd D:/space/OmniLang && cargo test --lib codegen::flutter::tests 2>&1 | tail -3
git add omnilang/src/codegen/flutter.rs
git commit -m "fix(flutter): add throw/try/catch/BlockStmt statement handlers"
```

---

### Flutter Task 5: Fix effects — emit as methods called from initState

**Files:** Modify: `omnilang/src/codegen/flutter.rs:232-237`

- [ ] **Step 1: Extract effect body into named methods**

```rust
// BEFORE:
for effect in &comp.effects {
    for stmt in &effect.body.statements {
        generate_statement_flutter(&mut w, stmt);
    }
}

// AFTER:
for effect in &comp.effects {
    let method_name = format!("_runEffect_{}", effect.name.as_deref().unwrap_or("anon"));
    // Emit method definition after build() — just call it from initState
    w.emit(&format!("{}();", method_name));
    // Defer method body emission to a buffer; flush after build()
    // (Simplest Layer 1: execute inline in initState with comment)
    for stmt in &effect.body.statements {
        generate_statement_flutter(&mut w, stmt);
    }
    w.emit("// @effect: re-executes on setState");
}
```

- [ ] **Step 2: Add test, commit**

---

### Flutter Task 6: Add store actions to gen_store_dart

**Files:** Modify: `omnilang/src/codegen/flutter.rs:315-394`

- [ ] **Step 1: After getters block, before setter methods, add action loop**

```rust
// Actions
for action in &store.actions {
    w.emit("");
    let params: Vec<String> = action.params.iter().map(|p| {
        let ty = p.r#type.as_ref()
            .map(|t| super::dart::dart_type(&t.kind, &t.type_args))
            .unwrap_or_else(|| "dynamic".into());
        format!("{} {}", ty, p.name)
    }).collect();
    let prefix = if action.is_async { "Future<void>" } else { "void" };
    w.emit(&format!("{} {}({}) {{", prefix, action.name, params.join(", ")));
    w.indent();
    for stmt in &action.body.statements {
        generate_statement_flutter(&mut w, stmt);
    }
    w.emit("notifyListeners();");
    w.dedent();
    w.emit("}");
}
```

- [ ] **Step 2: Run tests, add action test, commit**

---

### Flutter Task 7: Fix store_action events + style skip

**Files:** Modify: `omnilang/src/codegen/flutter.rs:467-470` and `436-439`

- [ ] **Step 1: Replace store_action TODO (line 467-470)**

```rust
"store_action" => {
    if let Some(ref action) = handler.r#ref {
        let action_fn = action.split('.').last().unwrap_or(action);
        format!("() {{ {}(); }}", action_fn)
    } else { "() {}".to_string() }
}
```

- [ ] **Step 2: Replace style skip (line 436-439)**

Map CSS properties from `el.props` named `"style"` to Flutter widget parameters. Parse JSON object value and convert known CSS keys (padding, margin, color, background-color, font-size, border-radius, etc.) to their Flutter equivalents (EdgeInsets, Color, TextStyle, BorderRadius).

- [ ] **Step 3: Run all Flutter tests, commit**

---

### React Task 9 + Flutter Task 8: Component composition (in parallel)

**Files:** Modify: `react.rs:810` (gen_view_node_tsx signature + Element handling) and `react.rs:48` (gen_component_tsx) | `flutter.rs:428` (format_element + format_view_tree signature chain)

For **React**: 
1. Build `HashMap<String, ComponentIR>` from `ir.all_components()` in `gen_component_tsx`
2. Thread `&component_map` into `gen_view_node_tsx` as a new parameter
3. At the top of the Element match arm, check if `el.tag` is in component_map
4. If yes, emit `<ComponentName prop1={val} onEvent={handler}>children</ComponentName>` JSX
5. If no, fall through to existing HTML element handling

For **Flutter**:
1. Same pattern: thread `&HashMap<String, ComponentIR>` through `format_view_tree` → `format_element` → `format_conditional` → `format_list_rendering`
2. In `format_element`, check component_map before `tag_to_flutter_widget`
3. If hit, build `ChildWidget(param1: val1, onEvent1: cb)` string

- [ ] **Step 1-4 each: implement, test, commit**

---

### Remaining P2/P3 Tasks (outline — same pattern as above)

**React**: Slot→`{props.children}` / named slot props, EventEmitStmtIR→`props.onEvent?.()`, sanitize_js_ident, CSS @keyframes for @motion defs, @leave/@animate stubs via framer-motion, @states state machine, Hero → framer-motion `layoutId`, SourceMap comment

**Flutter**: class:→decoration mapping, style:→CSS-to-Flutter mapping table (complete the parser from Task 7), Slot→widget placeholder, DrivenByBuilder class, @motion Timeline/CallbackOnly/Stagger, @leave/@animate triggers, prop_defaults, EventEmitStmtIR→widget callback, Hero flightShuttleBuilder, SourceMap comment

Each follows: 1) exact lines 2) before/after code 3) test 4) `cargo test` 5) commit.

---

_Plan: 9 React + 8 Flutter detailed tasks for P0/P1. P2/P3 same pattern._
