//! Lowering for component-level navigation/routing declarations
//! (`@route` / `@layout` / `@query`, spec 2026-08-09 §2.1/§2.4/§2.5/§2.6/§2.8).
//!
//! These declarations live on `ComponentDef` (parsed by
//! `parser/top_level/route.rs`), NOT in the view tree — so unlike
//! `scan_view_intents` (form/load), the collectors read the component
//! fields directly. No cross-file registry here: route↔component mapping is
//! resolved at the AST level (T5).

use super::component::lower_type;
use super::expr::lower_expr;
use crate::ir::types::*;
use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use rustc_hash::FxHashMap;

/// Component-level route declaration → `RouteIntentIR` (spec §2.1/§2.5/§2.8).
///
/// The parser keeps EVERY `@route` declaration (V4 duplicate is a
/// semantic-layer error whose message names both paths); lowering
/// deterministically takes the FIRST declaration. Duplicate reporting is the
/// semantic pass's job (T4), not lowering's.
pub(super) fn collect_route_intent(comp: &ComponentDef) -> Option<RouteIntentIR> {
    comp.route_decls.first().map(|r| RouteIntentIR {
        path: r.path.clone(),
        params: r.params.clone(),
        guard: r.guard.clone(),
        lazy: r.lazy,
        redirect: r.redirect.clone(),
    })
}

/// Component-level layout declaration → `LayoutIntentIR` (spec §2.6).
/// `Option` holds at most one `@layout` (a second overwrites — parse-time
/// last-wins, component.rs dispatch loop); the mutual-exclusion with
/// `@route` is V3, a semantic-layer error.
pub(super) fn collect_layout_intent(comp: &ComponentDef) -> Option<LayoutIntentIR> {
    comp.layout.as_ref().map(|l| LayoutIntentIR {
        prefix: l.prefix.clone(),
    })
}

/// Component-level query parameter declarations → `Vec<QueryParamIR>`
/// (spec §2.4). Declaration order is preserved; each default value
/// expression is lowered through the regular expression lowering pipeline
/// (so `@query filter: String = "all";` keeps its literal and
/// `@query count: Int = total;` keeps its identifier reference).
pub(super) fn collect_query_params(
    comp: &ComponentDef,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> Vec<QueryParamIR> {
    comp.queries
        .iter()
        .map(|q| QueryParamIR {
            name: q.name.clone(),
            ty: lower_type(&q.type_ann),
            default: q
                .default
                .as_ref()
                .map(|d| lower_expr(d, variant_map, None, semantics)),
        })
        .collect()
}
