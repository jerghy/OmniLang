//! Checks signal initializer types and function return types at the IR level.
//!
//! Category: correctness
//! Severity: error
//!
//! This lint operates post-lowering (IR level) as defense-in-depth:
//! - Signal initializer type checks: NOT covered by the AST-level type
//!   checker (validate_types.rs), which focuses on expression/statement types.
//! - Function return type checks: REDUNDANT with validate_types::check_return
//!   but provides a second check after IR lowering transformations.
//!
//! Future: integrate signal init checking into the AST checker and remove
//! the return-type check from this lint once validate_types is fully proven.

use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::{BlockIR, ExprIR, IRProgram, StatementIR};
use crate::lexer::Span;
use rustc_hash::FxHashMap;

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

    fn check_ir(
        &self,
        ir: &IRProgram,
        file_map: &FxHashMap<String, String>,
    ) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        for (comp_name, comp) in ir.all_components() {
            let comp_file = comp
                .source_file
                .as_deref()
                .or_else(|| file_map.keys().next().map(|s| s.as_str()))
                .unwrap_or(comp_name);

            // Check signal initializer types. SignalIntent carries no span —
            // anchor at the component span (Task 4.11; 0:0 only when no span
            // exists at all, e.g. hand-built IR in unit tests).
            let comp_span = comp.source_span;
            for state in &comp.signals {
                let type_kind_str = state.r#type.kind.as_str();
                let init = &state.initial_value;
                if !Self::types_compatible(type_kind_str, init) {
                    let (line, col) = comp_span.map(|s| (s.line, s.col)).unwrap_or((0, 0));
                    diags.push(LintDiagnostic {
                        rule: self.name().into(),
                        severity: self.severity(),
                        message: format!(
                            "Signal '{}' in component '{}': initializer type \
                             does not match declared type '{}'",
                            state.name, comp_name, type_kind_str
                        ),
                        file: comp_file.to_string(),
                        line,
                        col,
                    });
                }
            }

            // Check function return types against return statements —
            // anchored at the function's own span when available.
            for func in &comp.fns {
                if let Some(ref ret_type) = func.return_type {
                    Self::check_function_returns(
                        &func.body,
                        ret_type.kind.as_str(),
                        comp_name,
                        &func.name,
                        comp_file,
                        func.source_span.or(comp_span),
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
        // auto accepts any value
        if type_ann == "auto" {
            return true;
        }
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
        span: Option<Span>,
        diags: &mut Vec<LintDiagnostic>,
    ) {
        let ret_type_lower = ret_type.to_lowercase();
        let is_void =
            ret_type_lower == "void" || ret_type_lower == "()" || ret_type_lower == "unit";
        let (line, col) = span.map(|s| (s.line, s.col)).unwrap_or((0, 0));

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
                            line,
                            col,
                        });
                    }
                }
                StatementIR::IfStmtIR {
                    then_body,
                    else_body,
                    ..
                } => {
                    Self::check_function_returns(
                        then_body, ret_type, comp_name, func_name, file_path, span, diags,
                    );
                    if let Some(ref eb) = else_body {
                        Self::check_function_returns(
                            eb, ret_type, comp_name, func_name, file_path, span, diags,
                        );
                    }
                }
                StatementIR::WhileStmtIR { body, .. } => {
                    Self::check_function_returns(
                        body, ret_type, comp_name, func_name, file_path, span, diags,
                    );
                }
                StatementIR::ForInStmtIR { body, .. } => {
                    Self::check_function_returns(
                        body, ret_type, comp_name, func_name, file_path, span, diags,
                    );
                }
                StatementIR::BlockStmtIR { body } => {
                    Self::check_function_returns(
                        body, ret_type, comp_name, func_name, file_path, span, diags,
                    );
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{BlockIR, ComponentIR, FunctionIR, StatementIR, TypeIR, Visibility};
    use crate::lexer::Span;
    use rustc_hash::FxHashMap;

    use super::super::helpers::test_helpers::make_test_ir;
    use super::*;

    #[test]
    fn test_type_mismatch_void_return_with_value() {
        let mut components = FxHashMap::default();
        components.insert(
            "Test".into(),
            ComponentIR {
                form_intent: None,
                load_intent: vec![],
                name: "Test".into(),
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![],
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

                    source_span: Some(Span {
                        start: 0,
                        end: 30,
                        line: 6,
                        col: 2,
                    }),
                }],
                store_refs: vec![],
                refs: vec![],
                events: vec![],
                view: None,
                styles: vec![],
                ..Default::default()
            },
        );
        let ir = make_test_ir(components, FxHashMap::default());

        let rule = TypeMismatch;
        let diags = rule.check_ir(&ir, &FxHashMap::default());
        assert!(
            !diags.is_empty(),
            "Should detect void function returning a value"
        );
        assert!(
            diags[0].message.contains("void"),
            "Message should mention void: {}",
            diags[0].message
        );
        assert_eq!(
            diags[0].line, 6,
            "must anchor at the function declaration (FunctionIR.source_span)"
        );
        assert_eq!(diags[0].col, 2);
    }

    #[test]
    fn test_type_mismatch_signal_init_anchors_component_span() {
        // SignalIntent carries no span — the diagnostic falls back to the
        // component's source span (Task 4.11: never 0:0 when a span exists).
        let mut components = FxHashMap::default();
        components.insert(
            "Test".into(),
            ComponentIR {
                form_intent: None,
                load_intent: vec![],
                name: "Test".into(),
                signals: vec![crate::ir::SignalIntent {
                    name: "age".into(),
                    r#type: TypeIR {
                        kind: "i32".into(),
                        type_args: vec![],
                        is_nullable: false,
                    },
                    initial_value: ExprIR::LiteralExpr {
                        value: serde_json::Value::String("adult".into()),
                    },
                }],
                source_span: Some(Span {
                    start: 0,
                    end: 40,
                    line: 3,
                    col: 1,
                }),
                ..Default::default()
            },
        );
        let ir = make_test_ir(components, FxHashMap::default());

        let diags = TypeMismatch.check_ir(&ir, &FxHashMap::default());
        assert!(
            diags.iter().any(|d| d.message.contains("initializer type")),
            "signal init type mismatch must be flagged, got: {:?}",
            diags
        );
        assert_eq!(
            diags[0].line, 3,
            "must fall back to the component source span"
        );
    }
}
