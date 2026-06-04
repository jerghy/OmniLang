pub mod diagnostic;
pub mod registry;
pub mod renderer;
pub mod fix_engine;
pub mod walker;
pub mod rules;
pub mod suppress;

use crate::ast::*;
use std::collections::HashMap;

/// Run all analyze rules on a program and collect diagnostics.
pub fn run_checks(
    program: &Program,
    type_map: &HashMap<NodeId, TypeRepr>,
    rename_map: &[HashMap<String, String>],
    targets: &[String],
    source_lines: HashMap<String, Vec<String>>,
) -> Vec<diagnostic::Diagnostic> {
    let registry = rules::build_default_registry();
    let ctx = registry::AnalyzeContext {
        program,
        source_lines,
        targets: targets.to_vec(),
        type_map,
        rename_map,
        capture_map: HashMap::new(),
    };

    let mut all_diags = vec![];
    for rule in registry.list_enabled() {
        let diags = rule.check(&ctx);
        all_diags.extend(diags);
    }
    all_diags
}
