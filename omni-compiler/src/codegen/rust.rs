//! Rust codegen -- placeholder.
//!
//! TODO: implement Rust codegen

use std::collections::HashMap;

use super::{Codegen, CompileOutput};

pub struct RustCodegen;

impl Codegen for RustCodegen {
    fn generate(&self, _ir: &crate::ir::IRProgram, _source_files: &HashMap<String, String>) -> CompileOutput {
        CompileOutput::error("Rust codegen: not yet implemented".into())
    }
}
