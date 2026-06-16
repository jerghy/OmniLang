//! Intent IR types and AST -> IR lowering.
//!
//! Mirrors Python omniview/ir_types.py. Serializes to/from JSON via serde.

use serde::{Deserialize, Serialize};

use crate::semantic::SemanticResult;
use crate::parser::ast::Program;

/// Top-level IR container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRProgram {
    pub version: String,
    pub components: std::collections::HashMap<String, ()>,
    pub stores: std::collections::HashMap<String, ()>,
    pub source_map: std::collections::HashMap<String, crate::sourcemap::SourceMapEntry>,
}

/// Lower AST -> IR.
pub fn lower(program: &Program, semantics: &SemanticResult) -> IRProgram {
    let _ = (program, semantics);
    IRProgram {
        version: "1.0".to_string(),
        components: std::collections::HashMap::new(),
        stores: std::collections::HashMap::new(),
        source_map: std::collections::HashMap::new(),
    }
}
