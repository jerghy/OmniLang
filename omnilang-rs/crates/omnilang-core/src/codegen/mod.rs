pub mod base;
pub mod python;
pub mod javascript;
pub mod dart;
pub mod rust_cg;

use crate::ast::*;
use std::collections::HashMap;

pub fn generate(
    program: &Program,
    target: &str,
    rename_map: &[HashMap<String, String>],
    type_map: &HashMap<NodeId, TypeRepr>,
) -> Result<String, String> {
    match target {
        "python" | "py" => {
            let gen = python::PythonGenerator::new(type_map.clone(), rename_map.to_vec());
            Ok(gen.generate(program))
        }
        "js" | "javascript" => {
            let gen = javascript::JavaScriptGenerator::new(type_map.clone(), rename_map.to_vec());
            Ok(gen.generate(program))
        }
        "dart" => {
            let gen = dart::DartGenerator::new(type_map.clone(), rename_map.to_vec());
            Ok(gen.generate(program))
        }
        "rust" | "rs" => {
            let gen = rust_cg::RustGenerator::new(type_map.clone(), rename_map.to_vec());
            Ok(gen.generate(program))
        }
        _ => Err(format!("Unknown target: {}", target)),
    }
}
