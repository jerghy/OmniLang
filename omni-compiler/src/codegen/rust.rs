//! Rust codegen -- placeholder.
//!
//! TODO: implement Rust codegen

use std::collections::HashMap;

use super::{CompileOutput};

/// Compile IR program to Rust output files.
///
/// Returns an error stub until implemented.
pub fn compile_files(_ir: &crate::ir::IRProgram, _source_files: &HashMap<String, String>) -> CompileOutput {
    CompileOutput::error("Rust codegen not yet implemented in Rust".into())
}
