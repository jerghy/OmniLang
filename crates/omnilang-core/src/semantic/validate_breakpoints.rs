//! Program-level `bp("name")` breakpoint-condition validation (batch 3,
//! spec 2026-08-09 §2.2).
//!
//! Collects the full-program breakpoint table from every module's designs
//! (`DesignIR.breakpoints`, lowered from `breakpoint name: value;` decls),
//! sweeps all expression slots (view tree + computed bodies + event inline
//! fn bodies) and:
//!   - E-401: `bp("name")` referencing an undeclared breakpoint name
//!   - E-405: non-string-literal argument (dynamic expression)
//!   - rewrites the argument to a numeric literal (`768`) so codegen only
//!     needs to handle `name == "bp"` + numeric arg (three-target dispatch).
//!
//! Registered once in `run_program_passes` (program-level, after IR merge —
//! cross-file breakpoint lookup is only visible there; same single-owner
//! model as validate_theme).

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::walk::{walk_expr_mut, walk_view_exprs_mut};
use crate::ir::{ExprIR, IRProgram};
use std::collections::HashMap;

/// Sweep one expression: diagnose (E-401/E-405) and rewrite `bp("name")`.
fn sweep_expr(e: &mut ExprIR, bp_table: &HashMap<String, u32>, diagnostics: &mut Vec<Diagnostic>) {
    if let ExprIR::CallExpr { name, args } = e {
        if name == "bp" {
            match args.first_mut() {
                Some(ExprIR::LiteralExpr {
                    value: serde_json::Value::String(bp_name),
                }) => {
                    match bp_table.get(bp_name) {
                        Some(&value) => {
                            // 改写参数为数字字面量（codegen 只认 name=="bp" + 数字参数）
                            args[0] = ExprIR::LiteralExpr {
                                value: serde_json::Value::Number(serde_json::Number::from(value)),
                            };
                        }
                        None => {
                            diagnostics.push(Diagnostic {
                                severity: Severity::Error,
                                kind: DiagnosticKind::E0006,
                                message: format!(
                                    "bp(\"{bp_name}\") 引用了未声明的断点名 {bp_name}——请在 design 块声明：breakpoint {bp_name}: <数值>;（建议 768）"
                                ),
                                file: None,
                                line: 0,
                                col: 0,
                                end_line: None,
                                end_col: None,
                                suggestion: None,
                            });
                        }
                    }
                }
                _ => {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Error,
                        kind: DiagnosticKind::E0006,
                        message:
                            "bp() 的参数必须是字符串字面量（编译期静态查表），当前为动态表达式"
                                .to_string(),
                        file: None,
                        line: 0,
                        col: 0,
                        end_line: None,
                        end_col: None,
                        suggestion: None,
                    });
                }
            }
        }
    }
}

/// Validate all `bp("name")` calls against the program's breakpoint table.
pub fn validate_breakpoints(program: &mut IRProgram, diagnostics: &mut Vec<Diagnostic>) {
    // 1) 全程序断点常量表（跨模块 designs 合并；重复名 last-wins v0——FxHashMap
    //    insert 天然 last-wins；同 design 块内重名已由 parser E-402 拦截）。
    let mut bp_table: HashMap<String, u32> = HashMap::new();
    for design in program.modules.values().flat_map(|m| m.designs.iter()) {
        for (name, value) in &design.breakpoints {
            bp_table.insert(name.clone(), *value);
        }
    }

    // 2) sweep 所有组件表达式槽位（视图 + computed）。
    //    spec 使用面 = @if 条件 / {} 绑定 / 三元——视图遍历覆盖之（walk_view_
    //    exprs_mut 含元素事件 InlineFn 体）；computed 为既有表达式面（bp 出现
    //    即查表，范围注记：模块级 fn 体不 sweep，与 spec 使用面清单一致）。
    for comp in program
        .modules
        .values_mut()
        .flat_map(|m| m.components.values_mut())
    {
        if let Some(view) = &mut comp.view {
            walk_view_exprs_mut(view, &mut |e| sweep_expr(e, &bp_table, diagnostics));
        }
        for c in &mut comp.computeds {
            walk_expr_mut(&mut c.body, &mut |e| sweep_expr(e, &bp_table, diagnostics));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Severity as Sev;

    fn compile_diags(src: &str) -> Vec<Diagnostic> {
        let result = crate::compile(src);
        result
            .errors
            .into_iter()
            .filter(|d| matches!(d.severity, Sev::Error))
            .collect()
    }

    #[test]
    fn undeclared_bp_is_e401() {
        let src = r#"
            component App {
                view {
                    @if (bp("md")) { <div>desktop</div> }
                }
            }
        "#;
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("未声明的断点名 md")),
            "got: {diags:?}"
        );
    }

    #[test]
    fn declared_bp_passes_and_rewrites_to_number() {
        let src = r#"
            design D { breakpoint md: 768; }
            component App {
                view {
                    @if (bp("md")) { <div>desktop</div> }
                    <div class: {desktopGrid: bp("md"), mobileList: !bp("md")} />
                }
            }
        "#;
        let result = crate::compile(src);
        assert!(result.success, "errors: {:?}", result.errors);
        let ir = result.ir.expect("ir present");
        let mut found_bp = 0;
        for (_, comp) in ir.iter_all_components() {
            if let Some(view) = &comp.view {
                let mut v = view.clone();
                crate::ir::walk::walk_view_exprs_mut(&mut v, &mut |e| {
                    if let ExprIR::CallExpr { name, args } = e {
                        if name == "bp" {
                            found_bp += 1;
                            assert!(
                                matches!(
                                    args.first(),
                                    Some(ExprIR::LiteralExpr {
                                        value: serde_json::Value::Number(n)
                                    }) if n.as_u64() == Some(768)
                                ),
                                "bp arg not rewritten to 768: {args:?}"
                            );
                        }
                    }
                });
            }
        }
        assert!(found_bp >= 2, "bp() calls not found in IR: {found_bp}");
    }

    #[test]
    fn non_literal_bp_arg_is_e405() {
        let src = r#"
            design D { breakpoint md: 768; }
            component App {
                signal bpName: String = "md";
                view {
                    @if (bp(bpName)) { <div>x</div> }
                }
            }
        "#;
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("必须是字符串字面量")),
            "got: {diags:?}"
        );
    }

    #[test]
    fn bp_inside_computed_and_event_are_checked() {
        let src = r#"
            design D { breakpoint md: 768; }
            component App {
                computed isDesktop: Bool = bp("md");
                view {
                    <button @onPress={() => { console.log(bp("lg")); }}>x</button>
                }
            }
        "#;
        // "lg" 未声明 → E-401（computed 与事件体内同样查表）
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("未声明的断点名 lg")),
            "got: {diags:?}"
        );
    }
}
