pub mod span;
pub mod errors;
pub mod ast;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod codegen;
pub mod analyze;
pub mod modules;
pub mod config;
pub mod e2e_tests;

#[cfg(test)]
mod fixture_tests;
#[cfg(test)]
mod semantic_tests;
#[cfg(test)]
mod analyze_tests;
#[cfg(test)]
mod compiler_tests;

use crate::ast::Program;
use crate::errors::ErrorCollector;
use std::collections::HashMap;

/// Compile an OmniLang source file to a target language.
/// This is the main entry point for the compiler library.
pub fn compile(
    source: &str,
    filename: &str,
    target: &str,
) -> Result<String, String> {
    // Phase 1: Parse
    let program = parser::parse(source, filename).map_err(|e| e.report(None))?;

    // Phase 2: Semantic analysis
    let source_lines = HashMap::from([(filename.to_string(), source.lines().map(|l| l.to_string()).collect())]);
    let (type_map, rename_map) = semantic::analyze(&program, Some(&source_lines))
        .map_err(|e| e.report(Some(&source_lines)))?;

    // Phase 3: Code generation
    let output = codegen::generate(&program, target, &rename_map, &type_map)
        .map_err(|e| format!("Code generation error: {:?}", e))?;

    Ok(output)
}

/// Run semantic analysis and return the type map and rename map.
pub fn analyze_semantic(
    program: &Program,
    source_lines: Option<&HashMap<String, Vec<String>>>,
) -> Result<
    (
        HashMap<ast::NodeId, ast::TypeRepr>,
        Vec<HashMap<String, String>>,
    ),
    ErrorCollector,
> {
    semantic::analyze(program, source_lines)
}

/// Generate code for a target language from analyzed program.
pub fn generate_code(
    program: &Program,
    target: &str,
    rename_map: &[HashMap<String, String>],
    type_map: &HashMap<ast::NodeId, ast::TypeRepr>,
) -> Result<String, String> {
    codegen::generate(program, target, rename_map, type_map)
        .map_err(|e| format!("Code generation error: {:?}", e))
}
