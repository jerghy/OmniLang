//! Semantic analysis — 2-pass: collect, then resolve+validate inline.

pub mod cycles;
pub mod lints;
pub mod symbols;
pub mod walk_view;

mod collect;
mod helpers;
mod motion;
mod resolve;
mod validate_inline;
mod validate_motion;
mod validate_motion_triggers;
mod validate_store_access;
mod validate_types;

use rustc_hash::FxHashMap;
use crate::error::Diagnostic;
use crate::parser::ast::Program;
use symbols::*;

/// Result of semantic analysis — always returned (diagnostics may contain errors/warnings).
#[derive(Debug)]
pub struct SemanticResult {
    pub symbols: SymbolTable,
    pub dependencies: FxHashMap<String, Vec<String>>,
    pub diagnostics: Vec<Diagnostic>,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn analyze_source(source: &str) -> SemanticResult {
        let tokens = tokenize(source);
        let program = parse(tokens, source).expect("parse should succeed");
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
}
