//! Checks that every store reference from a component actually
//! refers to an existing store in the IR program.
//!
//! Category: correctness
//! Severity: error

use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::IRProgram;
use rustc_hash::FxHashMap;

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

    fn check_ir(
        &self,
        ir: &IRProgram,
        file_map: &FxHashMap<String, String>,
    ) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        for (name, comp) in ir.all_components() {
            let comp_file = comp
                .source_file
                .as_deref()
                .or_else(|| file_map.keys().next().map(|s| s.as_str()))
                .unwrap_or(name);
            // Task 4.11: anchor at the component's source span.
            let (line, col) = comp.source_span.map(|s| (s.line, s.col)).unwrap_or((0, 0));
            for store_ref in &comp.store_refs {
                // Inline stores (`store Name { ... }` in the component body)
                // are flattened into the component and never registered in
                // IRProgram.stores — treat them as locally-defined.
                let is_inline = comp.inline_stores.iter().any(|s| s == store_ref);
                if !ir.has_store(store_ref.as_str()) && !is_inline {
                    diags.push(LintDiagnostic {
                        rule: self.name().into(),
                        severity: self.severity(),
                        message: format!(
                            "Component '{}' references undefined store '{}'",
                            name, store_ref
                        ),
                        file: comp_file.to_string(),
                        line,
                        col,
                    });
                }
            }
        }
        diags
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::ComponentIR;
    use rustc_hash::FxHashMap;

    use super::super::helpers::test_helpers::make_test_ir;
    use super::*;

    #[test]
    fn test_undefined_ref_detects_missing_store() {
        let mut components = FxHashMap::default();
        components.insert(
            "App".into(),
            ComponentIR {
                name: "App".into(),
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec!["MissingStore".into()],
                refs: vec![],
                events: vec![],
                view: None,
                styles: vec![],
                source_span: Some(crate::lexer::Span {
                    start: 0,
                    end: 30,
                    line: 3,
                    col: 1,
                }),
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
        assert_eq!(diags[0].line, 3, "must anchor at the component source span");
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
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec!["CounterStore".into()],
                refs: vec![],
                events: vec![],
                view: None,
                styles: vec![],
                ..Default::default()
            },
        );
        let ir = make_test_ir(components, stores);

        let diags = UndefinedRef.check_ir(&ir, &FxHashMap::default());
        assert_eq!(diags.len(), 0, "no diagnostics expected when store exists");
    }

    #[test]
    fn test_inline_store_not_reported_as_undefined() {
        // `store Local { ... }` inside a component body is flattened into the
        // component (its name is in store_refs but it never enters
        // IRProgram.stores) — the lint must treat it as locally-defined.
        let mut components = FxHashMap::default();
        components.insert(
            "X".into(),
            ComponentIR {
                name: "X".into(),
                store_refs: vec!["Local".into()],
                inline_stores: vec!["Local".into()],
                ..Default::default()
            },
        );
        let ir = make_test_ir(components, FxHashMap::default());

        let diags = UndefinedRef.check_ir(&ir, &FxHashMap::default());
        assert_eq!(
            diags.len(),
            0,
            "inline store must not be reported as undefined: {:?}",
            diags
        );
    }

    #[test]
    fn test_inline_store_does_not_mask_missing_store_in_other_component() {
        // The inline-store exemption is per-component: another component
        // referencing the same name without declaring it inline is still
        // flagged as undefined.
        let mut components = FxHashMap::default();
        components.insert(
            "Owner".into(),
            ComponentIR {
                name: "Owner".into(),
                store_refs: vec!["Local".into()],
                inline_stores: vec!["Local".into()],
                ..Default::default()
            },
        );
        components.insert(
            "Other".into(),
            ComponentIR {
                name: "Other".into(),
                store_refs: vec!["Local".into()],
                inline_stores: vec![],
                ..Default::default()
            },
        );
        let ir = make_test_ir(components, FxHashMap::default());

        let diags = UndefinedRef.check_ir(&ir, &FxHashMap::default());
        assert_eq!(diags.len(), 1, "expected exactly 1 diagnostic");
        assert!(
            diags[0].message.contains("'Other'"),
            "only the component without the inline store should be flagged: {}",
            diags[0].message
        );
    }
}
