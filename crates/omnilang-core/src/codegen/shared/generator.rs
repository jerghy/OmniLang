//! Build a component map and constructor helpers shared across codegen backends.

use rustc_hash::FxHashMap;

use crate::codegen::writer::CodeWriter;
use crate::ir::*;

/// Build an owned component map from all components in the IR program.
///
/// This map can be shared across per-component codegen calls to avoid
/// per-component cloning of `ComponentIR` values.
pub(crate) fn build_component_map(ir: &IRProgram) -> FxHashMap<String, ComponentIR> {
    ir.all_components()
        .into_iter()
        .map(|(n, c)| (n.to_string(), c.clone()))
        .collect()
}

// ═══════════════════════════════════════════════════════════
// Constructor helpers (shared across Dart and JS codegen)
// ═══════════════════════════════════════════════════════════

/// Emit a constructor that initializes props and states for a component.
///
/// `constructor_name` is the constructor keyword — the class name in Dart
/// (e.g. `"MyComponent"`) or `"constructor"` in JavaScript/TypeScript.
pub(crate) fn emit_constructor_init(
    w: &mut CodeWriter,
    constructor_name: &str,
    comp: &ComponentIR,
    format_init: impl Fn(&ExprIR) -> String,
) {
    w.emit_fmt(format_args!("{constructor_name}() {{"));
    w.indent();
    for prop in comp.props.keys() {
        w.emit_fmt(format_args!("this.{prop} = null;  // prop"));
    }
    for state in &comp.states {
        let init = format_init(&state.initial_value);
        w.emit_fmt(format_args!("this.{} = {};", state.name, init));
    }
    w.dedent();
    w.emit("}");
}

/// Emit a store constructor that initializes signal defaults.
///
/// `constructor_name` is the constructor keyword — the class name in Dart
/// or `"constructor"` in JavaScript/TypeScript.
///
/// Returns early without emitting anything when the store has no signals.
pub(crate) fn emit_store_constructor_init(
    w: &mut CodeWriter,
    constructor_name: &str,
    store: &StoreIR,
    format_default: impl Fn(&serde_json::Value) -> String,
) {
    if store.signals.is_empty() {
        return;
    }
    w.emit_fmt(format_args!("{constructor_name}() {{"));
    w.indent();
    for signal_name in store.signals.keys() {
        let default = store
            .signal_defaults
            .get(signal_name)
            .map(&format_default)
            .unwrap_or_else(|| "null".into());
        w.emit_fmt(format_args!("this.{} = {};", signal_name, default));
    }
    w.dedent();
    w.emit("}");
}
