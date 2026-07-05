//! Checks that every store reference from a component actually
//! refers to an existing store in the IR program.
//!
//! Category: correctness
//! Severity: error

use std::collections::HashMap;

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

    fn check_ir(&self, ir: &IRProgram, _file_map: &HashMap<String, String>) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        for (name, comp) in ir.all_components() {
            for store_ref in &comp.store_refs {
                if !ir.all_stores().contains_key(store_ref.as_str()) {
                    diags.push(LintDiagnostic {
                        rule: self.name().into(),
                        severity: self.severity(),
                        message: format!(
                            "Component '{}' references undefined store '{}'",
                            name, store_ref
                        ),
                        file: name.to_string(),
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
    use std::collections::HashMap;

    use crate::ir::ComponentIR;

    use super::super::helpers::test_helpers::make_test_ir;
    use super::*;

    #[test]
    fn test_undefined_ref_detects_missing_store() {
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
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
        let ir = make_test_ir(components, HashMap::new());

        let diags = UndefinedRef.check_ir(&ir, &HashMap::new());
        assert_eq!(diags.len(), 1, "expected 1 diagnostic for missing store");
        assert!(
            diags[0].message.contains("MissingStore"),
            "message should mention the missing store: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_undefined_ref_skips_existing_store() {
        let mut stores = HashMap::new();
        stores.insert(
            "CounterStore".into(),
            crate::ir::StoreIR {
                name: "CounterStore".into(),
                signals: HashMap::new(),
                signal_defaults: HashMap::new(),
                getters: HashMap::new(),
                actions: vec![],
                ..Default::default()
            },
        );
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
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

        let diags = UndefinedRef.check_ir(&ir, &HashMap::new());
        assert_eq!(diags.len(), 0, "no diagnostics expected when store exists");
    }
}
