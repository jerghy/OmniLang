//! Rust codegen -- basic implementation.
//!
//! TODO: implement full Rust codegen

use std::collections::HashMap;

use crate::ir::IRProgram;
use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};

pub struct RustCodegen;

impl Codegen for RustCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (source_path, _source_text) in source_files {
            let mut w = CodeWriter::new();
            w.emit(&format!("// Generated from {}", source_path));
            w.emit("");

            // Emit components as structs with impl blocks
            for (comp_name, _comp) in &ir.components {
                w.emit(&format!("struct {};", comp_name));
                w.emit("");
                w.emit(&format!("impl {} {{", comp_name));
                w.indent();
                w.emit(&format!("fn new() -> Self {{"));
                w.indent();
                w.emit(&format!("{}", comp_name));
                w.dedent();
                w.emit("}");
                w.dedent();
                w.emit("}");
                w.emit("");
            }

            let out_path = source_path.replace(".omni", ".rs");
            files.push(CompiledFile {
                path: out_path,
                content: w.into_string(),
            });
        }

        CompileOutput::success(files)
    }
}
