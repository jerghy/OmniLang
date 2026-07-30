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
    #[allow(clippy::too_many_arguments)]
    pub(super) fn gen_jsx_return(
        &self,
        w: &mut CodeWriter,
        comp: &ComponentIR,
        component_map: &FxHashMap<&str, &ComponentIR>,
        signal_names: &FxHashSet<String>,
        motion_names: &FxHashSet<String>,
        name: &str,
        ref_names: &FxHashSet<String>,
    ) {
        w.emit("return (");
        w.indent();
        if let Some(view) = &comp.view {
            view::gen_view_node_tsx(
                w,
                view,
                component_map,
                signal_names,
                motion_names,
                ref_names,
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
        ViewTree::Text(_) | ViewTree::Interpolation(_) | ViewTree::Slot(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
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
        ViewTree::Text(_) | ViewTree::Slot(_) => {}
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
        ViewTree::Text(_) | ViewTree::Interpolation(_) | ViewTree::Slot(_) => {}
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
            if let Some(class_expr) = el.props.get("class") {
                if let ExprIR::LiteralExpr { value } = class_expr {
                    if let Some(class_str) = value.as_str() {
                        for class_name in class_str.split_whitespace() {
                            acc.insert(class_name.to_string());
                        }
                    }
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
        ViewTree::Text(_) | ViewTree::Interpolation(_) | ViewTree::Slot(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}
