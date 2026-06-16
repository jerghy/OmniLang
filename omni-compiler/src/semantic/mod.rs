//! Semantic analysis — symbol resolution, type checking, constraint validation.

use crate::error::Diagnostic;
use crate::parser::ast::Program;

/// Result of semantic analysis.
#[derive(Debug)]
pub struct SemanticResult {
    // Will be populated in Task 1.5.
}

/// Run all semantic analysis passes on the parsed program.
pub fn analyze(program: &Program) -> Result<SemanticResult, Vec<Diagnostic>> {
    let _ = program;
    Ok(SemanticResult {})
}
