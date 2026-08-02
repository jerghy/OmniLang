// ═══════════════════════════════════════════════════════════
// Import collection and emission for React component .tsx files
// ═══════════════════════════════════════════════════════════

use crate::codegen::shared::types::{FormatType, JsTypeFormatter};
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

use super::jsx;

/// Holds all import decisions for a component, separating the "what"
/// (collected by [`collect_component_imports`]) from the "write"
/// (performed by [`ComponentImports::emit`]).
pub(super) struct ComponentImports {
    /// React hook names to import (e.g. `useState`, `useMemo`, `useEffect`).
    pub hooks: Vec<String>,
    /// Whether the view emits `<React.Fragment>` (any `@for` list
    /// rendering) — requires the React default import even when the
    /// component uses no hooks (e.g. a read-only `@for` over props).
    pub needs_react_default: bool,
    /// Whether the component has styles that need `./name.module.css`.
    pub has_styles: bool,
    /// Pre-formatted import lines for each used store hook.
    pub store_hook_lines: Vec<String>,
    /// Pre-formatted import lines for each used child component.
    pub child_import_lines: Vec<String>,
    /// Design runtime imports on the root component: `useDesignTokens` hook
    /// imports and `useThemeStore` side-effect imports (HIGH #2).
    pub design_import_lines: Vec<String>,
    /// Slot declarations (name + type constraint) that become typed props in
    /// the Props interface. Consumer-side projections are excluded.
    pub slot_infos: Vec<(Option<String>, Option<TypeIR>)>,
}

/// Collect all import data needed by the component.
///
/// Extracts store-hook imports (by checking `used_store_refs` against
/// `ir.all_stores()`) and child-component imports (via
/// [`jsx::collect_view_child_components`]).
pub(super) fn collect_component_imports(
    comp: &ComponentIR,
    ir: &IRProgram,
    component_map: &FxHashMap<&str, &ComponentIR>,
    used_store_refs: &FxHashSet<String>,
    hooks: &[String],
    has_styles: bool,
    design_import_lines: &[String],
) -> ComponentImports {
    // ── Store hook imports ──────────────────────────────────
    let all_stores = ir.all_stores();
    let mut store_hook_lines = Vec::new();
    for store_ref in &comp.store_refs {
        if !used_store_refs.contains(store_ref.as_str()) {
            continue;
        }
        if let Some(store) = all_stores.get(store_ref.as_str()) {
            let hook_name = format!("use{}", store_ref);
            let module_name = format!("use{}", store_ref);
            let import_path = super::super::relative_import_path(
                comp.source_file.as_deref(),
                store.source_file.as_deref(),
                &module_name,
            );
            store_hook_lines.push(format!(
                "import {{ {} }} from '{}';",
                hook_name, import_path
            ));
        }
    }

    // ── Slot props — typed as ReactNode in the Props interface, so the
    // named type import must exist whenever the view declares any slot
    // (body-level declarations or view-level directives). ──
    let mut hooks = hooks.to_vec();
    if (!comp.slots.is_empty()
        || comp
            .view
            .as_ref()
            .is_some_and(|v| jsx::view_has_slots(v, component_map)))
        && !hooks.iter().any(|h| h == "ReactNode")
    {
        hooks.push("ReactNode".to_string());
    }
    // Slot declarations — body-level `@slot` declarations (ADR-053 D5)
    // first, then view-level directives (consumer-side projections are
    // excluded).
    let slot_infos = jsx::component_declared_slots(comp, component_map);

    // ── Child component imports ─────────────────────────────
    let child_comps: FxHashSet<String> = if let Some(ref view) = comp.view {
        jsx::collect_view_child_components(view, component_map)
    } else {
        FxHashSet::default()
    };
    let mut child_names: Vec<String> = child_comps.into_iter().collect();
    child_names.sort();
    let child_import_lines: Vec<String> = child_names
        .iter()
        .map(|child_name| {
            let child_source = component_map
                .get(child_name.as_str())
                .and_then(|c| c.source_file.as_deref());
            let import_path = super::super::relative_import_path(
                comp.source_file.as_deref(),
                child_source,
                child_name,
            );
            format!("import {{ {} }} from '{}';", child_name, import_path)
        })
        .collect();

    // ── React default import (React.Fragment from @for) ─────
    // `@for` renders `<React.Fragment key=...>`, which requires the React
    // default import even when the component uses no hooks (read-only
    // signals/props). Without it tsc reports TS2686 ("React refers to a UMD
    // global") — the hook-driven import below used to be the only trigger.
    let needs_react_default = comp
        .view
        .as_ref()
        .is_some_and(view_has_list_rendering);

    ComponentImports {
        hooks,
        needs_react_default,
        has_styles,
        store_hook_lines,
        child_import_lines,
        design_import_lines: design_import_lines.to_vec(),
        slot_infos,
    }
}

/// Whether the view tree contains any `@for` list rendering (nested inside
/// elements, conditionals, slots or fragments).
fn view_has_list_rendering(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => el.children.iter().any(view_has_list_rendering),
        ViewTree::ListRendering(_) => true,
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().is_some_and(view_has_list_rendering))
                || cond.fallback.as_ref().is_some_and(view_has_list_rendering)
        }
        ViewTree::Fragment(children) => children.iter().any(view_has_list_rendering),
        ViewTree::Slot { fallback, .. } => fallback.iter().any(view_has_list_rendering),
        // ViewTree is #[non_exhaustive] — unknown variants cannot contain
        // list rendering.
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

impl ComponentImports {
    /// Write all import lines at the top of the generated .tsx file.
    ///
    /// Order of emission:
    /// 1. React hooks
    /// 2. `./name.module.css` (styles)
    /// 3. Props interface (`{name}Props`)
    /// 4. Store hook imports
    /// 5. Child component imports
    pub(super) fn emit(&self, w: &mut CodeWriter, name: &str, comp: &ComponentIR, has_props: bool) {
        // ── 1-2: React / styles imports ─────────────────────
        if !self.hooks.is_empty() || self.needs_react_default {
            if self.hooks.is_empty() {
                // React.Fragment from @for without any hooks — bare default
                // import is all that's needed.
                w.emit("import React from 'react';");
            } else {
                w.emit_fmt(format_args!(
                    "import React, {{ {} }} from 'react';",
                    self.hooks.join(", ")
                ));
            }
        }
        if self.has_styles {
            w.emit_fmt(format_args!("import styles from './{}.module.css';", name));
        }
        if !self.hooks.is_empty() || self.needs_react_default || self.has_styles {
            w.emit("");
        }

        // ── 4: Props interface ──────────────────────────────
        if has_props {
            w.emit_fmt(format_args!("interface {}Props {{", name));
            for (prop_name, prop_type) in &comp.props {
                let ts_type =
                    JsTypeFormatter.format_type(prop_type.kind.as_str(), &prop_type.type_args);
                let optional = if comp.prop_defaults.contains_key(prop_name) {
                    "?"
                } else {
                    ""
                };
                w.emit_fmt(format_args!("  {}{}: {};", prop_name, optional, ts_type));
            }
            for event in &comp.events {
                let ts_type = if event.param_type.is_empty() {
                    "unknown"
                } else {
                    event.param_type.as_str()
                };
                w.emit_fmt(format_args!(
                    "  {}?: (value: {}) => void;",
                    event.name, ts_type
                ));
            }
            // Slot props — unnamed slot → `children`, named slots → same-named
            // prop. Typed as ReactNode unless the source declared a type
            // constraint (`@slot header: String` → `header?: string`).
            // Dedup guards against parser artifacts where the literal text
            // "@slot …" inside view content is parsed as a directive.
            let mut emitted_slot_props: FxHashSet<String> = FxHashSet::default();
            let slot_collides: FxHashSet<&str> = comp
                .signals
                .iter()
                .map(|s| s.name.as_str())
                .chain(comp.computeds.iter().map(|c| c.name.as_str()))
                .chain(comp.fns.iter().map(|f| f.name.as_str()))
                .collect();
            for (slot_name, type_constraint) in &self.slot_infos {
                let slot_prop = slot_name.as_deref().unwrap_or("children");
                if comp.props.contains_key(slot_prop)
                    || comp.events.iter().any(|e| e.name == slot_prop)
                    || slot_collides.contains(slot_prop)
                    || !emitted_slot_props.insert(slot_prop.to_string())
                {
                    continue;
                }
                let ts_type = match type_constraint {
                    Some(ty) => {
                        let t = JsTypeFormatter.format_type(ty.kind.as_str(), &ty.type_args);
                        if t.is_empty() {
                            "ReactNode".to_string()
                        } else {
                            t
                        }
                    }
                    None => "ReactNode".to_string(),
                };
                w.emit_fmt(format_args!("  {}?: {};", slot_prop, ts_type));
            }
            w.emit("}");
            w.emit("");
        }

        // ── 5-6: Store / child imports ─────────────────────
        for line in &self.store_hook_lines {
            w.emit(line);
        }
        for line in &self.child_import_lines {
            w.emit(line);
        }
        // ── 7: Design runtime imports (root component) ─────
        for line in &self.design_import_lines {
            w.emit(line);
        }

        // Blank line after store/child/design imports
        if !self.store_hook_lines.is_empty()
            || !self.child_import_lines.is_empty()
            || !self.design_import_lines.is_empty()
        {
            w.emit("");
        }
    }
}
