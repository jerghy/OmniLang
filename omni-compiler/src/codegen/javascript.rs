//! JavaScript/TypeScript codegen.
//!
//! TODO: implement full JavaScript codegen

use std::collections::HashMap;

use crate::ir::IRProgram;
use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};

pub struct JavaScriptCodegen {
    ts_mode: bool,
}

impl JavaScriptCodegen {
    pub fn new(ts_mode: bool) -> Self {
        Self { ts_mode }
    }
}

impl Codegen for JavaScriptCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();
        let ext = if self.ts_mode { ".ts" } else { ".js" };

        for (source_path, _source_text) in source_files {
            let mut w = CodeWriter::new();
            w.emit(&format!("// Generated from {}", source_path));
            w.emit("");

            // Emit components
            for (comp_name, _comp) in &ir.components {
                let decl = if self.ts_mode {
                    format!("class {} {{", comp_name)
                } else {
                    format!("class {} {{", comp_name)
                };
                w.emit(&decl);
                w.indent();
                w.emit("constructor() {");
                w.indent();
                w.emit("}");
                w.dedent();
                w.dedent();
                w.emit("}");
                w.emit("");
            }

            let out_path = source_path.replace(".omni", ext);
            files.push(CompiledFile {
                path: out_path,
                content: w.into_string(),
            });
        }

        CompileOutput::success(files)
    }
}
