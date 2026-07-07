//! Checks assignment and return type compatibility.
//!
//! Category: correctness
//! Severity: error
//!
//! TODO: Full type inference integration. Currently a placeholder
//! that catches obvious mismatches in IR-level type annotations.

use std::collections::HashMap;

use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::{BlockIR, ExprIR, IRProgram, StatementIR};

pub struct TypeMismatch;

impl LintRule for TypeMismatch {
    fn name(&self) -> &str {
        "type-mismatch"
    }
    fn category(&self) -> &str {
        "correctness"
    }
    fn severity(&self) -> LintSeverity {
        LintSeverity::Error
    }

    fn check_ir(&self, ir: &IRProgram, file_map: &HashMap<String, String>) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        for (comp_name, comp) in ir.all_components() {
            let comp_file = comp
                .source_file
                .as_deref()
                .or_else(|| file_map.keys().next().map(|s| s.as_str()))
                .unwrap_or(comp_name);

            // Check signal initializer types
            for state in &comp.states {
                let type_kind_str = state.r#type.kind.as_str();
                let init = &state.initial_value;
                if !Self::types_compatible(type_kind_str, init) {
                    diags.push(LintDiagnostic {
                        rule: self.name().into(),
                        severity: self.severity(),
                        message: format!(
                            "Signal '{}' in component '{}': initializer type \
                             does not match declared type '{}'",
                            state.name, comp_name, type_kind_str
                        ),
                        file: comp_file.to_string(),
                        line: 0,
                        col: 0,
                    });
                }
            }

            // Check function return types against return statements
            for func in &comp.fns {
                if let Some(ref ret_type) = func.return_type {
                    Self::check_function_returns(
                        &func.body,
                        ret_type.kind.as_str(),
                        comp_name,
                        &func.name,
                        comp_file,
                        &mut diags,
                    );
                }
            }
        }
        diags
    }
}

impl TypeMismatch {
    fn types_compatible(type_ann: &str, value: &ExprIR) -> bool {
        let type_ann = type_ann.to_lowercase();
        match value {
            ExprIR::LiteralExpr { value } => match value {
                serde_json::Value::Number(_) => {
                    type_ann.contains("i32")
                        || type_ann.contains("i64")
                        || type_ann.contains("f64")
                        || type_ann.contains("f32")
                        || type_ann.contains("int")
                        || type_ann.contains("float")
                        || type_ann.contains("num")
                }
                serde_json::Value::String(_) => {
                    type_ann.contains("str") || type_ann.contains("String")
                }
                serde_json::Value::Bool(_) => type_ann.contains("bool") || type_ann == "bool",
                serde_json::Value::Null => true,
                _ => true,
            },
            _ => true,
        }
    }

    fn check_function_returns(
        body: &BlockIR,
        ret_type: &str,
        comp_name: &str,
        func_name: &str,
        file_path: &str,
        diags: &mut Vec<LintDiagnostic>,
    ) {
        let ret_type_lower = ret_type.to_lowercase();
        let is_void =
            ret_type_lower == "void" || ret_type_lower == "()" || ret_type_lower == "unit";

        for stmt in &body.statements {
            match stmt {
                StatementIR::ReturnStmtIR { value } => {
                    if is_void && value.is_some() {
                        diags.push(LintDiagnostic {
                            rule: "type-mismatch".into(),
                            severity: LintSeverity::Error,
                            message: format!(
                                "Function '{}.{}' has void return type but returns a value",
                                comp_name, func_name
                            ),
                            file: file_path.to_string(),
                            line: 0,
                            col: 0,
                        });
                    }
                }
                StatementIR::IfStmtIR {
                    then_body,
                    else_body,
                    ..
                } => {
                    Self::check_function_returns(
                        then_body, ret_type, comp_name, func_name, file_path, diags,
                    );
                    if let Some(ref eb) = else_body {
                        Self::check_function_returns(
                            eb, ret_type, comp_name, func_name, file_path, diags,
                        );
                    }
                }
                StatementIR::WhileStmtIR { body, .. } => {
                    Self::check_function_returns(
                        body, ret_type, comp_name, func_name, file_path, diags,
                    );
                }
                StatementIR::ForInStmtIR { body, .. } => {
                    Self::check_function_returns(
                        body, ret_type, comp_name, func_name, file_path, diags,
                    );
                }
                StatementIR::BlockStmtIR { body } => {
                    Self::check_function_returns(
                        body, ret_type, comp_name, func_name, file_path, diags,
                    );
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::ir::{BlockIR, ComponentIR, FunctionIR, StatementIR, TypeIR, Visibility};

    use super::super::helpers::test_helpers::make_test_ir;
    use super::*;

    #[test]
    fn test_type_mismatch_void_return_with_value() {
        let mut components = HashMap::new();
        components.insert(
            "Test".into(),
            ComponentIR {
                name: "Test".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![FunctionIR {
                    name: "returns_void".into(),
                    visibility: Visibility::Private,
                    params: vec![],
                    return_type: Some(TypeIR {
                        kind: "void".into(),
                        type_args: vec![],
                        is_nullable: false,
                    }),
                    body: BlockIR {
                        statements: vec![StatementIR::ReturnStmtIR {
                            value: Some(ExprIR::LiteralExpr {
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

        let rule = TypeMismatch;
        let diags = rule.check_ir(&ir, &HashMap::new());
        assert!(
            !diags.is_empty(),
            "Should detect void function returning a value"
        );
        assert!(
            diags[0].message.contains("void"),
            "Message should mention void: {}",
            diags[0].message
        );
    }
}
