// ═══════════════════════════════════════════════════════════
// Computed value codegen: computed → useMemo / useCallback
// ═══════════════════════════════════════════════════════════

use crate::codegen::react::expr;
use crate::codegen::react::utils;
use crate::codegen::react::ReactStoreMode;
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashSet;

/// Generate useMemo / useCallback hooks for all @computed declarations.
pub(super) fn gen_computed_values(
    w: &mut CodeWriter,
    comp: &ComponentIR,
    known_signal_names: &[String],
    signal_names: &FxHashSet<String>,
    store_mode: ReactStoreMode,
    ref_names: &FxHashSet<String>,
) {
    for computed in &comp.computeds {
        let mut deps_list: Vec<&str> = Vec::new();
        for sig_name in known_signal_names {
            if utils::getter_expr_contains_ident(&computed.body, sig_name) {
                deps_list.push(sig_name.as_str());
            }
        }

        // Function-form computed: `computed name() -> Type { ... }` → useCallback
        // (detected by BlockExpr with ReturnStmtIR in statements)
        let is_fn_form = match &computed.body {
            ExprIR::BlockExpr { statements, .. } => statements
                .iter()
                .any(|s| matches!(s, StatementIR::ReturnStmtIR { .. })),
            _ => false,
        };

        if is_fn_form {
            w.emit_fmt(format_args!(
                "const {} = useCallback(() => {{",
                computed.name
            ));
        } else {
            w.emit_fmt(format_args!("const {} = useMemo(() => {{", computed.name));
        }
        w.indent();
        if let ExprIR::BlockExpr { statements, tail } = &computed.body {
            // Render preceding statements (let declarations, if/return, etc.)
            for stmt in statements {
                expr::generate_statement_tsx_with_mode(
                    w,
                    stmt,
                    signal_names,
                    store_mode,
                    ref_names,
                );
            }
            // Render tail as return value if non-null
            // (null tail means the block has explicit return statements)
            if !matches!(
                tail.as_ref(),
                ExprIR::LiteralExpr { value } if value == &serde_json::Value::Null
            ) {
                w.emit_fmt(format_args!(
                    "return {};",
                    expr::format_expr_ir_tsx(tail, ref_names)
                ));
            }
        } else {
            w.emit_fmt(format_args!(
                "return {};",
                expr::format_expr_ir_tsx(&computed.body, ref_names)
            ));
        }
        w.dedent();
        w.emit_fmt(format_args!("}}, [{}]);", deps_list.join(", ")));
    }
}
