//! Rust-family shared generation defaults.
//!
//! Free functions that implement common generation logic for Rust-family
//! backends (Rust, Dioxus).
//!
//! Each function is generic over [`UnifiedCodegen`] so it works with any backend
//! that uses the Rust-family formatters.  Backends call these from their trait
//! method overrides to share the common structural patterns.

use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::shared::types::FormatType;
use crate::codegen::traits::UnifiedCodegen;
use crate::codegen::writer::CodeWriter;
use crate::ir::*;

/// Generate a function definition using Rust-family syntax.
///
/// Produces:
/// ```rust
/// fn name(param1: Type1, param2: Type2) -> ReturnType {
///     body
/// }
/// ```
pub fn generate_function<C: UnifiedCodegen>(codegen: &C, w: &mut CodeWriter, f: &FunctionIR) {
    let return_type = f
        .return_type
        .as_ref()
        .map(|t| {
            format!(
                " -> {}",
                codegen.type_fmt().format_type(t.kind.as_str(), &t.type_args)
            )
        })
        .unwrap_or_default();

    let params: Vec<String> = f
        .params
        .iter()
        .map(|p| {
            let id = codegen.expr_fmt().format_identifier(&p.name);
            let type_str = p
                .r#type
                .as_ref()
                .map(|t| {
                    codegen.type_fmt().format_type(t.kind.as_str(), &t.type_args)
                })
                .unwrap_or_else(|| "()".into());
            format!("{id}: {type_str}")
        })
        .collect();

    let params_str = params.join(", ");

    w.emit_fmt(format_args!(
        "fn {}({params_str}){return_type} {{",
        f.name
    ));
    w.indent();
    codegen.stmt_fmt().format_block(w, &f.body);
    w.dedent();
    w.emit("}");
}

/// Generate a component using Dioxus/Rust syntax.
pub fn generate_component<C: UnifiedCodegen>(
    codegen: &C,
    w: &mut CodeWriter,
    name: &str,
    comp: &ComponentIR,
) {
    w.emit_fmt(format_args!(
        "#[component]\npub fn {name}(cx: Scope) -> Element {{"
    ));
    w.indent();
    for state in &comp.states {
        let init = codegen.expr_fmt().format_expr(&state.initial_value);
        w.emit_fmt(format_args!(
            "let {} = use_state(cx, || {});",
            state.name, init
        ));
    }
    w.emit("render! {");
    w.indent();
    w.emit_fmt(format_args!("div {{ \"{name}\" }}"));
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");
}
