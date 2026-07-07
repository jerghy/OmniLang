//! Dart-family shared generation defaults.
//!
//! Free functions that implement common generation logic for Dart-family
//! backends (Dart, Flutter).
//!
//! Each function is generic over [`UnifiedCodegen`] so it works with any backend
//! that uses the Dart-family formatters.  Backends call these from their trait
//! method overrides to share the common structural patterns.

use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::shared::types::FormatType;
use crate::codegen::traits::UnifiedCodegen;
use crate::codegen::writer::CodeWriter;
use crate::ir::*;

/// Generate a function definition using Dart-family syntax.
///
/// Produces:
/// ```dart
/// ReturnType name(ParamType param1, ParamType param2) {
///     body
/// }
/// ```
///
/// Type annotations are always emitted (Dart is statically typed).
pub fn generate_function<C: UnifiedCodegen>(codegen: &C, w: &mut CodeWriter, f: &FunctionIR) {
    let return_type = f
        .return_type
        .as_ref()
        .map(|t| {
            format!(
                "{} ",
                codegen.type_fmt().format_type(t.kind.as_str(), &t.type_args)
            )
        })
        .unwrap_or_else(|| "void ".into());

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
                .unwrap_or_else(|| "dynamic".into());
            format!("{} {}", type_str, id)
        })
        .collect();

    w.emit_fmt(format_args!(
        "{return_type}{}({}) {{",
        f.name,
        params.join(", ")
    ));
    w.indent();
    codegen.stmt_fmt().format_block(w, &f.body);
    w.dedent();
    w.emit("}");
}

/// Generate a class-based component using Dart/Flutter syntax.
///
/// Produces a minimal StatelessWidget scaffold.
pub fn generate_component<C: UnifiedCodegen>(
    codegen: &C,
    w: &mut CodeWriter,
    name: &str,
    comp: &ComponentIR,
) {
    w.emit_fmt(format_args!("class {name} extends StatelessWidget {{"));
    w.indent();
    w.emit_fmt(format_args!("const {name}({{super.key}});"));
    w.emit("");
    w.emit_fmt(format_args!(
        "@override\nWidget build(BuildContext context) {{"
    ));
    w.indent();
    for state in &comp.states {
        let init = codegen.expr_fmt().format_expr(&state.initial_value);
        w.emit_fmt(format_args!("var {} = {};", state.name, init));
    }
    w.emit("return Container();");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");
}
