# OmniView Event Data Transmission — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all `e.detail` event payloads in OmniView codegen with compile-time-resolved, dead-field-eliminated, null-safe extraction of native event data across all 9 targets.

**Architecture:** IR-first design — `EventDataKind` enum + `event_data_schema()` define 20 event categories with field definitions. Two-pass lowering collects `used_fields` from inline and fn_ref handlers. Each codegen target has an adapter that maps `(kind, field)` → native extraction code. @for delegation is rewritten with dynamic event-type subscription and a JS kind-map dispatcher.

**Tech Stack:** Rust (IR + codegen), JavaScript embedded strings in codegen output, Dart for Flutter codegen, JSX for React codegen.

## Global Constraints

- All 362 existing tests must continue to pass
- Backward compatible: existing `.omni` handlers that don't use `e.field` are unaffected
- `onSubmit` auto-preventDefault behavior is preserved
- `used_fields: Option<Vec<String>>` — `None` means "all fields" (conservative), `Some(vec)` means exact set
- All extraction code uses optional chaining `?.` + `??` for null-safety
- ResizeEvent binds to `window`, not individual elements
- TouchEvent merges `touches` + `changedTouches`, deduplicated by identifier
- `onMouseMove` must be added to `map_event` in omniview.rs
- No `data-event-kind` HTML attributes — use JS map instead

---

## File Structure

| File | Purpose |
|------|---------|
| `omnilang/src/ir/mod.rs` | EventDataKind enum, EventFieldDef, schema/mapping functions, EventHandlerIR |
| `omnilang/src/semantic/mod.rs` | Two-pass lowering: collect e.field, validate, fn_ref backfill |
| `omnilang/src/semantic/lints.rs` | New lint rules for event data misuse |
| `omnilang/src/codegen/omniview.rs` | Web codegen — event extraction, map_event, @for rewrite, special bindings |
| `omnilang/src/codegen/react.rs` | React codegen — SyntheticEvent extraction |
| `omnilang/src/codegen/flutter.rs` | Flutter codegen — Dart event extraction + HardwareKeyboard |
| `omnilang/src/codegen/dioxus.rs` | Dioxus codegen — web_sys extraction |
| `omnilang/src/codegen/vue.rs` | Vue codegen — DOM extraction |
| `omnilang/src/codegen/javascript.rs` | JS codegen — DOM extraction |
| `test/projects/learn-omniview/src/components/EventsDemo.omni` | Updated test fixture exercising new event data fields |
| `test/projects/learn-omniview/src/pages/EventDataDemo.omni` | New page — comprehensive event data test |

---

### Task 1: IR Layer — EventDataKind, EventFieldDef, and Schema Functions

**Files:**
- Modify: `omnilang/src/ir/mod.rs` — add types + functions after the existing `EventHandlerIR` definition (~line 179)

**Interfaces:**
- Produces:
  ```rust
  pub enum EventDataKind { PointerEvent, TouchEvent, WheelEvent, KeyEvent, ... }  // 20 variants
  pub struct EventFieldDef { pub name: String, pub field_type: String }
  pub fn event_data_schema(kind: &EventDataKind) -> Vec<EventFieldDef>
  pub fn map_event_data_kind(event_name: &str) -> Option<EventDataKind>
  pub fn event_data_kind_label(kind: &EventDataKind) -> &'static str
  ```

- [ ] **Step 1: Add EventDataKind enum and EventFieldDef struct**

In `omnilang/src/ir/mod.rs`, after the `EventHandlerIR` struct (~after line 179), insert:

```rust
// ═══════════════════════════════════════════════════════
// Event Data Schema — cross-target event payload definitions
// ═══════════════════════════════════════════════════════

/// Event payload categories. Each variant has a fixed set of fields
/// defined in `event_data_schema()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventDataKind {
    PointerEvent,
    TouchEvent,
    WheelEvent,
    KeyEvent,
    ScrollEvent,
    InputEvent,
    ToggleEvent,
    SubmitEvent,
    FocusEvent,
    ClipboardEvent,
    DragEvent,
    DropEvent,
    ContextMenuEvent,
    SelectEvent,
    ResizeEvent,
    MediaEvent,
    MediaLoadEvent,
    MediaErrorEvent,
    AnimationEvent,
    TransitionEvent,
}

/// A single field within an event data payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFieldDef {
    pub name: String,
    /// "float" | "string" | "bool" | "int" | "{x,y,id}[]" | "string[]"
    pub field_type: String,
}

/// Full schema: kind -> ordered field definitions.
/// Used by both lowering (field validation) and codegen (extraction).
pub fn event_data_schema(kind: &EventDataKind) -> Vec<EventFieldDef> {
    let f = |name: &str, field_type: &str| EventFieldDef {
        name: name.to_string(),
        field_type: field_type.to_string(),
    };
    match kind {
        EventDataKind::PointerEvent => vec![
            f("x", "float"), f("y", "float"), f("button", "int"),
            f("pressure", "float"), f("tiltX", "float"), f("tiltY", "float"),
            f("pointerType", "string"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::TouchEvent => vec![
            f("touches", "{x,y,id}[]"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::WheelEvent => vec![
            f("x", "float"), f("y", "float"),
            f("deltaX", "float"), f("deltaY", "float"), f("deltaZ", "float"),
            f("deltaMode", "int"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::KeyEvent => vec![
            f("key", "string"), f("code", "string"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::ScrollEvent => vec![
            f("x", "float"), f("y", "float"),
        ],
        EventDataKind::InputEvent => vec![
            f("value", "string"), f("checked", "bool"),
        ],
        EventDataKind::ToggleEvent => vec![
            f("checked", "bool"),
        ],
        EventDataKind::SubmitEvent => vec![
            f("values", "{string: string}"),
        ],
        EventDataKind::FocusEvent => vec![
            f("value", "string"),
        ],
        EventDataKind::ClipboardEvent => vec![
            f("text", "string"),
        ],
        EventDataKind::DragEvent => vec![
            f("x", "float"), f("y", "float"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::DropEvent => vec![
            f("x", "float"), f("y", "float"),
            f("files", "string[]"), f("text", "string"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::ContextMenuEvent => vec![
            f("x", "float"), f("y", "float"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::SelectEvent => vec![
            f("start", "int"), f("end", "int"), f("text", "string"),
        ],
        EventDataKind::ResizeEvent => vec![
            f("width", "float"), f("height", "float"),
        ],
        EventDataKind::MediaEvent => vec![
            f("currentTime", "float"), f("duration", "float"),
        ],
        EventDataKind::MediaLoadEvent => vec![
            f("naturalWidth", "float"), f("naturalHeight", "float"),
        ],
        EventDataKind::MediaErrorEvent => vec![
            f("message", "string"),
        ],
        EventDataKind::AnimationEvent => vec![
            f("animationName", "string"), f("elapsedTime", "float"),
        ],
        EventDataKind::TransitionEvent => vec![
            f("propertyName", "string"), f("elapsedTime", "float"),
        ],
    }
}

/// Map .omni event name string → EventDataKind.
/// Returns None for store-only or non-data events.
pub fn map_event_data_kind(event_name: &str) -> Option<EventDataKind> {
    match event_name {
        "onClick" | "onPress" | "onTap" | "onDoubleClick" | "onLongPress"
        | "onMouseMove" | "onHover" | "onMouseEnter" | "onMouseLeave"
        | "onPointerDown" | "onPointerUp" | "onPointerMove" | "onPointerCancel"
            => Some(EventDataKind::PointerEvent),
        "onTouchStart" | "onTouchMove" | "onTouchEnd" | "onTouchCancel"
            => Some(EventDataKind::TouchEvent),
        "onWheel" => Some(EventDataKind::WheelEvent),
        "onKeyDown" | "onKeyUp" => Some(EventDataKind::KeyEvent),
        "onScroll" => Some(EventDataKind::ScrollEvent),
        "onChange" | "onInput" => Some(EventDataKind::InputEvent),
        "onToggle" => Some(EventDataKind::ToggleEvent),
        "onSubmit" => Some(EventDataKind::SubmitEvent),
        "onFocus" | "onBlur" => Some(EventDataKind::FocusEvent),
        "onCopy" | "onCut" | "onPaste" => Some(EventDataKind::ClipboardEvent),
        "onDragStart" | "onDrag" | "onDragEnd" => Some(EventDataKind::DragEvent),
        "onDrop" => Some(EventDataKind::DropEvent),
        "onContextMenu" => Some(EventDataKind::ContextMenuEvent),
        "onSelect" => Some(EventDataKind::SelectEvent),
        "onResize" => Some(EventDataKind::ResizeEvent),
        "onPlay" | "onPause" | "onEnded" => Some(EventDataKind::MediaEvent),
        "onLoad" => Some(EventDataKind::MediaLoadEvent),
        "onError" => Some(EventDataKind::MediaErrorEvent),
        "onAnimationStart" | "onAnimationEnd" | "onAnimationIteration"
            => Some(EventDataKind::AnimationEvent),
        "onTransitionEnd" => Some(EventDataKind::TransitionEvent),
        _ => None,
    }
}

/// Map EventDataKind to a string label for the @for JS kind-lookup map.
pub fn event_data_kind_label(kind: &EventDataKind) -> &'static str {
    match kind {
        EventDataKind::PointerEvent => "PointerEvent",
        EventDataKind::TouchEvent => "TouchEvent",
        EventDataKind::WheelEvent => "WheelEvent",
        EventDataKind::KeyEvent => "KeyEvent",
        EventDataKind::ScrollEvent => "ScrollEvent",
        EventDataKind::InputEvent => "InputEvent",
        EventDataKind::ToggleEvent => "ToggleEvent",
        EventDataKind::SubmitEvent => "SubmitEvent",
        EventDataKind::FocusEvent => "FocusEvent",
        EventDataKind::ClipboardEvent => "ClipboardEvent",
        EventDataKind::DragEvent => "DragEvent",
        EventDataKind::DropEvent => "DropEvent",
        EventDataKind::ContextMenuEvent => "ContextMenuEvent",
        EventDataKind::SelectEvent => "SelectEvent",
        EventDataKind::ResizeEvent => "ResizeEvent",
        EventDataKind::MediaEvent => "MediaEvent",
        EventDataKind::MediaLoadEvent => "MediaLoadEvent",
        EventDataKind::MediaErrorEvent => "MediaErrorEvent",
        EventDataKind::AnimationEvent => "AnimationEvent",
        EventDataKind::TransitionEvent => "TransitionEvent",
    }
}
```

- [ ] **Step 2: Write unit test for map_event_data_kind**

In the same file, add a test module (or locate the existing test module):

```rust
#[cfg(test)]
mod event_data_tests {
    use super::*;

    #[test]
    fn test_map_event_data_kind_pointer() {
        assert_eq!(map_event_data_kind("onClick"), Some(EventDataKind::PointerEvent));
        assert_eq!(map_event_data_kind("onMouseMove"), Some(EventDataKind::PointerEvent));
        assert_eq!(map_event_data_kind("onPointerDown"), Some(EventDataKind::PointerEvent));
    }

    #[test]
    fn test_map_event_data_kind_keyboard() {
        assert_eq!(map_event_data_kind("onKeyDown"), Some(EventDataKind::KeyEvent));
        assert_eq!(map_event_data_kind("onKeyUp"), Some(EventDataKind::KeyEvent));
    }

    #[test]
    fn test_map_event_data_kind_scroll() {
        assert_eq!(map_event_data_kind("onScroll"), Some(EventDataKind::ScrollEvent));
    }

    #[test]
    fn test_map_event_data_kind_none_for_unknown() {
        assert_eq!(map_event_data_kind("onUnknown"), None);
    }
}
```

- [ ] **Step 3: Write unit test for event_data_schema**

```rust
    #[test]
    fn test_schema_pointer_event_has_button_and_modifiers() {
        let fields = event_data_schema(&EventDataKind::PointerEvent);
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"x"));
        assert!(names.contains(&"y"));
        assert!(names.contains(&"button"));
        assert!(names.contains(&"alt"));
        assert!(names.contains(&"ctrl"));
    }

    #[test]
    fn test_schema_scroll_event_only_position() {
        let fields = event_data_schema(&EventDataKind::ScrollEvent);
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["x", "y"]);
    }

    #[test]
    fn test_schema_twenty_kinds() {
        // All 20 variants produce non-empty field lists
        let kinds = [
            EventDataKind::PointerEvent, EventDataKind::TouchEvent,
            EventDataKind::WheelEvent, EventDataKind::KeyEvent,
            EventDataKind::ScrollEvent, EventDataKind::InputEvent,
            EventDataKind::ToggleEvent, EventDataKind::SubmitEvent,
            EventDataKind::FocusEvent, EventDataKind::ClipboardEvent,
            EventDataKind::DragEvent, EventDataKind::DropEvent,
            EventDataKind::ContextMenuEvent, EventDataKind::SelectEvent,
            EventDataKind::ResizeEvent, EventDataKind::MediaEvent,
            EventDataKind::MediaLoadEvent, EventDataKind::MediaErrorEvent,
            EventDataKind::AnimationEvent, EventDataKind::TransitionEvent,
        ];
        assert_eq!(kinds.len(), 20);
        for kind in &kinds {
            assert!(!event_data_schema(kind).is_empty(),
                "Schema empty for {:?}", kind);
        }
    }
```

- [ ] **Step 4: Run tests to verify**

```bash
cargo test --package omnilang event_data_tests
```
Expected: all 6 new tests pass.

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/ir/mod.rs
git commit -m "feat(ir): add EventDataKind, EventFieldDef, and schema functions for 20 event categories"
```

---

### Task 2: Update EventHandlerIR with Event Data Fields

**Files:**
- Modify: `omnilang/src/ir/mod.rs` — update `EventHandlerIR` struct (~line 173)

**Interfaces:**
- Consumes: `EventDataKind` from Task 1
- Produces: `EventHandlerIR` with `event_data`, `used_fields`, `needs_prevent_default`, `needs_stop_propagation`

- [ ] **Step 1: Add new fields to EventHandlerIR**

Replace the existing `EventHandlerIR` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHandlerIR {
    /// "fn_ref" | "inline_fn" | "store_action"
    pub kind: String,
    /// Inline handler body (None for fn_ref, store_action)
    pub body: Option<BlockIR>,
    /// Function name (fn_ref) or action path (store_action)
    pub r#ref: Option<String>,
    /// Inline handler declared parameter names
    #[serde(default)]
    pub params: Vec<String>,
    // ── Event data fields (new) ──
    /// Event payload category. None for store_action handlers.
    #[serde(default)]
    pub event_data: Option<EventDataKind>,
    /// Fields of `e` actually referenced in the handler body.
    /// None = all fields (conservative, for unresolved fn_ref).
    /// Some(vec) = exact set; empty vec = no fields used (pass `{}`).
    #[serde(default)]
    pub used_fields: Option<Vec<String>>,
    /// Handler explicitly calls e.preventDefault()
    #[serde(default)]
    pub needs_prevent_default: bool,
    /// Handler explicitly calls e.stopPropagation()
    #[serde(default)]
    pub needs_stop_propagation: bool,
}
```

- [ ] **Step 2: Verify existing serialization tests still pass**

```bash
cargo test --package omnilang
```
Expected: 362+ tests pass (same or more than baseline).

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/ir/mod.rs
git commit -m "feat(ir): add event_data, used_fields, needs_prevent_default, needs_stop_propagation to EventHandlerIR"
```

---

### Task 3: Semantic Lowering — Pass 1 (Inline Handlers)

**Files:**
- Modify: `omnilang/src/semantic/mod.rs` — locate the `ElementProp::Event` lowering arm (~line 1302)

**Interfaces:**
- Consumes: `map_event_data_kind`, `event_data_schema`, `EventDataKind` from Task 1
- Consumes: `EventHandlerIR` with new fields from Task 2
- Produces: lowered `EventHandlerIR` with `event_data`, `used_fields`, `needs_prevent_default` populated for inline_fn handlers

- [ ] **Step 1: Locate the event lowering code**

Read the current lowering of `ElementProp::Event`:

```bash
# Find the exact location
grep -n "ElementProp::Event" omnilang/src/semantic/mod.rs
```

The existing code at this location creates `EventHandlerIR` with `kind`, `body`, `r#ref`, `params`. We need to add logic after this creation to populate the new fields.

- [ ] **Step 2: Add helper function — walk handler body for e.field references**

In `semantic/mod.rs`, add a helper function:

```rust
/// Walk an AST Block to collect all `e.field` accesses and detect
/// `e.preventDefault()` / `e.stopPropagation()` calls.
struct EventFieldCollector {
    fields: Vec<String>,
    needs_prevent_default: bool,
    needs_stop_propagation: bool,
}

impl EventFieldCollector {
    fn collect(block: &crate::parser::ast::Block) -> Self {
        let mut c = Self { fields: vec![], needs_prevent_default: false, needs_stop_propagation: false };
        for stmt in &block.statements {
            c.visit_stmt(stmt);
        }
        c
    }

    fn visit_stmt(&mut self, stmt: &crate::parser::ast::Stmt) {
        match stmt {
            crate::parser::ast::Stmt::Expr(expr) | crate::parser::ast::Stmt::Semi(expr) => {
                self.visit_expr(expr);
            }
            crate::parser::ast::Stmt::Assign { target, value, .. } => {
                // `e.preventDefault()` or `e.stopPropagation()` in a Stmt
                self.visit_expr(value);
                // target could reference `e` indirectly but ignore for now
            }
            crate::parser::ast::Stmt::If { condition, then_block, else_block, .. } => {
                self.visit_expr(condition);
                self.visit_block(then_block);
                if let Some(else_b) = else_block {
                    self.visit_stmt(else_b);
                }
            }
            crate::parser::ast::Stmt::For { body, .. } => {
                self.visit_block(body);
            }
            crate::parser::ast::Stmt::Block(block) => {
                self.visit_block(block);
            }
            _ => {}
        }
    }

    fn visit_block(&mut self, block: &crate::parser::ast::Block) {
        for stmt in &block.statements {
            self.visit_stmt(stmt);
        }
    }

    fn visit_expr(&mut self, expr: &crate::parser::ast::Expr) {
        match expr {
            // e.y → FieldAccess { object: VarRef("e"), field: "y" }
            crate::parser::ast::Expr::FieldAccess { object, field, .. } => {
                match object.as_ref() {
                    crate::parser::ast::Expr::VarRef(name, _) if name == "e" => {
                        if !self.fields.contains(field) {
                            self.fields.push(field.clone());
                        }
                    }
                    _ => {
                        // Nested: base.field.subfield — walk base
                        self.visit_expr(object);
                    }
                }
            }
            // e.method(args)
            crate::parser::ast::Expr::MethodCall { object, method, args, .. } => {
                match object.as_ref() {
                    crate::parser::ast::Expr::VarRef(name, _) if name == "e" => {
                        match method.as_str() {
                            "preventDefault" => self.needs_prevent_default = true,
                            "stopPropagation" => self.needs_stop_propagation = true,
                            _ => {}
                        }
                    }
                    _ => self.visit_expr(object),
                }
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            // Recursively walk other expression types
            crate::parser::ast::Expr::BinaryOp { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            crate::parser::ast::Expr::UnaryOp { operand, .. } => {
                self.visit_expr(operand);
            }
            crate::parser::ast::Expr::FuncCall { func, args, .. } => {
                self.visit_expr(func);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            crate::parser::ast::Expr::IfExpr { condition, then_expr, else_expr, .. } => {
                self.visit_expr(condition);
                self.visit_expr(then_expr);
                self.visit_expr(else_expr);
            }
            crate::parser::ast::Expr::TernaryExpr { condition, then_expr, else_expr, .. } => {
                self.visit_expr(condition);
                self.visit_expr(then_expr);
                self.visit_expr(else_expr);
            }
            _ => {}
        }
    }
}
```

- [ ] **Step 3: Integrate into EventProp::Event lowering**

At the lowering site for `ElementProp::Event` (~line 1303), after creating the `ir_handler`:

```rust
// After the existing ir_handler creation:
ElementProp::Event { name, handler } => {
    let ir_handler = match handler { /* ... existing match ... */ };

    // ── NEW: Populate event data fields ──
    let mut event_data = None;
    let mut used_fields = None;
    let mut needs_pd = false;
    let mut needs_sp = false;

    // Always preventDefault for onSubmit (backward compat)
    if name == "onSubmit" {
        needs_pd = true;
    }

    // Determine event data kind
    if let Some(kind) = map_event_data_kind(&name) {
        event_data = Some(kind.clone());

        match handler {
            EventHandler::InlineFn { ref body, .. } => {
                let schema = event_data_schema(&kind);
                let valid_names: Vec<&str> = schema.iter().map(|f| f.name.as_str()).collect();
                let collector = EventFieldCollector::collect(body);

                // Validate fields
                for field in &collector.fields {
                    if !valid_names.contains(&field.as_str()) {
                        diags.push(Diagnostic::error(format!(
                            "`e.{}` is not a field of {:?}. Available fields: {:?}",
                            field, kind, valid_names
                        )));
                    }
                }

                used_fields = Some(collector.fields);
                needs_pd = needs_pd || collector.needs_prevent_default;
                needs_sp = collector.needs_stop_propagation;
            }
            EventHandler::FnRef(_) => {
                // Defer to Pass 2 — used_fields stays None
                needs_pd = needs_pd; // onSubmit already set
            }
            EventHandler::StoreAction { .. } => {
                // Store actions get no event data
                event_data = None;
            }
        }
    }

    let ir_handler = EventHandlerIR {
        kind: ir_handler.kind,
        body: ir_handler.body,
        r#ref: ir_handler.r#ref,
        params: ir_handler.params,
        event_data,
        used_fields,
        needs_prevent_default: needs_pd,
        needs_stop_propagation: needs_sp,
    };
    // ... insert into events map as before
}
```

**Important:** The `Diagnostic::error` path and `diags` variable may need adjustment depending on the function signature at this site. Read the surrounding code to determine the correct diagnostic emission pattern.

- [ ] **Step 4: Run tests**

```bash
cargo test --package omnilang
```
Expected: 362+ tests pass. If any test fails due to new warnings/errors in the test fixtures, note them — they'll be fixed in Task 12.

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/semantic/mod.rs
git commit -m "feat(semantic): Pass 1 lowering — collect e.field references and validate against EventDataKind schema"
```

---

### Task 4: Semantic Lowering — Pass 2 (FnRef Backfill)

**Files:**
- Modify: `omnilang/src/semantic/mod.rs` — add a post-processing pass after all component functions are lowered

**Interfaces:**
- Consumes: `EventHandlerIR.used_fields == None` (unresolved fn_ref) from Task 3
- Consumes: lowered function `BlockIR` bodies from the component
- Produces: `used_fields: Some(vec)` for resolved fn_ref handlers

- [ ] **Step 1: Add the backfill pass function**

```rust
/// Pass 2: backfill used_fields for fn_ref handlers by walking the
/// referenced function's IR body.
fn backfill_fn_ref_event_fields(
    component_events: &mut HashMap<String, HashMap<String, EventHandlerIR>>,
    component_functions: &HashMap<String, super::ir::FunctionIR>,
    event_data_lookup: &HashMap<String, EventDataKind>, // event_name → kind, collected during Pass 1
) {
    for (_el_id, events) in component_events.iter_mut() {
        for (event_name, handler) in events.iter_mut() {
            if handler.used_fields.is_some() || handler.event_data.is_none() {
                continue; // Already resolved, or store_action
            }
            let kind = handler.event_data.as_ref().unwrap();

            if let Some(ref fn_name) = handler.r#ref {
                // Resolve: fn_name may be "ComponentName_fnName" after prefixed rename
                // Try the full name first, then strip component prefix
                let resolved = component_functions.get(fn_name)
                    .or_else(|| {
                        // Try stripping known component prefixes
                        component_functions.iter()
                            .find(|(k, _)| k.ends_with(&format!("_{}", fn_name)) || k == fn_name)
                            .map(|(_, v)| v)
                    });

                if let Some(func_ir) = resolved {
                    let schema = event_data_schema(kind);
                    let valid_names: Vec<&str> = schema.iter().map(|f| f.name.as_str()).collect();
                    let mut used = Vec::new();

                    // Walk the function body IR for FieldAccessExpr(e, field)
                    if let Some(ref body) = func_ir.body {
                        collect_ir_field_accesses(body, &mut used);
                    }

                    // Filter to valid event data fields
                    let used: Vec<String> = used.into_iter()
                        .filter(|f| valid_names.contains(&f.as_str()))
                        .collect();
                    handler.used_fields = Some(used);
                }
                // If not found: leave used_fields as None (conservative)
            }
        }
    }
}

/// Recursively walk BlockIR statements for FieldAccessExpr where object is VarRefExpr("e").
fn collect_ir_field_accesses(block: &BlockIR, fields: &mut Vec<String>) {
    for stmt in &block.statements {
        match stmt {
            StatementIR::ExpressionStmt(expr) | StatementIR::Semi(expr) => {
                collect_expr_fields(expr, fields);
            }
            StatementIR::IfStmtIR { condition, then_block, else_ifs, else_block, .. } => {
                collect_expr_fields(condition, fields);
                collect_ir_field_accesses(then_block, fields);
                for (_, block) in else_ifs {
                    collect_ir_field_accesses(block, fields);
                }
                if let Some(block) = else_block {
                    collect_ir_field_accesses(block, fields);
                }
            }
            StatementIR::LetStmtIR { value: Some(expr), .. } => {
                collect_expr_fields(expr, fields);
            }
            StatementIR::AssignStmt { target, value, .. } => {
                collect_expr_fields(target, fields);
                collect_expr_fields(value, fields);
            }
            _ => {}
        }
    }
}

fn collect_expr_fields(expr: &ExprIR, fields: &mut Vec<String>) {
    match expr {
        ExprIR::FieldAccessExpr { object, field } => {
            if let box ExprIR::VarRefExpr(name) = object {
                if name == "e" && !fields.contains(field) {
                    fields.push(field.clone());
                }
            }
            collect_expr_fields(object, fields);
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            collect_expr_fields(object, fields);
            for arg in args { collect_expr_fields(arg, fields); }
        }
        ExprIR::FuncCallExpr { args, .. } => {
            for arg in args { collect_expr_fields(arg, fields); }
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            collect_expr_fields(left, fields);
            collect_expr_fields(right, fields);
        }
        ExprIR::UnaryOpExpr { operand, .. } => {
            collect_expr_fields(operand, fields);
        }
        ExprIR::IfExprIR { cond, then_expr, else_expr, .. } => {
            collect_expr_fields(cond, fields);
            collect_expr_fields(then_expr, fields);
            collect_expr_fields(else_expr, fields);
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Call backfill at the right point in the lowering pipeline**

Find where all component functions have been lowered (likely after the component lowering loop in `semantic/mod.rs`). Add:

```rust
// After all components lowered:
backfill_fn_ref_event_fields(&mut component_events, &component_functions, &event_data_lookup);
```

**Note:** The exact variable names (`component_events`, `component_functions`) depend on the lowering function's internal structure. Read the containing function to determine the correct variable names. If `component_events` is not a separate map (events live on `ElementIR`), iterate over all elements in all components.

- [ ] **Step 3: Run tests**

```bash
cargo test --package omnilang
```
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/semantic/mod.rs
git commit -m "feat(semantic): Pass 2 — backfill used_fields for fn_ref event handlers"
```

---

### Task 5: New Lint Rules

**Files:**
- Modify: `omnilang/src/semantic/lints.rs` — add new lint functions

**Interfaces:**
- Produces: three new lint rules

- [ ] **Step 1: Add lint — store action accessing event data**

```rust
/// LINT: Store actions cannot receive event data.
/// If a store action path references `e` (event data), it's a compile error.
pub fn check_store_action_no_event_data(
    events: &HashMap<String, crate::ir::EventHandlerIR>,
    diags: &mut Vec<Diagnostic>,
) {
    for (event_name, handler) in events {
        if handler.kind == "store_action" {
            if let Some(ref path) = handler.r#ref {
                // Check if the path references `e` anywhere
                if path.contains("e.") || path == "e" {
                    diags.push(Diagnostic::error(format!(
                        "Store action `{}` cannot access event data. \
                         Store actions receive no arguments. \
                         Use an inline handler or fn_ref instead for `{}`.",
                        path, event_name
                    )));
                }
            }
        }
    }
}
```

- [ ] **Step 2: Add lint — platform compatibility warning**

```rust
/// LINT: Warn when an event has no equivalent on the current target platform.
pub fn check_event_platform_compatibility(
    events: &HashMap<String, crate::ir::EventHandlerIR>,
    target: crate::Target,
    diags: &mut Vec<Diagnostic>,
) {
    for (event_name, _handler) in events {
        let warning = match (target, event_name.as_str()) {
            (crate::Target::Flutter, "onAnimationStart" | "onAnimationEnd"
                | "onAnimationIteration" | "onTransitionEnd") => Some(format!(
                "`{}` has no native Flutter equivalent. \
                 Consider using AnimationController.addStatusListener instead.",
                event_name
            )),
            _ => None,
        };
        if let Some(msg) = warning {
            diags.push(Diagnostic::warning(msg));
        }
    }
}
```

- [ ] **Step 3: Wire lints into the existing linting pass**

Find the lint entry point (likely a function like `run_all_lints` or similar) and add calls:

```rust
// In the lint execution function:
for component in &ir.components {
    for el in &component.template.elements {
        check_store_action_no_event_data(&el.events, diags);
        check_event_platform_compatibility(&el.events, target, diags);
    }
}
```

**Note:** The exact iteration patterns depend on the lints.rs structure. Match the existing pattern.

- [ ] **Step 4: Run tests**

```bash
cargo test --package omnilang
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/semantic/lints.rs
git commit -m "feat(lints): add store_action_no_event_data and platform_compatibility lints"
```

---

### Task 6: Codegen — Event Extraction Adapter (omniview.rs)

**Files:**
- Modify: `omnilang/src/codegen/omniview.rs` — add extraction helper, update all 5 `e.detail` sites

**Interfaces:**
- Consumes: `EventHandlerIR.event_data`, `EventHandlerIR.used_fields`, `event_data_schema()`
- Produces: `emit_event_extract()` function; updated JS emission at 5 call sites

- [ ] **Step 1: Add field extraction mapping function**

In `omnilang/src/codegen/omniview.rs`, add a helper that maps `(EventDataKind, field_name, raw_var_name)` → JS extraction code with null-safe fallback:

```rust
/// Emit JS extraction code for a single event data field.
/// Returns (extraction_expression, fallback_value_literal).
fn field_extract_js(kind: &EventDataKind, field: &str, raw: &str) -> (String, String) {
    match (kind, field) {
        // PointerEvent
        (EventDataKind::PointerEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::PointerEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::PointerEvent, "button") => (format!("{}.button", raw), "0".into()),
        (EventDataKind::PointerEvent, "pressure") => (format!("{}.pressure", raw), "0".into()),
        (EventDataKind::PointerEvent, "tiltX") => (format!("{}.tiltX", raw), "0".into()),
        (EventDataKind::PointerEvent, "tiltY") => (format!("{}.tiltY", raw), "0".into()),
        (EventDataKind::PointerEvent, "pointerType") => (format!("{}.pointerType", raw), "\"\"".into()),
        (EventDataKind::PointerEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // TouchEvent
        (EventDataKind::TouchEvent, "touches") => (
            format!("__mergeTouches({})", raw),
            "[]".into()
        ),
        (EventDataKind::TouchEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // WheelEvent
        (EventDataKind::WheelEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::WheelEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaX") => (format!("{}.deltaX", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaY") => (format!("{}.deltaY", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaZ") => (format!("{}.deltaZ", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaMode") => (format!("{}.deltaMode", raw), "0".into()),
        (EventDataKind::WheelEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // KeyEvent
        (EventDataKind::KeyEvent, "key") => (format!("{}.key", raw), "\"\"".into()),
        (EventDataKind::KeyEvent, "code") => (format!("{}.code", raw), "\"\"".into()),
        (EventDataKind::KeyEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // ScrollEvent
        (EventDataKind::ScrollEvent, "x") => (format!("{}.target?.scrollLeft", raw), "0".into()),
        (EventDataKind::ScrollEvent, "y") => (format!("{}.target?.scrollTop", raw), "0".into()),

        // InputEvent
        (EventDataKind::InputEvent, "value") => (format!("{}.target?.value", raw), "\"\"".into()),
        (EventDataKind::InputEvent, "checked") => (format!("{}.target?.checked", raw), "false".into()),

        // ToggleEvent
        (EventDataKind::ToggleEvent, "checked") => (format!("{}.target?.checked", raw), "false".into()),

        // SubmitEvent — Web
        (EventDataKind::SubmitEvent, "values") => (
            format!("Object.fromEntries(new FormData({}.target))", raw),
            "{}".into()
        ),

        // FocusEvent
        (EventDataKind::FocusEvent, "value") => (format!("{}.target?.value", raw), "\"\"".into()),

        // ClipboardEvent
        (EventDataKind::ClipboardEvent, "text") => (
            format!("(function(){{try{{return {}.clipboardData?.getData('text')||''}}catch(_){{return''}}}})()", raw),
            "\"\"".into()
        ),

        // DragEvent
        (EventDataKind::DragEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::DragEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::DragEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // DropEvent
        (EventDataKind::DropEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::DropEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::DropEvent, "files") => (
            format!("[...({}.dataTransfer?.files ?? [])].map(f => f.name)", raw),
            "[]".into()
        ),
        (EventDataKind::DropEvent, "text") => (
            format!("(function(){{try{{return {}.dataTransfer?.getData('text')||''}}catch(_){{return''}}}})()", raw),
            "\"\"".into()
        ),
        (EventDataKind::DropEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // ContextMenuEvent
        (EventDataKind::ContextMenuEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::ContextMenuEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::ContextMenuEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // SelectEvent
        (EventDataKind::SelectEvent, "start") => (format!("{}.target?.selectionStart", raw), "0".into()),
        (EventDataKind::SelectEvent, "end") => (format!("{}.target?.selectionEnd", raw), "0".into()),
        (EventDataKind::SelectEvent, "text") => (
            format!("({}.target?.value ?? '').substring({}.target?.selectionStart ?? 0, {}.target?.selectionEnd ?? 0)", raw, raw, raw),
            "\"\"".into()
        ),

        // ResizeEvent
        (EventDataKind::ResizeEvent, "width") => ("window.innerWidth".into(), "0".into()),
        (EventDataKind::ResizeEvent, "height") => ("window.innerHeight".into(), "0".into()),

        // MediaEvent
        (EventDataKind::MediaEvent, "currentTime") => (format!("{}.target?.currentTime", raw), "0".into()),
        (EventDataKind::MediaEvent, "duration") => (format!("{}.target?.duration", raw), "0".into()),

        // MediaLoadEvent
        (EventDataKind::MediaLoadEvent, "naturalWidth") => (format!("{}.target?.naturalWidth", raw), "0".into()),
        (EventDataKind::MediaLoadEvent, "naturalHeight") => (format!("{}.target?.naturalHeight", raw), "0".into()),

        // MediaErrorEvent
        (EventDataKind::MediaErrorEvent, "message") => (format!("{}.target?.error?.message", raw), "\"unknown\"".into()),

        // AnimationEvent
        (EventDataKind::AnimationEvent, "animationName") => (format!("{}.animationName", raw), "\"\"".into()),
        (EventDataKind::AnimationEvent, "elapsedTime") => (format!("{}.elapsedTime", raw), "0".into()),

        // TransitionEvent
        (EventDataKind::TransitionEvent, "propertyName") => (format!("{}.propertyName", raw), "\"\"".into()),
        (EventDataKind::TransitionEvent, "elapsedTime") => (format!("{}.elapsedTime", raw), "0".into()),

        _ => (format!("undefined /* unknown field {} for {:?} */", field, kind), "undefined".into()),
    }
}
```

- [ ] **Step 2: Add emit_event_object helper**

```rust
/// Emit a JS object literal for event data extraction from a raw event variable.
/// Only emits fields in `used_fields` (or all fields from schema if None).
fn emit_event_object(
    event_data: &Option<EventDataKind>,
    used_fields: &Option<Vec<String>>,
    raw_var: &str,
) -> String {
    let kind = match event_data {
        Some(k) => k,
        None => return "{}".to_string(),
    };

    let schema = event_data_schema(kind);
    let fields: Vec<String> = match used_fields {
        Some(fields) if !fields.is_empty() => {
            // Only emit used fields, in schema order
            schema.iter()
                .filter(|f| fields.contains(&f.name))
                .map(|f| {
                    let (extract, fallback) = field_extract_js(kind, &f.name, raw_var);
                    format!("{}: {} ?? {}", f.name, extract, fallback)
                })
                .collect()
        }
        Some(_) => {
            // Empty vec — no fields used
            return "{}".to_string();
        }
        None => {
            // Conservative — emit all fields
            schema.iter()
                .map(|f| {
                    let (extract, fallback) = field_extract_js(kind, &f.name, raw_var);
                    format!("{}: {} ?? {}", f.name, extract, fallback)
                })
                .collect()
        }
    };

    format!("{{ {} }}", fields.join(", "))
}
```

- [ ] **Step 3: Update fn_ref direct binding site (~lines 1930-1937 and 2254-2260)**

Replace the `e.detail` pattern at both fn_ref sites:

**Before:**
```rust
self.js.push_str(&format!(
    "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{ e.stopPropagation(); {} {}(e.detail); }}); }}\n",
    nid, js_event, pd, prefixed
));
```

**After:**
```rust
let event_obj = emit_event_object(&handler.event_data, &handler.used_fields, "e");
self.js.push_str(&format!(
    "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ e.stopPropagation(); {pd} {prefixed}({event_obj}); }}); }}\n",
    nid = nid, js_event = js_event, pd = pd, prefixed = prefixed, event_obj = event_obj
));
```

Do this at BOTH fn_ref call sites (the non-@for path and the component-event path).

- [ ] **Step 4: Update inline_fn direct binding site (~lines 1962-1966 and 2282-2286)**

**Before:**
```rust
self.js.push_str(&format!(
    "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{ e.stopPropagation(); {} {}(e.detail); }}); }}\n",
    nid, js_event, pd, wrapper_name
));
```

**After:**
```rust
let event_obj = emit_event_object(&handler.event_data, &handler.used_fields, "e");
self.js.push_str(&format!(
    "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ e.stopPropagation(); {pd} {wrapper_name}({event_obj}); }}); }}\n",
    nid = nid, js_event = js_event, pd = pd, wrapper_name = wrapper_name, event_obj = event_obj
));
```

- [ ] **Step 5: Add onMouseMove to map_event**

```rust
"onMouseMove" => "mousemove".into(),
```

- [ ] **Step 6: Fix @for delegation e.detail → extractEvent (~line 1738)**

Replace:
```rust
self.js.push_str("        if (typeof fn === 'function') fn(e.detail);\n");
```
With:
```rust
self.js.push_str("        if (typeof fn === 'function') fn(_extractEvent(_eventKinds[e.type] || '', e));\n");
```

- [ ] **Step 7: Run tests**

```bash
cargo test --package omnilang
```

Expected: tests pass. Codegen snapshot tests may need updating — if any exist, update them.

- [ ] **Step 8: Commit**

```bash
git add omnilang/src/codegen/omniview.rs
git commit -m "feat(omniview): replace e.detail with compile-time field extraction, add event_object emit"
```

---

### Task 7: Codegen — @for Event Delegation Rewrite (omniview.rs)

**Files:**
- Modify: `omnilang/src/codegen/omniview.rs` — @for handler generation (~lines 1700-1770)

- [ ] **Step 1: Collect all event types used in @for template**

Before the @for JS emission, scan all elements in the @for body for events:

```rust
// Collect all unique DOM event types used in this @for template
let mut for_event_types: Vec<String> = Vec::new();
let mut for_event_kinds: Vec<(String, EventDataKind)> = Vec::new();

// Walk template elements recursively to find event handlers
fn collect_for_events(
    tree: &ViewTree,
    event_types: &mut Vec<String>,
    event_kinds: &mut Vec<(String, EventDataKind)>,
) {
    match tree {
        ViewTree::Element(el) => {
            for (event_name, handler) in &el.events {
                let js_event = map_event(event_name);
                if !event_types.contains(&js_event) {
                    event_types.push(js_event.clone());
                    if let Some(ref kind) = handler.event_data {
                        event_kinds.push((js_event.clone(), kind.clone()));
                    }
                }
            }
            for child in &el.children {
                collect_for_events(child, event_types, event_kinds);
            }
        }
        ViewTree::If { then_branch, else_branch, .. } => {
            for node in then_branch { collect_for_events(node, event_types, event_kinds); }
            if let Some(nodes) = else_branch { for node in nodes { collect_for_events(node, event_types, event_kinds); } }
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Emit _eventKinds JS map and dynamic subscriptions**

Replace the static subscription list (lines 1762-1768) with:

```rust
// Emit JS event-kind lookup map (only for types present)
if !for_event_kinds.is_empty() {
    self.js.push_str("    const _eventKinds = {\n");
    for (js_event, kind) in &for_event_kinds {
        self.js.push_str(&format!(
            "      '{}': '{}',\n",
            js_event,
            event_data_kind_label(kind)
        ));
    }
    self.js.push_str("    };\n");

    // Dynamic subscription
    self.js.push_str("    [");
    for (i, evt) in for_event_types.iter().enumerate() {
        if i > 0 { self.js.push_str(", "); }
        self.js.push_str(&format!("'{}'", evt));
    }
    self.js.push_str("].forEach(function(evt) {\n");
    self.js.push_str("      parent.addEventListener(evt, _forHandle);\n");
    self.js.push_str("    });\n");
}
```

- [ ] **Step 3: Emit _extractEvent switch function**

Emit a JS function that dispatches extraction by kind label:

```rust
if !for_event_kinds.is_empty() {
    self.js.push_str("    function _extractEvent(kind, e) {\n");
    self.js.push_str("      switch (kind) {\n");
    for (_, kind) in &for_event_kinds {
        let label = event_data_kind_label(kind);
        self.js.push_str(&format!("        case '{}':\n", label));
        self.js.push_str(&format!(
            "          return {};\n",
            emit_event_object(&Some(kind.clone()), &None, "e") // None = all fields for @for
        ));
    }
    self.js.push_str("        default: return {};\n");
    self.js.push_str("      }\n");
    self.js.push_str("    }\n");
}
```

- [ ] **Step 4: Emit global _mergeTouches helper (once per compilation unit)**

If TouchEvent appears anywhere in the @for (or anywhere in the IR), emit once:

```rust
// Check if TouchEvent is needed globally
let needs_merge_touches = /* scan IR for any TouchEvent usage */;
if needs_merge_touches {
    self.js.push_str(
        "function __mergeTouches(e) {\n\
         \x20 const seen = new Map();\n\
         \x20 for (const t of e.touches)\n\
         \x20   seen.set(t.identifier, { x: t.clientX, y: t.clientY, id: t.identifier });\n\
         \x20 for (const t of e.changedTouches)\n\
         \x20   if (!seen.has(t.identifier))\n\
         \x20     seen.set(t.identifier, { x: t.clientX, y: t.clientY, id: t.identifier });\n\
         \x20 return [...seen.values()];\n\
         }\n"
    );
}
```

For the initial implementation, emit `__mergeTouches` unconditionally when TouchEvent is used. Track via a field on the generator struct: `needs_touch_merge: bool`.

- [ ] **Step 5: Run tests**

```bash
cargo test --package omnilang
```

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview.rs
git commit -m "feat(omniview): rewrite @for event delegation with dynamic subscriptions and _extractEvent"
```

---

### Task 8: Codegen — ResizeEvent and SubmitEvent Special Cases (omniview.rs)

**Files:**
- Modify: `omnilang/src/codegen/omniview.rs`

- [ ] **Step 1: ResizeEvent binds to window**

In the event binding emission, check if the event is ResizeEvent:

```rust
if let Some(EventDataKind::ResizeEvent) = handler.event_data {
    self.js.push_str(&format!(
        "{{ window.addEventListener(\"resize\", (_e) => {{ {} }}); }}\n",
        format!("{}({})", handler_name, emit_event_object(&handler.event_data, &handler.used_fields, "_e"))
    ));
    continue; // Skip normal el.addEventListener
}
```

- [ ] **Step 2: SubmitEvent emit values (not value)**

In `field_extract_js`, the SubmitEvent "values" field already maps to `Object.fromEntries(new FormData(_e.target))`.  Verify this code path is reached by ensuring the `event_data_schema` for SubmitEvent has `values` not `value`.

- [ ] **Step 3: Handle preventDefault emission**

In the event binding emission, before the handler call:

```rust
let pd_code = if handler.needs_prevent_default || js_event == "submit" {
    "_e.preventDefault(); "
} else {
    ""
};
```

And use `pd_code` instead of the current `pd` variable in the template.

- [ ] **Step 4: Run tests and commit**

```bash
cargo test --package omnilang
git add omnilang/src/codegen/omniview.rs
git commit -m "feat(omniview): ResizeEvent binds to window, preventDefault from IR flag"
```

---

### Task 9: Codegen — React (react.rs)

**Files:**
- Modify: `omnilang/src/codegen/react.rs`

- [ ] **Step 1: Add React event extraction helper**

```rust
/// React target field extraction — mostly identical to Web but uses
/// SyntheticEvent conventions (no optional chaining needed on target).
fn field_extract_react(kind: &EventDataKind, field: &str, raw: &str) -> String {
    // React SyntheticEvent guarantees e.target exists during the event
    match (kind, field) {
        (EventDataKind::PointerEvent, "x") => format!("{}.clientX", raw),
        (EventDataKind::PointerEvent, "y") => format!("{}.clientY", raw),
        (EventDataKind::PointerEvent, "button") => format!("{}.button", raw),
        // ... (same mapping as Web but without ?. on target)
        (EventDataKind::ScrollEvent, "x") => format!("{}.target.scrollLeft", raw),
        (EventDataKind::ScrollEvent, "y") => format!("{}.target.scrollTop", raw),
        (EventDataKind::KeyEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            format!("{}.{}", raw, key)
        }
        // ... (complete all 20 kinds)
        _ => format!("undefined /* {} */", field),
    }
}
```

- [ ] **Step 2: Update JSX event handler emission**

Find the React event emission site (search for `to_react_event` or `onClick`) and update:

```jsx
// Before:
<div onClick={() => handler()} />

// After:
<div onScroll={(e) => handleScroll({ y: e.target.scrollTop })} />
```

The generated JSX should pass the extracted event object.

- [ ] **Step 3: Run tests and commit**

```bash
cargo test --package omnilang
git add omnilang/src/codegen/react.rs
git commit -m "feat(react): add event data extraction for React SyntheticEvent"
```

---

### Task 10: Codegen — Flutter (flutter.rs)

**Files:**
- Modify: `omnilang/src/codegen/flutter.rs`

- [ ] **Step 1: Add Flutter field extraction**

```rust
fn field_extract_flutter(kind: &EventDataKind, field: &str, raw: &str) -> String {
    match (kind, field) {
        (EventDataKind::PointerEvent, "x") => format!("{}.localPosition.dx", raw),
        (EventDataKind::PointerEvent, "y") => format!("{}.localPosition.dy", raw),
        (EventDataKind::PointerEvent, "button") => {
            // buttons is a bitmask: 0x1=primary, 0x2=secondary, 0x4=middle
            format!("({}.buttons & 0x1 != 0 ? 0 : {}.buttons & 0x2 != 0 ? 2 : {}.buttons & 0x4 != 0 ? 1 : 0)", raw, raw, raw)
        },
        (EventDataKind::PointerEvent, "pressure") => format!("{}.pressure", raw),
        (EventDataKind::PointerEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let prop = match m { "alt" => "isAltPressed", "ctrl" => "isControlPressed", "shift" => "isShiftPressed", _ => "isMetaPressed" };
            format!("HardwareKeyboard.instance.{}", prop)
        },
        (EventDataKind::ScrollEvent, "x") => format!("{}.metrics.pixels", raw),
        (EventDataKind::ScrollEvent, "y") => format!("{}.metrics.pixels", raw),
        (EventDataKind::KeyEvent, "key") => format!("{}.logicalKey.keyLabel", raw),
        (EventDataKind::KeyEvent, "code") => format!("{}.logicalKey.debugName", raw),
        (EventDataKind::KeyEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let prop = match m { "alt" => "isAltPressed", "ctrl" => "isControlPressed", "shift" => "isShiftPressed", _ => "isMetaPressed" };
            format!("HardwareKeyboard.instance.{}", prop)
        },
        // ... (complete all applicable kinds)
        _ => format!("null /* {} */", field),
    }
}
```

- [ ] **Step 2: Update Flutter event callbacks**

Find event handler emission in `flutter.rs` (search for `event_to_flutter_callback`) and replace with extraction-based emission.

- [ ] **Step 3: Run tests and commit**

```bash
cargo test --package omnilang
git add omnilang/src/codegen/flutter.rs
git commit -m "feat(flutter): add event data extraction, HardwareKeyboard for modifiers"
```

---

### Task 11: Codegen — Remaining Targets (dioxus, vue, javascript)

**Files:**
- Modify: `omnilang/src/codegen/dioxus.rs`
- Modify: `omnilang/src/codegen/vue.rs`
- Modify: `omnilang/src/codegen/javascript.rs`

**Note:** These targets use DOM-based events. Reuse the same extraction patterns from omniview.rs, adapted to each framework's callback conventions.

- [ ] **Step 1: Dioxus** — Apply Web-style extraction. Dioxus uses `web_sys` events, same field names as DOM.

- [ ] **Step 2: Vue** — Apply Web-style extraction. Vue's `v-on` / `@event` passes native DOM events.

- [ ] **Step 3: JavaScript** — Apply Web-style extraction. Raw DOM events.

- [ ] **Step 4: Run full test suite**

```bash
cargo test --package omnilang
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/dioxus.rs omnilang/src/codegen/vue.rs omnilang/src/codegen/javascript.rs
git commit -m "feat(codegen): add event data extraction for dioxus, vue, javascript targets"
```

---

### Task 12: Test Fixtures and E2E Verification

**Files:**
- Modify: `test/projects/learn-omniview/src/components/EventsDemo.omni`
- Create: `test/projects/learn-omniview/src/pages/EventDataDemo.omni`

- [ ] **Step 1: Update EventsDemo to exercise event data fields**

Update the existing `EventsDemo.omni` scroll handler to use `e.y`:

```omni
// Before:
@onScroll={handleScroll}
fn handleScroll() { scrollCount = scrollCount + 1; }

// After:
@onScroll={scrollY = e.y; scrollCount = scrollCount + 1}
```

And the keydown handler in Modal.omni:

```omni
// Before:
@onKeyDown={handleKeyDown}
fn handleKeyDown() { if pressed_keys.contains("Escape") { close() } }

// After: use e.key directly
@onKeyDown={if e.key == "Escape" { close() }}
```

- [ ] **Step 2: Create EventDataDemo page**

```omni
// EventDataDemo.omni — exercise event data fields across categories
component EventDataDemo {
    signal scrollY = 0
    signal keyPressed = ""
    signal mouseX = 0
    signal mouseY = 0
    signal wheelDelta = 0
    signal ctrlHeld = false

    fn handleScroll(e) {
        scrollY = e.y
    }

    fn handleKeyDown(e) {
        keyPressed = e.key
        ctrlHeld = e.ctrl
    }

    fn handleMouseMove(e) {
        mouseX = e.x
        mouseY = e.y
    }

    fn handleWheel(e) {
        wheelDelta = e.deltaY
        ctrlHeld = e.ctrl
    }

    view {
        <div class: event-data-demo @onScroll={handleScroll} @onKeyDown={handleKeyDown}>
            <div class: mouse-tracker @onMouseMove={handleMouseMove}>
                <p>Mouse: ({mouseX}, {mouseY})</p>
            </div>
            <div class: scroll-tracker @onWheel={handleWheel}>
                <p>Scroll Y: {scrollY}</p>
                <p>Wheel Delta: {wheelDelta}</p>
                <p>Ctrl held: {ctrlHeld}</p>
            </div>
            <input @onKeyDown={handleKeyDown} placeholder="Press a key" />
            <p>Key: {keyPressed}</p>
        </div>
    }
}
```

- [ ] **Step 3: Run the full test suite**

```bash
cargo test --package omnilang
```

Expected: all 362+ tests pass. Fix any compilation errors in test fixtures.

- [ ] **Step 4: Compile the learn-omniview project to verify output**

```bash
cargo run -- compile test/projects/learn-omniview/src/main.omni --target omniview
```

Inspect the generated JS to verify:
- Scroll handler receives `{ x: ..., y: ... }` not `undefined`
- KeyDown handler receives `{ key: ..., code: ..., alt: ..., ... }` 
- Wheel handler receives `{ deltaY: ..., ctrl: ... }`
- `e.preventDefault()` emits correctly for onSubmit and explicit calls
- @for delegation emits `_eventKinds` map and dynamic subscriptions

- [ ] **Step 5: Commit**

```bash
git add test/projects/learn-omniview/
git commit -m "test: add event data exercise fixtures — EventsDemo update + EventDataDemo page"
```

---

## Execution Order

Tasks must run sequentially due to dependencies:
```
Task 1 (IR types) → Task 2 (EventHandlerIR) → Task 3 (Pass 1) → Task 4 (Pass 2) →
Task 5 (lints) → Task 6 (omniview extraction) → Task 7 (@for rewrite) → Task 8 (special cases) →
Task 9 (react) → Task 10 (flutter) → Task 11 (remaining targets) → Task 12 (fixtures)
```

Each task ends with a passing `cargo test --package omnilang` and a commit.
