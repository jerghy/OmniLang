//! CodegenVisitor trait — eliminates duplicated container traversal across 8 codegen backends.
//!
//! Provides a single `generate_module` dispatch method backed by overrideable
//! per-collection and per-item generators. Defaults iterate all six container
//! types. Backends override only the methods that differ.

use crate::codegen::CodeWriter;
use crate::ir::*;

/// Trait for target-language module-level code generation.
///
/// Default implementations iterate each container collection in order
/// (type_aliases → structs → enums → functions → stores → components)
/// with blank-line separation. Backends override per-item methods for
/// language-specific output, or override an entire collection method
/// (e.g., Python's `generate_functions` to add `if __name__ == "__main__"`).
#[allow(dead_code)]
pub(crate) trait CodegenVisitor {
    /// Generate code for all containers in a module. Default iterates all 6 container types.
    fn generate_module(&mut self, w: &mut CodeWriter, module: &ModuleIR) {
        self.generate_type_aliases(w, module);
        self.generate_structs(w, module);
        self.generate_enums(w, module);
        self.generate_functions(w, module);
        self.generate_stores(w, module);
        self.generate_components(w, module);
    }

    // ── Per-collection defaults with blank-line separation ──

    /// Generate all type aliases in a module.
    fn generate_type_aliases(&mut self, w: &mut CodeWriter, module: &ModuleIR) {
        for alias in module.type_aliases.values() {
            self.generate_type_alias(w, alias);
        }
        if !module.type_aliases.is_empty() {
            w.emit("");
        }
    }

    /// Generate all structs in a module.
    fn generate_structs(&mut self, w: &mut CodeWriter, module: &ModuleIR) {
        for s in module.structs.values() {
            self.generate_struct(w, s);
            w.emit("");
        }
    }

    /// Generate all enums in a module.
    fn generate_enums(&mut self, w: &mut CodeWriter, module: &ModuleIR) {
        for e in module.enums.values() {
            self.generate_enum(w, e);
            w.emit("");
        }
    }

    /// Generate all functions in a module.
    fn generate_functions(&mut self, w: &mut CodeWriter, module: &ModuleIR) {
        for f in module.functions.values() {
            self.generate_function(w, f);
            w.emit("");
        }
    }

    /// Generate all stores in a module.
    fn generate_stores(&mut self, w: &mut CodeWriter, module: &ModuleIR) {
        for (name, store) in &module.stores {
            self.generate_store(w, name, store);
            w.emit("");
        }
    }

    /// Generate all components in a module.
    fn generate_components(&mut self, w: &mut CodeWriter, module: &ModuleIR) {
        for (name, comp) in &module.components {
            self.generate_component(w, name, comp);
            w.emit("");
        }
    }

    // ── Per-item generators (default no-op, backends override what they support) ──

    /// Generate a single type alias.
    fn generate_type_alias(&mut self, _w: &mut CodeWriter, _alias: &TypeAliasIR) {}

    /// Generate a single struct.
    fn generate_struct(&mut self, _w: &mut CodeWriter, _s: &StructIR) {}

    /// Generate a single enum.
    fn generate_enum(&mut self, _w: &mut CodeWriter, _e: &EnumIR) {}

    /// Generate a single standalone (module-level) function.
    fn generate_function(&mut self, _w: &mut CodeWriter, _f: &FunctionIR) {}

    /// Generate a single store.
    fn generate_store(&mut self, _w: &mut CodeWriter, _name: &str, _store: &StoreIR) {}

    /// Generate a single component.
    fn generate_component(&mut self, _w: &mut CodeWriter, _name: &str, _comp: &ComponentIR) {}
}
