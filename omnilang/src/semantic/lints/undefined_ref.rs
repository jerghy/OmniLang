//! Checks that every store reference from a component actually
//! refers to an existing store in the IR program.
//!
//! Category: correctness
//! Severity: error

use rustc_hash::FxHashMap;
use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::IRProgram;

pub struct UndefinedRef;

impl LintRule for UndefinedRef {
    fn name(&self) -> &str {
        "undefined-ref"
    }
    fn category(&self) -> &str {
        "correctness"
    }
    fn severity(&self) -> LintSeverity {
        LintSeverity::Error
    }

    fn check_ir(&self, ir: &IRProgram, file_map: &FxHashMap<String, String>) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        for (name, comp) in ir.all_components() {
            let comp_file = comp
                .source_file
                .as_deref()
                .or_else(|| file_map.keys().next().map(|s| s.as_str()))
                .unwrap_or(name);
            for store_ref in &comp.store_refs {
                if !ir.all_stores().contains_key(store_ref.as_str()) {
                    diags.push(LintDiagnostic {
                        rule: self.name().into(),
                        severity: self.severity(),
                        message: format!(
                            "Component '{}' references undefined store '{}'",
                            name, store_ref
                        ),
                        file: comp_file.to_string(),
                        line: 0,
                        col: 0,
                    });
                }
            }
        }
        diags
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;
    use crate::ir::ComponentIR;

    use super::super::helpers::test_helpers::make_test_ir;
    use super::*;

    #[test]
    fn test_undefined_ref_detects_missing_store() {
        let mut components = FxHashMap::default();
        components.insert(
            "App".into(),
            ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec!["MissingStore".into()],
                refs: vec![],
                event_names: vec![],
                view: None,
                styles: vec![],
                motions: vec![],
                if_mode_default: None,
                hero_defs: vec![],
                timelines: vec![],
                ..Default::default()
            },
        );
        let ir = make_test_ir(components, FxHashMap::default());

        let diags = UndefinedRef.check_ir(&ir, &FxHashMap::default());
        assert_eq!(diags.len(), 1, "expected 1 diagnostic for missing store");
        assert!(
            diags[0].message.contains("MissingStore"),
            "message should mention the missing store: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_undefined_ref_skips_existing_store() {
        let mut stores = FxHashMap::default();
        stores.insert(
            "CounterStore".into(),
            crate::ir::StoreIR {
                name: "CounterStore".into(),
                signals: FxHashMap::default(),
                signal_defaults: FxHashMap::default(),
                getters: FxHashMap::default(),
                actions: vec![],
                ..Default::default()
            },
        );
        let mut components = FxHashMap::default();
        components.insert(
            "App".into(),
            ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec!["CounterStore".into()],
                refs: vec![],
                event_names: vec![],
                view: None,
                styles: vec![],
                motions: vec![],
                if_mode_default: None,
                hero_defs: vec![],
                timelines: vec![],
                ..Default::default()
            },
        );
        let ir = make_test_ir(components, stores);

        let diags = UndefinedRef.check_ir(&ir, &FxHashMap::default());
        assert_eq!(diags.len(), 0, "no diagnostics expected when store exists");
    }
}
