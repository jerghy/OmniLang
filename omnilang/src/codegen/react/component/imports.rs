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
    /// Whether the component has styles that need `./name.module.css`.
    pub has_styles: bool,
    /// Whether the component needs `import { animate } from "@omni/animation"`.
    pub has_motion: bool,
    /// Pre-formatted import lines for each used store hook.
    pub store_hook_lines: Vec<String>,
    /// Pre-formatted import lines for each used child component.
    pub child_import_lines: Vec<String>,
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
    has_motion: bool,
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

    ComponentImports {
        hooks: hooks.to_vec(),
        has_styles,
        has_motion,
        store_hook_lines,
        child_import_lines,
    }
}

impl ComponentImports {
    /// Write all import lines at the top of the generated .tsx file.
    ///
    /// Order of emission:
    /// 1. React hooks
    /// 2. `./name.module.css` (styles)
    /// 3. `@omni/animation`
    /// 4. Props interface (`{name}Props`)
    /// 5. Store hook imports
    /// 6. Child component imports
    pub(super) fn emit(
        &self,
        w: &mut CodeWriter,
        name: &str,
        comp: &ComponentIR,
        has_props: bool,
    ) {
        // ── 1-3: React / styles / animate imports ───────────
        if !self.hooks.is_empty() {
            w.emit_fmt(format_args!(
                "import React, {{ {} }} from 'react';",
                self.hooks.join(", ")
            ));
        }
        if self.has_styles {
            w.emit_fmt(format_args!("import styles from './{}.module.css';", name));
        }
        if self.has_motion {
            w.emit("import { animate } from \"@omni/animation\";");
        }
        if !self.hooks.is_empty() || self.has_styles || self.has_motion {
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
            for (event_name, param_type) in
                comp.event_names.iter().zip(comp.event_param_types.iter())
            {
                let ts_type = if param_type.is_empty() {
                    "any"
                } else {
                    param_type.as_str()
                };
                w.emit_fmt(format_args!(
                    "  {}?: (value: {}) => void;",
                    event_name, ts_type
                ));
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

        // Blank line after store/child imports
        if !self.store_hook_lines.is_empty() || !self.child_import_lines.is_empty() {
            w.emit("");
        }
    }
}
