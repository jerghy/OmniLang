//! Codegen trait -- implement for each target.

use rustc_hash::FxHashMap;
use crate::ir::{ComponentIR, FunctionIR, IRProgram, ModuleIR, StoreIR};

use super::writer::{CodeWriter, CompileOutput, CompiledFile};

/// Codegen trait — implement for each target.
///
/// Provides default per-module and per-component generation strategies.
/// Backends override only the methods they need.
///
/// # Built-in Strategies
///
/// - [`generate_per_module`](Codegen::generate_per_module): Iterates all
///   modules and emits code for each one (Camp A style). Override per-item
///   methods (`generate_function`, etc.) to customize.
///
/// - [`generate_per_component`](Codegen::generate_per_component): Iterates
///   all components and stores, delegating to `gen_component_file` /
///   `gen_store_file` (Camp B style).
///
/// Backends can call the default strategy from their `generate()` implementation
/// or provide completely custom logic.
pub trait Codegen: Send + Sync {
    /// Generate output files for the given IR program and source files.
    #[must_use]
    fn generate(&self, ir: &IRProgram, source_files: &FxHashMap<String, String>) -> CompileOutput;

    // ── Per-component (Camp B) overrides ────────────────────

    /// Generate output file(s) for a single component.
    ///
    /// Override this to produce one or more files per component.
    /// The default returns an empty vector (no output).
    fn gen_component_file(
        &self,
        _name: &str,
        _comp: &ComponentIR,
        _ir: &IRProgram,
    ) -> Vec<CompiledFile> {
        vec![]
    }

    /// Generate output file(s) for a single store.
    ///
    /// Override this to produce one or more files per store.
    /// The default returns an empty vector (no output).
    fn gen_store_file(&self, _name: &str, _store: &StoreIR) -> Vec<CompiledFile> {
        vec![]
    }

    // ── Per-item generators (defaults do nothing) ────────────

    /// Generate module-level header (imports, declarations, etc.).
    fn generate_module_header(&self, _w: &mut CodeWriter, _module: &ModuleIR) {}

    /// Generate a single function definition.
    fn generate_function(&self, _w: &mut CodeWriter, _f: &FunctionIR) {}

    // ── Built-in strategies ─────────────────────────────────

    /// Generate output by iterating all modules (Camp A style).
    ///
    /// For each module, calls [`generate_module_header`] once, then
    /// [`generate_function`] for each function.
    /// All output is written to a single file.
    #[must_use]
    fn generate_per_module(&self, ir: &IRProgram, _source_files: &FxHashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();
        for (module_name, module) in &ir.modules {
            let mut w = CodeWriter::default();
            self.generate_module_header(&mut w, module);
            for f in module.functions.values() {
                self.generate_function(&mut w, f);
            }
            files.push(CompiledFile {
                path: format!("{}.ext", module_name),
                content: w.into_string(),
            });
        }
        CompileOutput::success(files)
    }

    /// Generate output by iterating all components and stores (Camp B style).
    ///
    /// Delegates to [`gen_component_file`] and [`gen_store_file`] for each
    /// component and store, collecting all returned files into a single
    /// `CompileOutput`.
    #[must_use]
    fn generate_per_component(&self, ir: &IRProgram, _source_files: &FxHashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();
        for (name, comp) in ir.all_components() {
            files.extend(self.gen_component_file(name, comp, ir));
        }
        for (name, store) in ir.all_stores() {
            files.extend(self.gen_store_file(name, store));
        }
        CompileOutput::success(files)
    }
}
