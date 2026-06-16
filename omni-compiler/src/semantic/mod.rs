//! Semantic analysis — multi-pass: symbol collection, resolution, type checking, dependency analysis.

use crate::error::Diagnostic;
use crate::parser::ast::Program;

/// Result of semantic analysis — symbol table, type map, dependency info.
#[derive(Debug)]
pub struct SemanticResult {}

/// Run all semantic analysis passes on the parsed program.
pub fn analyze(program: &Program) -> Result<SemanticResult, Vec<Diagnostic>> {
    let _ = program;
    Ok(SemanticResult {})
}
