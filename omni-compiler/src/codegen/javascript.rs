//! JavaScript codegen -- placeholder.
//!
//! TODO: implement JavaScript codegen

use std::collections::HashMap;

use super::{CompileOutput};

/// Compile IR program to JavaScript output files.
///
/// Returns an error stub until implemented.
pub fn compile_files(_ir: &crate::ir::IRProgram, _source_files: &HashMap<String, String>) -> CompileOutput {
    CompileOutput::error("JavaScript codegen not yet implemented in Rust".into())
}
