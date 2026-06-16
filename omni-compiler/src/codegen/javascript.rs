//! JavaScript/TypeScript codegen -- placeholder.
//!
//! TODO: implement JavaScript codegen

use std::collections::HashMap;

use super::{Codegen, CompileOutput};

pub struct JavaScriptCodegen {
    ts_mode: bool,
}

impl JavaScriptCodegen {
    pub fn new(ts_mode: bool) -> Self {
        Self { ts_mode }
    }
}

impl Codegen for JavaScriptCodegen {
    fn generate(&self, _ir: &crate::ir::IRProgram, _source_files: &HashMap<String, String>) -> CompileOutput {
        let label = if self.ts_mode { "TypeScript" } else { "JavaScript" };
        CompileOutput::error(format!("{label} codegen: not yet implemented"))
    }
}
