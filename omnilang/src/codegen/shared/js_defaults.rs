//! JS-family shared generation defaults.
//!
//! Free functions that implement common generation logic for JavaScript-family
//! backends (JavaScript, TypeScript, React, Vue, OmniView).
//!
//! Each function is generic over [`UnifiedCodegen`] so it works with any backend
//! that uses the JS-family expression formatter.  Backends call these from their
//! trait method overrides to share the common structural patterns (function
//! declarations, import/export syntax, etc.).

use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::shared::types::FormatType;
use crate::codegen::traits::UnifiedCodegen;
use crate::codegen::writer::CodeWriter;
use crate::ir::*;

/// Generate a function definition using JS-family syntax.
///
/// Produces:
/// ```js
/// function name(param1, param2) {
///     body
/// }
/// ```
///
/// In TypeScript mode, type annotations are appended to parameters and the
/// return type.
pub fn generate_function<C: UnifiedCodegen>(codegen: &C, w: &mut CodeWriter, f: &FunctionIR) {
    let params: Vec<String> = f
        .params
        .iter()
        .map(|p| {
            let id = codegen.expr_fmt().format_identifier(&p.name);
            if let Some(ty) = &p.r#type {
                if codegen.expr_fmt().ts_mode() {
                    format!(
                        "{}: {}",
                        id,
                        codegen.type_fmt().format_type(ty.kind.as_str(), &ty.type_args)
                    )
                } else {
                    id
                }
            } else {
                id
            }
        })
        .collect();

    let return_ty = f
        .return_type
        .as_ref()
        .filter(|_| codegen.expr_fmt().ts_mode())
        .map(|t| {
            format!(
                ": {}",
                codegen.type_fmt().format_type(t.kind.as_str(), &t.type_args)
            )
        })
        .unwrap_or_default();

    let params_str = params.join(", ");

    w.emit_fmt(format_args!(
        "function {}({params_str}){return_ty} {{",
        f.name
    ));
    w.indent();
    codegen.stmt_fmt().format_block(w, &f.body);
    w.dedent();
    w.emit("}");
}

/// Generate a component function using JS-family syntax.
///
/// Produces a function component:
/// ```tsx
/// export function TodoItem(props: TodoItemProps) {
///     // ... component body
/// }
/// ```
pub fn generate_component<C: UnifiedCodegen>(
    codegen: &C,
    w: &mut CodeWriter,
    name: &str,
    comp: &ComponentIR,
) {
    let props_ty = if codegen.expr_fmt().ts_mode() {
        format!("props: {}Props", name)
    } else {
        "props".to_string()
    };

    w.emit_fmt(format_args!("export function {name}({props_ty}) {{"));
    w.indent();
    for state in &comp.states {
        let init = codegen.expr_fmt().format_expr(&state.initial_value);
        w.emit_fmt(format_args!("let {} = $signal({init});", state.name));
    }
    for prop_name in comp.props.keys() {
        w.emit_fmt(format_args!("let {prop_name} = props.{prop_name};"));
    }
    w.dedent();
    w.emit("}");
}

/// Generate an import statement for JS-family syntax.
pub fn generate_import<C: UnifiedCodegen>(
    codegen: &C,
    w: &mut CodeWriter,
    source: &str,
    imports: &[String],
) {
    let items = imports.join(", ");
    w.emit_fmt(format_args!(
        "import {{ {} }} from \"{}\";",
        items,
        source
    ));
    let _ = codegen; // kept for future use (e.g., handling .js vs .ts extensions)
}
