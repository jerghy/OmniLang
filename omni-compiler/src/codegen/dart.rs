//! Dart codegen -- placeholder.
//!
//! TODO: implement Dart codegen

use std::collections::HashMap;

use super::{CompileOutput};

/// Compile IR program to Dart output files.
///
/// Returns an error stub until implemented.
pub fn compile_files(_ir: &crate::ir::IRProgram, _source_files: &HashMap<String, String>) -> CompileOutput {
    CompileOutput::error("Dart codegen not yet implemented in Rust".into())
}
