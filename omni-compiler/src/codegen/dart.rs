//! Dart codegen -- placeholder.
//!
//! TODO: implement Dart codegen

use std::collections::HashMap;

use super::{Codegen, CompileOutput};

pub struct DartCodegen;

impl Codegen for DartCodegen {
    fn generate(&self, _ir: &crate::ir::IRProgram, _source_files: &HashMap<String, String>) -> CompileOutput {
        CompileOutput::error("Dart codegen: not yet implemented".into())
    }
}
