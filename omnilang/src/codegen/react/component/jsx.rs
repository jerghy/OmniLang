// ═══════════════════════════════════════════════════════════
// JSX codegen: view tree → return (JSX) rendering
// ═══════════════════════════════════════════════════════════

use crate::codegen::react::view;
use crate::codegen::react::ReactCodegen;
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

use super::collect_expr_identifiers;

impl ReactCodegen {
    /// Generate the return (JSX) section of the component function.
    /// `motion_counter` is the shared ordinal counter for motion elements
    /// (see `motion::classify_motion_el`) — it must run in the same DFS
    /// order as the motion effect emitter so ref/handler names match.
    /// `prop_fn_ctx` carries the task 2.8 useCallback/memo decisions.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn gen_jsx_return(
        &self,
        w: &mut CodeWriter,
        view: Option<&ViewTree>,
        comp: &ComponentIR,
        component_map: &FxHashMap<&str, &ComponentIR>,
        design_ctx: &crate::codegen::react::design::DesignCtx,
        signal_names: &FxHashSet<String>,
        store_signal_owners: &FxHashMap<String, String>,
        motion_names: &FxHashSet<String>,
        name: &str,
        ref_names: &FxHashSet<String>,
        store_field_renames: &FxHashMap<String, String>,
        pruned_signal_names: &FxHashSet<String>,
        possibly_undefined: &FxHashSet<String>,
        motion_counter: &mut usize,
        prop_fn_ctx: &view::PropFnMemoCtx,
    ) {
        w.emit("return (");
        w.indent();
        // `view` is the caller's materialized view: body-level `@slot`
        // declarations (ADR-053 D5) have already been converted into
        // `ViewTree::Slot` nodes carrying their fallbacks, so the slot
        // rendering below emits `{name ?? (fallback)}` like view-level
        // slots.
        if let Some(view) = view {
            // gen_branch_body_tsx wraps multi-element roots in a React
            // fragment (`<>...</>`) so the emitted JSX has one parent
            // element — sibling roots would not compile (TS2657).
            view::gen_branch_body_tsx(
                w,
                view,
                comp,
                component_map,
                design_ctx,
                signal_names,
                store_signal_owners,
                motion_names,
                ref_names,
                store_field_renames,
                pruned_signal_names,
                possibly_undefined,
                motion_counter,
                prop_fn_ctx,
                &self.warnings,
                // the view root sits directly inside `return ( ... )` — a
                // parenthesized expression, NOT JSX children: expression
                // nodes must not be wrapped in `{...}` containers (view-
                // root block expressions, I2)
                false,
            );
        } else {
            w.emit("<div className={styles.container}>");
            w.indent();
            w.emit_fmt(format_args!("<h1>{{'{0}'}}</h1>", name));
            w.dedent();
            w.emit("</div>");
        }
        w.dedent();
        w.emit(");");
    }
}

// ═══════════════════════════════════════════════════════════
// View tree traversal helpers
// ═══════════════════════════════════════════════════════════

/// Recursively walk a ViewTree and collect child component tag names
/// that match known components (used to generate import statements).
pub(super) fn collect_view_child_components(
    node: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
) -> FxHashSet<String> {
    let mut result = FxHashSet::default();
    collect_view_child_components_inner(node, component_map, &mut result);
    result
}

fn collect_view_child_components_inner(
    node: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
    acc: &mut FxHashSet<String>,
) {
    match node {
        ViewTree::Element(el) => {
            if component_map.contains_key(el.tag.as_str()) && el.tag != "fragment" {
                acc.insert(el.tag.clone());
            }
            for child in &el.children {
                collect_view_child_components_inner(child, component_map, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_view_child_components_inner(body, component_map, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_view_child_components_inner(fb, component_map, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_view_child_components_inner(body, component_map, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_view_child_components_inner(empty_body, component_map, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_view_child_components_inner(child, component_map, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            // Fallback content renders inside the component — its child
            // components need imports too.
            for child in fallback {
                collect_view_child_components_inner(child, component_map, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Returns true if the view tree declares any slot (`@slot`). Consumer-side
/// projections (slots inside a child component's children) do not count —
/// they are content passed to another component, not declarations.
pub(crate) fn view_has_slots(
    node: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
) -> bool {
    match node {
        ViewTree::Slot { .. } => true,
        ViewTree::Element(el) => {
            if component_map.contains_key(el.tag.as_str()) && el.tag != "fragment" {
                // Children of a component instantiation are projections.
                false
            } else {
                el.children.iter().any(|c| view_has_slots(c, component_map))
            }
        }
        ViewTree::Conditional(cond) => {
            cond.branches.iter().any(|b| {
                b.body
                    .as_ref()
                    .is_some_and(|v| view_has_slots(v, component_map))
            }) || cond
                .fallback
                .as_ref()
                .is_some_and(|v| view_has_slots(v, component_map))
        }
        ViewTree::ListRendering(list) => {
            list.body
                .as_ref()
                .is_some_and(|v| view_has_slots(v, component_map))
                || list
                    .empty_body
                    .as_ref()
                    .is_some_and(|v| view_has_slots(v, component_map))
        }
        ViewTree::Fragment(children) => children.iter().any(|c| view_has_slots(c, component_map)),
        ViewTree::Text(_) | ViewTree::Interpolation(_) => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Collect slot declarations (names) from a view tree.
/// Used to type slot props (`children` / named) in the component TS
/// interface. Slots inside a child component's children are consumer-side
/// projections, not declarations — they are skipped.
pub(crate) fn collect_view_slots(
    node: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
    acc: &mut Vec<Option<String>>,
) {
    match node {
        ViewTree::Slot { name, fallback, .. } => {
            acc.push(name.clone());
            for child in fallback {
                collect_view_slots(child, component_map, acc);
            }
        }
        ViewTree::Element(el) => {
            if component_map.contains_key(el.tag.as_str()) && el.tag != "fragment" {
                // Children of a component instantiation are projections —
                // the child component types them via its own declarations.
                return;
            }
            for child in &el.children {
                collect_view_slots(child, component_map, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_view_slots(body, component_map, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_view_slots(fb, component_map, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_view_slots(body, component_map, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_view_slots(empty_body, component_map, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_view_slots(child, component_map, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// All slot declarations of a component: body-level `@slot` declarations
/// (ADR-053 D5) first, then view-level `@slot` directives (back-compat).
/// Consumer-side projections are not declarations and stay excluded.
/// v2 (design D4): slots carry no type constraint — the name is all there is.
pub(crate) fn component_declared_slots(
    comp: &ComponentIR,
    component_map: &FxHashMap<&str, &ComponentIR>,
) -> Vec<Option<String>> {
    let mut acc: Vec<Option<String>> = comp.slots.iter().map(|s| s.name.clone()).collect();
    if let Some(ref view) = comp.view {
        collect_view_slots(view, component_map, &mut acc);
    }
    acc
}

/// Collect all identifier references from a ViewTree.
pub(super) fn collect_view_identifiers(node: &ViewTree, acc: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for val in el.props.values() {
                collect_expr_identifiers(val, acc);
            }
            for handler in el.events.values() {
                match handler {
                    EventHandlerIR::StoreAction { .. } => {}
                    EventHandlerIR::FnRef { r#ref, .. } => {
                        acc.insert(r#ref.clone());
                    }
                    EventHandlerIR::InlineFn { body, .. } => {
                        for stmt in &body.statements {
                            super::collect_stmt_identifiers(stmt, acc);
                        }
                    }
                }
            }
            // Collect bind:value etc.
            for sig_ref in el.bindings.values() {
                acc.insert(sig_ref.name.clone());
            }
            for child in &el.children {
                collect_view_identifiers(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                collect_expr_identifiers(&branch.condition, acc);
                if let Some(ref body) = branch.body {
                    collect_view_identifiers(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_view_identifiers(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            collect_expr_identifiers(&list.iterable, acc);
            if let Some(ref body) = list.body {
                collect_view_identifiers(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_view_identifiers(empty_body, acc);
            }
        }
        ViewTree::Interpolation(expr) => {
            collect_expr_identifiers(expr, acc);
        }
        ViewTree::Slot { fallback, .. } => {
            // Fallback content renders inside the component — its identifier
            // references (signals, fns) must be collected too.
            for child in fallback {
                collect_view_identifiers(child, acc);
            }
        }
        ViewTree::Text(_) => {}
        ViewTree::Fragment(children) => {
            for child in children {
                collect_view_identifiers(child, acc);
            }
        }
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Collect identifiers from motion triggers on view elements.
pub(super) fn collect_motion_identifiers(node: &ViewTree, acc: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            // Props may contain motion-related expressions
            for val in el.props.values() {
                collect_expr_identifiers(val, acc);
            }
            // Walk children
            for child in &el.children {
                collect_motion_identifiers(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_motion_identifiers(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_motion_identifiers(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_motion_identifiers(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_motion_identifiers(empty_body, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_motion_identifiers(child, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_motion_identifiers(child, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Collect CSS class names actually referenced in the view via class: bindings.
pub(super) fn collect_used_style_classes(node: &ViewTree) -> FxHashSet<String> {
    let mut acc = FxHashSet::default();
    collect_used_style_classes_inner(node, &mut acc);
    acc
}

fn collect_used_style_classes_inner(node: &ViewTree, acc: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            // Collect from style_classes (space-separated string)
            if let Some(ref classes_val) = el.style_classes {
                if let Some(classes_str) = classes_val.as_str() {
                    for class_name in classes_str.split_whitespace() {
                        acc.insert(class_name.to_string());
                    }
                }
            }
            // Collect from class_map_bindings keys
            for class_name in el.class_map_bindings.keys() {
                acc.insert(class_name.clone());
            }
            // Collect from props["class"] (used by DesignClass references like
            // `class: LearnDesign.Page` which store the class name in props)
            if let Some(class_str) = el.props.get("class").and_then(|e| match e {
                ExprIR::LiteralExpr { value } => value.as_str(),
                _ => None,
            }) {
                for class_name in class_str.split_whitespace() {
                    acc.insert(class_name.to_string());
                }
            }
            for child in &el.children {
                collect_used_style_classes_inner(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_used_style_classes_inner(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_used_style_classes_inner(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_used_style_classes_inner(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_used_style_classes_inner(empty_body, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_used_style_classes_inner(child, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_used_style_classes_inner(child, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}
