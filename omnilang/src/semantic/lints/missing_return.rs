//! Flags functions with a non-void return type that can fall
//! off the end without a return statement.
//!
//! Category: correctness
//! Severity: warning

use std::collections::HashMap;

use super::helpers::statements_contain_return;
use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::IRProgram;

pub struct MissingReturn;

impl LintRule for MissingReturn {
    fn name(&self) -> &str {
        "missing-return"
    }
    fn category(&self) -> &str {
        "correctness"
    }
    fn severity(&self) -> LintSeverity {
        LintSeverity::Warning
    }

    fn check_ir(&self, ir: &IRProgram, file_map: &HashMap<String, String>) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        for (comp_name, comp) in ir.all_components() {
            let comp_file = comp
                .source_file
                .as_deref()
                .or_else(|| file_map.keys().next().map(|s| s.as_str()))
                .unwrap_or(comp_name);
            for func in &comp.fns {
                if let Some(ref ret_type) = func.return_type {
                    let kind = ret_type.kind.as_str();
                    if kind != "void" && kind != "()" && kind != "Unit" {
                        let has_return = statements_contain_return(&func.body.statements, false);
                        if !has_return {
                            diags.push(LintDiagnostic {
                                rule: self.name().into(),
                                severity: self.severity(),
                                message: format!(
                                    "Function '{}' in component '{}' \
                                     has return type '{}' but no return statement",
                                    func.name, comp_name, ret_type.kind
                                ),
                                file: comp_file.to_string(),
                                line: 0,
                                col: 0,
                            });
                        }
                    }
                }
            }
        }
        diags
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::ir::{BlockIR, ComponentIR, FunctionIR, StatementIR, TypeIR, Visibility};

    use super::super::helpers::test_helpers::make_test_ir;
    use super::*;

    #[test]
    fn test_missing_return_detects_missing_return() {
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
                fns: vec![FunctionIR {
                    name: "getCount".into(),
                    visibility: Visibility::Private,
                    params: vec![],
                    return_type: Some(TypeIR {
                        kind: "Int".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    }),
                    body: BlockIR {
                        statements: vec![StatementIR::LetStmtIR {
                            name: "x".into(),
                            value: None,
                            r#type: None,
                        }],
                    },
                    is_async: false,
                }],
                store_refs: vec![],
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

        let diags = MissingReturn.check_ir(&ir, &HashMap::new());
        assert_eq!(diags.len(), 1, "expected 1 diagnostic for missing return");
        assert!(
            diags[0].message.contains("getCount"),
            "message should mention the function: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_missing_return_skips_void_functions() {
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
                fns: vec![FunctionIR {
                    name: "log".into(),
                    visibility: Visibility::Private,
                    params: vec![],
                    return_type: Some(TypeIR {
                        kind: "void".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    }),
                    body: BlockIR { statements: vec![] },
                    is_async: false,
                }],
                store_refs: vec![],
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

        let diags = MissingReturn.check_ir(&ir, &HashMap::new());
        assert_eq!(
            diags.len(),
            0,
            "void functions should not trigger missing-return"
        );
    }

    #[test]
    fn test_missing_return_skips_when_return_present() {
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
                fns: vec![FunctionIR {
                    name: "getCount".into(),
                    visibility: Visibility::Private,
                    params: vec![],
                    return_type: Some(TypeIR {
                        kind: "Int".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    }),
                    body: BlockIR {
                        statements: vec![StatementIR::ReturnStmtIR {
                            value: Some(crate::ir::ExprIR::LiteralExpr {
                                value: serde_json::Value::Number(42.into()),
                            }),
                        }],
                    },
                    is_async: false,
                }],
                store_refs: vec![],
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

        let diags = MissingReturn.check_ir(&ir, &HashMap::new());
        assert_eq!(diags.len(), 0, "functions with return should not trigger");
    }
}
