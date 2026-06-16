//! Python codegen -- placeholder.
//!
//! TODO: implement Python codegen

use std::collections::HashMap;

use super::{Codegen, CompileOutput};

pub struct PythonCodegen;

impl Codegen for PythonCodegen {
    fn generate(&self, _ir: &crate::ir::IRProgram, _source_files: &HashMap<String, String>) -> CompileOutput {
        CompileOutput::error("Python codegen: not yet implemented".into())
    }
}
