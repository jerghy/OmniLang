//! Semantic analysis — multi-pass: collect, resolve, type-check, deps, validate.

pub mod cycles;
pub mod lints;
pub mod symbols;

mod collect;
mod helpers;
mod motion;
mod resolve;
mod validate_motion;
mod validate_store_access;
mod validate_types;

use std::collections::HashMap;

use crate::error::Diagnostic;
use crate::parser::ast::Program;
use symbols::*;

/// Result of semantic analysis — always returned (diagnostics may contain errors/warnings).
#[derive(Debug)]
pub struct SemanticResult {
    pub symbols: SymbolTable,
    pub dependencies: HashMap<String, Vec<String>>,
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
    dependencies: HashMap<String, Vec<String>>,
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
            dependencies: HashMap::new(),
            resolving_interpolation: false,
        }
    }

    /// Declare a symbol, emitting a warning diagnostic on duplicate declaration.
    fn declare_or_warn(&mut self, symbol: Symbol) {
        let declared_at = symbol.declared_at;
        if let Err(msg) = self.symbols.declare(symbol) {
            self.diagnostics.push(Diagnostic {
                severity: crate::error::Severity::Warning,
                code: "E0002".into(),
                message: msg,
                line: 0,
                col: declared_at as u32,
                end_line: 0,
                end_col: declared_at as u32,
                suggestion: None,
            });
        }
    }

    pub fn analyze(&mut self, program: &Program) -> SemanticResult {
        // Pass 1: Collect symbols from all modules
        for (_filepath, module) in &program.modules {
            self.collect_module_symbols(module);
        }

        // Pass 2: Resolve types per module
        for (_filepath, module) in &program.modules {
            self.resolve_module_types(module);
        }

        // Pass 3: Collect dependencies per module
        for (_filepath, module) in &program.modules {
            self.collect_module_dependencies(module);
        }

        // Pass 4: Validate per module
        for (_filepath, module) in &program.modules {
            self.validate_module(module);
        }

        // Always return what we have — diagnostics accumulated in self.diagnostics
        SemanticResult {
            symbols: std::mem::take(&mut self.symbols),
            dependencies: std::mem::take(&mut self.dependencies),
            diagnostics: std::mem::take(&mut self.diagnostics),
        }
    }
}
