//! PerComponentCodegen trait and generate_per_component helper.
//!
//! Unifies the "iterate all components/stores" pattern across Camp B
//! codegen targets (React, Vue, Flutter, Dioxus).

use crate::codegen::writer::{CodeWriter, CompileOutput, CompiledFile};
use crate::ir::*;

/// Trait for codegen targets that generate one or more files per
/// component or store.
pub(crate) trait PerComponentCodegen {
    /// Generate output file(s) for a single component.
    fn gen_component_file(
        &self,
        name: &str,
        comp: &ComponentIR,
        ir: &IRProgram,
    ) -> Vec<CompiledFile>;

    /// Generate output file(s) for a single store.
    fn gen_store_file(&self, name: &str, store: &StoreIR) -> Vec<CompiledFile>;
}

/// Iterate all components and stores in the IR program, delegating to the
/// provided [`PerComponentCodegen`] implementation.
pub(crate) fn generate_per_component(
    cg: &impl PerComponentCodegen,
    ir: &IRProgram,
) -> CompileOutput {
    let mut files = Vec::new();
    for (name, comp) in ir.all_components() {
        files.extend(cg.gen_component_file(name, comp, ir));
    }
    for (name, store) in ir.all_stores() {
        files.extend(cg.gen_store_file(name, store));
    }
    CompileOutput::success(files)
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
