//! Slot materialization (ADR-053 D5): body-level `@slot` declarations surface
//! in the component view as `{name}` interpolations. This module converts
//! those interpolations into `ViewTree::Slot` nodes (carrying the declared
//! fallbacks) so every backend renders and projects body-level slots
//! uniformly with view-level `@slot` directives.

use super::types::{
    ComponentIR, ConditionalBranch, ConditionalIntent, ElementIR, ExprIR, ListRenderingIntent,
    SlotDeclIR, ViewTree,
};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// Names that win over a same-named slot for `{name}` interpolations: props,
/// signals, computeds, functions, and events. The semantic layer warns on
/// such collisions; codegen must keep treating `{name}` as the non-slot
/// declaration (mirrors the slot-prop collision guards in the React/Flutter
/// backends).
pub fn slot_collision_names(comp: &ComponentIR) -> FxHashSet<&str> {
    comp.props
        .keys()
        .map(String::as_str)
        .chain(comp.signals.iter().map(|s| s.name.as_str()))
        .chain(comp.computeds.iter().map(|c| c.name.as_str()))
        .chain(comp.fns.iter().map(|f| f.name.as_str()))
        .chain(comp.events.iter().map(|e| e.name.as_str()))
        .collect()
}

/// Convert `{name}` interpolations matching a body-level `@slot` declaration
/// into `ViewTree::Slot` nodes (carrying the declaration's fallback) so slot
/// rendering and projection machinery applies uniformly with view-level
/// slots. Back-compat: view-level `@slot` directives are already
/// `ViewTree::Slot` nodes and pass through untouched. Interpolations whose
/// name is in `excluded` (see [`slot_collision_names`]) stay ordinary
/// interpolations — the existing declaration wins.
///
/// Note: slot references inside slot fallbacks (view-level `@slot` fallbacks
/// or body-declared fallbacks) are not materialized — fallbacks keep their
/// interpolations.
pub fn materialize_view_slots(
    view: &ViewTree,
    comp: &ComponentIR,
    excluded: &FxHashSet<&str>,
) -> ViewTree {
    let mut slots: FxHashMap<&str, &SlotDeclIR> = FxHashMap::default();
    for decl in &comp.slots {
        slots
            .entry(decl.name.as_deref().unwrap_or("children"))
            .or_insert(decl);
    }
    fn go(
        node: &ViewTree,
        excluded: &FxHashSet<&str>,
        slots: &FxHashMap<&str, &SlotDeclIR>,
    ) -> ViewTree {
        match node {
            ViewTree::Interpolation(expr) => {
                if let ExprIR::IdentifierExpr { name, .. } = expr.as_ref() {
                    if let Some(decl) = slots.get(name.as_str()) {
                        if !excluded.contains(name.as_str()) {
                            return (*decl).clone().into_view_tree();
                        }
                    }
                }
                ViewTree::Interpolation(expr.clone())
            }
            ViewTree::Element(el) => {
                let children = el.children.iter().map(|c| go(c, excluded, slots)).collect();
                ViewTree::Element(Box::new(ElementIR {
                    children,
                    ..*el.clone()
                }))
            }
            ViewTree::Conditional(cond) => {
                let branches = cond
                    .branches
                    .iter()
                    .map(|b| ConditionalBranch {
                        body: b.body.as_ref().map(|v| go(v, excluded, slots)),
                        ..b.clone()
                    })
                    .collect();
                ViewTree::Conditional(Box::new(ConditionalIntent {
                    branches,
                    fallback: cond.fallback.as_ref().map(|v| go(v, excluded, slots)),
                    transition_mode: cond.transition_mode.clone(),
                }))
            }
            ViewTree::ListRendering(list) => {
                ViewTree::ListRendering(Box::new(ListRenderingIntent {
                    body: list.body.as_ref().map(|v| go(v, excluded, slots)),
                    empty_body: list.empty_body.as_ref().map(|v| go(v, excluded, slots)),
                    ..*list.clone()
                }))
            }
            ViewTree::Fragment(children) => {
                ViewTree::Fragment(children.iter().map(|c| go(c, excluded, slots)).collect())
            }
            other => other.clone(),
        }
    }
    go(view, excluded, &slots)
}
