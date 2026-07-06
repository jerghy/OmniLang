//! Python codegen visitor — generates Python source from ModuleIR containers.
//!
//! Implements `CodegenVisitor` for the Python backend with special overrides:
//! - `generate_type_aliases` emits module-level imports before type aliases
//! - `generate_functions` adds `if __name__ == "__main__": main()` entry point

use crate::codegen::shared::visitor::CodegenVisitor;
use crate::codegen::CodeWriter;
use crate::ir::*;

pub(crate) struct PythonVisitor;

impl CodegenVisitor for PythonVisitor {
    /// Generate type aliases with preceding module-level imports.
    fn generate_type_aliases(&mut self, w: &mut CodeWriter, module: &ModuleIR) {
        super::generate_module_imports(w, module);
        for alias in module.type_aliases.values() {
            self.generate_type_alias(w, alias);
        }
        if !module.type_aliases.is_empty() {
            w.emit("");
        }
    }

    /// Generate functions with optional `if __name__ == "__main__"` entry point.
    fn generate_functions(&mut self, w: &mut CodeWriter, module: &ModuleIR) {
        let mut has_main = false;
        for (fname, func) in &module.functions {
            self.generate_function(w, func);
            w.emit("");
            if fname == "main" {
                has_main = true;
            }
        }
        if has_main {
            w.emit("if __name__ == \"__main__\":");
            w.indent();
            w.emit("main()");
            w.dedent();
            w.emit("");
        }
    }

    fn generate_type_alias(&mut self, w: &mut CodeWriter, alias: &TypeAliasIR) {
        let target = super::expr::format_type_ir(&alias.target);
        w.emit_fmt(format_args!("{} = {}", alias.name, target));
    }

    fn generate_struct(&mut self, w: &mut CodeWriter, s: &StructIR) {
        w.emit("@dataclass");
        w.emit_fmt(format_args!("class {}:", s.name));
        w.indent();
        for field in &s.fields {
            let type_str = super::expr::format_type_ir(&field.r#type);
            w.emit_fmt(format_args!("{}: {}", field.name, type_str));
        }
        if s.fields.is_empty() {
            w.emit("pass");
        }
        w.dedent();
    }

    fn generate_enum(&mut self, w: &mut CodeWriter, e: &EnumIR) {
        w.emit_fmt(format_args!("class {}(Enum):", e.name));
        w.indent();
        if e.variants.is_empty() {
            w.emit("pass");
        } else {
            for (i, variant) in e.variants.iter().enumerate() {
                w.emit_fmt(format_args!("{} = {}", variant.name, i));
            }
        }
        w.dedent();
    }

    fn generate_function(&mut self, w: &mut CodeWriter, f: &FunctionIR) {
        super::stmt::generate_function(w, f, true);
    }

    fn generate_store(&mut self, w: &mut CodeWriter, name: &str, store: &StoreIR) {
        super::stmt::generate_store(w, name, store);
    }

    fn generate_component(&mut self, w: &mut CodeWriter, name: &str, comp: &ComponentIR) {
        super::stmt::generate_component(w, name, comp);
    }
}
