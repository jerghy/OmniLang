//! Semantic analysis — 2-pass: collect, then resolve+validate inline.

pub mod cycles;
pub mod lints;
pub mod symbols;
pub mod types;
pub mod validate_props;
pub mod validate_ref_ops;
pub mod validate_refs;
pub mod validate_semantic_bindings;
pub mod walk_view;

mod collect;
mod helpers;
mod motion;
mod resolve;
mod validate_component;
mod validate_inline;
mod validate_motion;
mod validate_motion_triggers;
mod validate_store_access;
mod validate_types;

pub use motion::parse_duration_ms;

use crate::error::Diagnostic;
use crate::parser::ast::Program;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use symbols::*;

/// Result of semantic analysis — always returned (diagnostics may contain errors/warnings).
#[derive(Debug, Default)]
pub struct SemanticResult {
    pub symbols: SymbolTable,
    pub dependencies: FxHashMap<String, Vec<String>>,
    pub diagnostics: Vec<Diagnostic>,
    /// Event handler parameter name, set during lowering of event handler bodies.
    /// Used by lower_expr to detect e.field event data access.
    pub event_param_name: Option<String>,
}

/// Run all semantic analysis passes on the parsed program.
/// Always returns a `SemanticResult` (never early-exits) — diagnostics are accumulated
/// in the result for downstream processing.
pub fn analyze(program: &Program) -> SemanticResult {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(program)
}

pub struct SemanticAnalyzer {
    symbols: SymbolTable,
    diagnostics: Vec<Diagnostic>,
    dependencies: FxHashMap<String, Vec<String>>,
    /// True when resolving a view template interpolation {expr}
    resolving_interpolation: bool,
    /// Component names declared in the current module — element tags
    /// matching one of these are component instantiations (their children
    /// are consumer-side projections, not this component's own content).
    module_component_names: FxHashSet<String>,
    /// Set to `Some(component_tag)` while resolving the children of a
    /// component instantiation — used to detect unsupported slot
    /// forwarding (`{slot}` inside another component's children).
    inside_component_children: Option<String>,
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        SemanticAnalyzer {
            symbols: SymbolTable::new(),
            diagnostics: Vec::new(),
            dependencies: FxHashMap::default(),
            resolving_interpolation: false,
            module_component_names: FxHashSet::default(),
            inside_component_children: None,
        }
    }

    /// Declare a symbol, emitting a warning diagnostic on duplicate declaration.
    fn declare_or_warn(&mut self, symbol: Symbol) {
        let declared_at = symbol.declared_at;
        if let Err(msg) = self.symbols.declare(symbol) {
            self.diagnostics.push(Diagnostic {
                severity: crate::error::Severity::Warning,
                kind: crate::error::DiagnosticKind::E0002,
                message: msg,
                file: None,
                line: 0,
                col: declared_at as u32,
                end_line: None,
                end_col: None,
                suggestion: None,
            });
        }
    }

    pub fn analyze(&mut self, program: &Program) -> SemanticResult {
        // Pass 1: Collect symbols from all modules (includes dependency collection)
        for module in program.modules.values() {
            self.collect_module_symbols(module);
        }

        // Pass 2: Resolve types + validate control flow inline per module
        for module in program.modules.values() {
            self.resolve_and_validate_module(module);
        }

        // Always return what we have — diagnostics accumulated in self.diagnostics
        SemanticResult {
            symbols: std::mem::take(&mut self.symbols),
            dependencies: std::mem::take(&mut self.dependencies),
            diagnostics: std::mem::take(&mut self.diagnostics),
            event_param_name: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::symbols::{Symbol, SymbolKind};
    use super::*;
    use crate::error::DiagnosticKind;
    use crate::lexer::tokenize;
    use crate::lexer::Span;
    use crate::parser::ast::{DesignDef, Expr, Module, TokenDef, TokenType, TokenValue};
    use crate::parser::parse;

    fn analyze_source(source: &str) -> SemanticResult {
        let tokens = tokenize(source);
        let (program, _) = parse(tokens, source);
        analyze(&program)
    }

    #[test]
    fn analyze_simple_function() {
        let result = analyze_source("fn main() { let x = 42; print(x); }");
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == crate::error::Severity::Error)
            .collect();
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
        // Should have collected at least: global scope, function:main
        assert!(!result.symbols.scopes().is_empty(), "should have scopes");
    }

    #[test]
    fn analyze_body_slot_symbols_registered() {
        // {header} in the view resolves to the body-level slot symbol
        // (SymbolKind::Slot) — no E0001 "Undeclared variable".
        let result = analyze_source(
            "component Modal { @slot header { <h4>x</h4> } view { <div>{header}</div> } }",
        );
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == crate::error::Severity::Error)
            .collect();
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
        // The slot symbol must exist in the component scope with kind Slot
        let scope_idx = result
            .symbols
            .find_scope_by_name("component:Modal")
            .expect("component scope should exist");
        let slot_sym = result
            .symbols
            .get_scope(scope_idx)
            .and_then(|s| s.symbols.get("header"))
            .expect("slot symbol 'header' should be registered");
        assert_eq!(slot_sym.kind, SymbolKind::Slot);
    }

    #[test]
    fn analyze_body_slot_type_constraint_becomes_symbol_type() {
        let result =
            analyze_source("component Modal { @slot footer: String view { <div>{footer}</div> } }");
        let scope_idx = result
            .symbols
            .find_scope_by_name("component:Modal")
            .expect("component scope should exist");
        let slot_sym = result
            .symbols
            .get_scope(scope_idx)
            .and_then(|s| s.symbols.get("footer"))
            .expect("slot symbol 'footer' should be registered");
        assert_eq!(slot_sym.type_ann.as_deref(), Some("String"));
    }

    #[test]
    fn analyze_duplicate_body_slots_warn() {
        let result =
            analyze_source("component Modal { @slot header @slot header view { <div/> } }");
        assert!(
            result.diagnostics.iter().any(|d| {
                d.severity == crate::error::Severity::Warning
                    && d.message.contains("Duplicate declaration")
            }),
            "expected duplicate-slot warning, got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn analyze_duplicate_body_slots_warn_at_declaration_span() {
        // The duplicate warning should point at the second declaration,
        // not (0, 0) — SlotDecl carries its source span.
        let result =
            analyze_source("component Modal { @slot header @slot header view { <div/> } }");
        let dup = result
            .diagnostics
            .iter()
            .find(|d| d.message.contains("Duplicate declaration"))
            .expect("expected duplicate-slot warning");
        assert!(
            dup.col > 0,
            "duplicate-slot warning should point at the second declaration, col was {}",
            dup.col
        );
    }

    #[test]
    fn analyze_slot_name_colliding_with_signal_warns() {
        // A slot named like a signal/prop is ambiguous for `{name}`
        // interpolations — declare_or_warn flags the collision.
        let result = analyze_source(
            "component Modal { signal count: i32 = 0; @slot count view { <div>{count}</div> } }",
        );
        assert!(
            result.diagnostics.iter().any(|d| {
                d.severity == crate::error::Severity::Warning
                    && d.message.contains("Duplicate declaration")
            }),
            "slot name colliding with a signal must warn, got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn analyze_slot_interpolation_undeclared_still_suppressed() {
        // Back-compat: an interpolation referencing a name that is NOT a
        // declared slot stays suppressed (view-level @slot / interpolation
        // behavior unchanged).
        let result = analyze_source("component Modal { view { <div>{children}</div> } }");
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == crate::error::Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "interpolation suppression must remain, got: {:?}",
            errors
        );
    }

    #[test]
    fn analyze_slot_forwarding_in_component_children_warns() {
        // Slot forwarding: Modal's `{header}` appears inside the children of
        // `<Card>...</Card>` — codegen materializes it into a ViewTree::Slot
        // and treats it as Card's own projection, so the forwarded content
        // silently never reaches Card. Must warn (not error — forwarding may
        // become supported later).
        let result = analyze_source(
            "component Card { @slot header { <h5>Card default</h5> } view { <div>{header}</div> } }
             component Modal { @slot header { <h4>Modal default</h4> } view { <Card>{header}</Card> } }",
        );
        let forwarding: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.message.contains("slot forwarding"))
            .collect();
        assert_eq!(
            forwarding.len(),
            1,
            "expected exactly one slot-forwarding warning, got: {:?}",
            result.diagnostics
        );
        assert!(
            forwarding
                .iter()
                .all(|d| d.severity == crate::error::Severity::Warning),
            "slot forwarding must be a warning, got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn analyze_slot_forwarding_no_warning_for_normal_usage() {
        // Own-view interpolation (plain element), consumer named-prop
        // projection, fallback content, and slot references nested inside
        // plain elements must not warn.
        let result = analyze_source(
            "component Modal { @slot header { <p>{header}</p> } view { <div>{header}</div> } }
             component App { view { <Modal header={<div>Title</div>} /> } }",
        );
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("slot forwarding")),
            "normal slot usage must not warn, got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn analyze_if_mode_fade_wait_none_accepted() {
        // Parser and semantic whitelist must agree: "fade", "wait", "none"
        // are the only valid transition modes (regression: semantic used to
        // reject "fade"/"none" with E0006).
        for mode in ["fade", "wait", "none"] {
            let src = format!(
                "component App {{ signal open: Bool = true; \
                 view {{ @if open mode: \"{}\" {{ <div/> }} }} }}",
                mode
            );
            let result = analyze_source(&src);
            let mode_errors: Vec<_> = result
                .diagnostics
                .iter()
                .filter(|d| d.message.contains("mode"))
                .collect();
            assert!(
                mode_errors.is_empty(),
                "mode '{}' should be accepted, got: {:?}",
                mode,
                mode_errors
            );
        }
    }

    #[test]
    fn analyze_unresolved_variable() {
        // 'y' is not declared anywhere — should produce an error diagnostic
        let result = analyze_source("fn main() { let x = y; }");
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == crate::error::Severity::Error)
            .collect();
        assert!(
            !errors.is_empty(),
            "expected at least one error for unresolved variable"
        );
        assert!(
            errors
                .iter()
                .any(|d| d.message.contains("unresolved") || d.message.contains("'y'")),
            "error message should mention the unresolved variable 'y', got: {:?}",
            errors
        );
    }

    #[test]
    fn analyze_valid_struct_and_function() {
        let source = "struct Point { x: i32, y: i32 }
                       fn distance(p: Point) -> i32 { return p.x + p.y; }";
        let result = analyze_source(source);
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == crate::error::Severity::Error)
            .collect();
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn rejects_expression_in_theme_value() {
        let design = DesignDef {
            name: "App".to_string(),
            themes: vec!["dark".to_string()],
            default_theme: None,
            tokens: vec![TokenDef {
                name: "primary".to_string(),
                token_type: TokenType::Color,
                value: TokenValue::Literal("#ffffff".to_string()),
                theme_values: vec![(
                    "dark".to_string(),
                    TokenValue::Expression(Expr::FieldAccess {
                        object: Box::new(Expr::VarRef(
                            "ThemeStore".to_string(),
                            Span {
                                start: 0,
                                end: 10,
                                line: 0,
                                col: 0,
                            },
                        )),
                        field: "darkColor".to_string(),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 0,
                            col: 0,
                        },
                    }),
                )],
            }],
            classes: vec![],
            variants: vec![],
        };
        let module = Module {
            designs: vec![design],
            ..Default::default()
        };
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.resolve_and_validate_module(&module);
        let errors: Vec<_> = analyzer
            .diagnostics
            .iter()
            .filter(|d| d.kind == DiagnosticKind::E0001)
            .collect();
        assert!(
            !errors.is_empty(),
            "should reject expression in theme value"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Theme") && e.message.contains("dynamic")),
            "diagnostic should mention Theme and dynamic expression restriction, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn rejects_non_const_symbol_in_token_variable_reference() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer
            .symbols
            .declare(Symbol {
                name: "ThemeStore".to_string(),
                kind: SymbolKind::Store,
                type_ann: None,
                mutable: false,
                declared_at: 0,
            })
            .ok();
        let design = DesignDef {
            name: "App".to_string(),
            themes: vec![],
            default_theme: None,
            tokens: vec![TokenDef {
                name: "primary".to_string(),
                token_type: TokenType::Color,
                value: TokenValue::Variable("ThemeStore".to_string()),
                theme_values: vec![],
            }],
            classes: vec![],
            variants: vec![],
        };
        let module = Module {
            designs: vec![design],
            ..Default::default()
        };
        analyzer.resolve_and_validate_module(&module);
        let errors: Vec<_> = analyzer
            .diagnostics
            .iter()
            .filter(|d| d.kind == DiagnosticKind::E0001)
            .collect();
        assert!(
            !errors.is_empty(),
            "should warn when TokenValue::Variable references a non-const symbol"
        );
        assert!(
            errors
                .iter()
                .any(|w| w.message.contains("compile-time constant")),
            "diagnostic should explain that only compile-time constants are valid"
        );
    }

    #[test]
    fn accepts_const_symbol_in_token_variable_reference() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer
            .symbols
            .declare(Symbol {
                name: "BRAND".to_string(),
                kind: SymbolKind::Const,
                type_ann: Some("String".to_string()),
                mutable: false,
                declared_at: 0,
            })
            .ok();
        let design = DesignDef {
            name: "App".to_string(),
            themes: vec![],
            default_theme: None,
            tokens: vec![TokenDef {
                name: "primary".to_string(),
                token_type: TokenType::Color,
                value: TokenValue::Variable("BRAND".to_string()),
                theme_values: vec![],
            }],
            classes: vec![],
            variants: vec![],
        };
        let module = Module {
            designs: vec![design],
            ..Default::default()
        };
        analyzer.resolve_and_validate_module(&module);
        let errors: Vec<_> = analyzer
            .diagnostics
            .iter()
            .filter(|d| d.kind == DiagnosticKind::E0001)
            .collect();
        assert!(
            errors.is_empty(),
            "should accept const symbol in TokenValue::Variable, got: {:?}",
            errors
        );
    }

    #[test]
    fn rejects_non_const_variable_in_theme_value() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer
            .symbols
            .declare(Symbol {
                name: "darkColor".to_string(),
                kind: SymbolKind::Variable,
                type_ann: Some("String".to_string()),
                mutable: false,
                declared_at: 0,
            })
            .ok();
        let design = DesignDef {
            name: "App".to_string(),
            themes: vec![],
            default_theme: None,
            tokens: vec![TokenDef {
                name: "primary".to_string(),
                token_type: TokenType::Color,
                value: TokenValue::Literal("#ffffff".to_string()),
                theme_values: vec![(
                    "dark".to_string(),
                    TokenValue::Variable("darkColor".to_string()),
                )],
            }],
            classes: vec![],
            variants: vec![],
        };
        let module = Module {
            designs: vec![design],
            ..Default::default()
        };
        analyzer.resolve_and_validate_module(&module);
        let errors: Vec<_> = analyzer
            .diagnostics
            .iter()
            .filter(|d| d.kind == DiagnosticKind::E0001)
            .collect();
        assert!(
            !errors.is_empty(),
            "should warn when theme value Variable references a non-const symbol"
        );
        assert!(
            errors
                .iter()
                .any(|w| w.message.contains("compile-time constant")),
            "diagnostic should explain that only compile-time constants are valid in theme values"
        );
    }

    #[test]
    fn accepts_numeric_for_fontweight_token() {
        let src = r#"
            fn getWeight(): int { return 700; }
            design Test {
                token bold: fontWeight = getWeight();
            }
        "#;
        let result = analyze_source(src);
        let has_type_error = result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("type") && d.message.contains("fontWeight"));
        assert!(
            !has_type_error,
            "FontWeight should accept numeric expressions, got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn bare_signal_name_in_token_value_gets_const_kind_diagnostic() {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer
            .symbols
            .declare(Symbol {
                name: "App_dynamicAccent".to_string(),
                kind: SymbolKind::Signal,
                type_ann: Some("String".to_string()),
                mutable: false,
                declared_at: 0,
            })
            .ok();
        let design = DesignDef {
            name: "App".to_string(),
            themes: vec![],
            default_theme: None,
            tokens: vec![TokenDef {
                name: "accent".to_string(),
                token_type: TokenType::Color,
                value: TokenValue::Variable("App_dynamicAccent".to_string()),
                theme_values: vec![],
            }],
            classes: vec![],
            variants: vec![],
        };
        let module = Module {
            designs: vec![design],
            ..Default::default()
        };
        analyzer.resolve_and_validate_module(&module);
        let errors: Vec<_> = analyzer
            .diagnostics
            .iter()
            .filter(|d| d.kind == DiagnosticKind::E0001)
            .collect();
        assert!(
            errors.iter().any(|e| e.message.contains("signal")
                && e.message.contains("App_dynamicAccent")
                && e.message.contains("compile-time constant")),
            "bare signal name should produce const-kind diagnostic. Got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }
}
