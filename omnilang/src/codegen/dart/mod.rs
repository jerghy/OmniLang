//! Dart codegen: IR -> Dart.
//!
//! Generates Dart source files from Intent IR. Each component becomes
//! a class, each function becomes a method, and stores become standalone
//! classes.

mod expr;
#[cfg(test)]
mod tests;
mod visitor;

use crate::codegen::shared::visitor::CodegenVisitor;

use rustc_hash::FxHashMap;
use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};
use crate::codegen::shared::expr::{DartFormatExpr, FormatExpr};
use crate::codegen::shared::generator::{emit_constructor_init, emit_store_constructor_init};
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::{format_json_value, JSON_DART};
use crate::ir::*;

// Bring expr helpers into scope for mod.rs internals and pub(crate) visibility for visitor/tests
pub(crate) use expr::dart_type;
pub(crate) use expr::format_expr_ir;

pub struct DartCodegen;

impl Codegen for DartCodegen {
    fn generate(&self, ir: &IRProgram, _source_files: &FxHashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (module_key, module) in &ir.modules {
            let module_name = super::sanitize_module_name(module_key);
            let source_path = format!("{}.omni", module_name);

            let mut w = CodeWriter::new();
            w.emit_fmt(format_args!("// Generated from {}", source_path));
            w.emit("");

            let mut visitor = visitor::DartVisitor;
            visitor.generate_module(&mut w, module);

            let out_path = format!("{}.dart", module_name);
            files.push(CompiledFile {
                path: out_path,
                content: w.into_string(),
            });
        }

        CompileOutput::success(files)
    }
}

// ═══════════════════════════════════════════════════════════
// Component generation
// ═══════════════════════════════════════════════════════════

fn generate_component(w: &mut CodeWriter, name: &str, comp: &ComponentIR) {
    w.emit_fmt(format_args!("class {} {{", name));
    w.indent();

    if comp.states.is_empty()
        && comp.props.is_empty()
        && comp.fns.is_empty()
        && comp.computeds.is_empty()
    {
        // Empty class — just emit default constructor
        w.emit_fmt(format_args!("{}();", name));
        w.dedent();
        w.emit("}");
        return;
    }

    // Field declarations for props (initialized to null)
    for (prop_name, prop_type) in &comp.props {
        let ann = dart_type(prop_type.kind.as_str(), &prop_type.type_args);
        w.emit_fmt(format_args!("{} {};", ann, prop_name));
    }

    // Field declarations + initialization for states
    for state in &comp.states {
        let ann = dart_type(state.r#type.kind.as_str(), &state.r#type.type_args);
        let init = format_expr_ir(&state.initial_value);
        // Use `late` for fields initialized in constructor
        w.emit_fmt(format_args!("{} {} = {};", ann, state.name, init));
    }

    // Constructor with state/prop initialization
    if !comp.states.is_empty() || !comp.props.is_empty() {
        w.emit("");
        emit_constructor_init(w, name, comp, format_expr_ir);
    }

    // Computed properties as methods
    for computed in &comp.computeds {
        w.emit("");
        let body = format_expr_ir(&computed.body);
        let ret_type = dart_type(computed.r#type.kind.as_str(), &computed.r#type.type_args);
        w.emit_fmt(format_args!("{} {}() {{", ret_type, computed.name));
        w.indent();
        w.emit_fmt(format_args!("return {};", body));
        w.dedent();
        w.emit("}");
    }

    // Functions (methods)
    for func in &comp.fns {
        w.emit("");
        generate_function(w, func);
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Store generation
// ═══════════════════════════════════════════════════════════

fn generate_store(w: &mut CodeWriter, name: &str, store: &StoreIR) {
    w.emit_fmt(format_args!("class {} {{", name));
    w.indent();

    if store.signals.is_empty() && store.getters.is_empty() {
        w.emit_fmt(format_args!("{}();", name));
        w.dedent();
        w.emit("}");
        return;
    }

    // Field declarations with defaults for signals
    for (signal_name, signal_type) in &store.signals {
        let ann = dart_type(signal_type.kind.as_str(), &signal_type.type_args);
        let default = store
            .signal_defaults
            .get(signal_name)
            .map(|v| format_json_value(v, &JSON_DART))
            .unwrap_or_else(|| "null".into());
        w.emit_fmt(format_args!("{} {} = {};", ann, signal_name, default));
    }

    // Default constructor
    if !store.signals.is_empty() {
        w.emit("");
        emit_store_constructor_init(w, name, store, |v| format_json_value(v, &JSON_DART));
    }

    // Getters
    for (getter_name, getter_expr) in &store.getters {
        w.emit("");
        w.emit_fmt(format_args!("get {} {{", getter_name));
        w.indent();
        w.emit_fmt(format_args!("return {};", format_expr_ir(getter_expr)));
        w.dedent();
        w.emit("}");
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Function generation
// ═══════════════════════════════════════════════════════════

fn generate_function(w: &mut CodeWriter, func: &FunctionIR) {
    // Build parameter list — Dart uses `Type name` for positional params
    let mut param_strs = Vec::new();
    for p in &func.params {
        let mut s = String::new();
        if let Some(ref ty) = p.r#type {
            let ann = dart_type(ty.kind.as_str(), &ty.type_args);
            s.push_str(&format!("{} ", ann));
        }
        s.push_str(&p.name);
        param_strs.push(s);
    }

    // Determine return type (skip void — implicit in Dart)
    // Wrap in Future<T> for async functions
    let return_prefix = if func.is_async {
        match func.return_type {
            Some(ref rt) if !matches!(rt.kind, TypeKind::Void) => {
                format!("Future<{}> ", dart_type(rt.kind.as_str(), &rt.type_args))
            }
            _ => "Future<void> ".to_string(),
        }
    } else {
        func.return_type
            .as_ref()
            .filter(|rt| !matches!(rt.kind, TypeKind::Void))
            .map(|rt| format!("{} ", dart_type(rt.kind.as_str(), &rt.type_args)))
            .unwrap_or_default()
    };

    let async_kw = if func.is_async { "async " } else { "" };
    let all_params = param_strs.join(", ");
    w.emit_fmt(format_args!(
        "{}{}({}) {}{{",
        return_prefix, func.name, all_params, async_kw
    ));
    w.indent();

    if func.body.statements.is_empty() {
        // Empty body — valid Dart syntax
    } else {
        for stmt in &func.body.statements {
            generate_statement(w, stmt);
        }
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Statement generation
// ═══════════════════════════════════════════════════════════

/// Dart statement formatter.
struct DartStmtFormatter {
    expr: DartFormatExpr,
}

impl FormatStmt for DartStmtFormatter {
    type ExprFormatter = DartFormatExpr;

    fn expr_fmt(&self) -> &Self::ExprFormatter {
        &self.expr
    }

    fn null_value(&self) -> &str {
        "null"
    }

    fn for_in_keyword(&self) -> &str {
        "var"
    }

    fn for_in_uses_of(&self) -> bool {
        false // Dart uses `in`
    }

    fn format_let_stmt(
        &self,
        w: &mut CodeWriter,
        name: &str,
        value: &Option<ExprIR>,
        type_ann: &Option<TypeIR>,
    ) {
        let rhs = value
            .as_ref()
            .map(|v| self.expr.format_expr(v))
            .unwrap_or_else(|| "null".into());
        if let Some(ref ty) = type_ann {
            let ann = dart_type(ty.kind.as_str(), &ty.type_args);
            w.emit_fmt(format_args!("{ann} {name} = {rhs};"));
        } else {
            w.emit_fmt(format_args!("var {name} = {rhs};"));
        }
    }
}

fn generate_statement(w: &mut CodeWriter, stmt: &StatementIR) {
    let fmt = DartStmtFormatter {
        expr: DartFormatExpr,
    };
    fmt.format_stmt(w, stmt);
}
