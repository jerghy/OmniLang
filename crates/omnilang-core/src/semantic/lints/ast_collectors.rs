//! AST name reference collectors for lint rules.
//!
//! Extracted from `helpers.rs` to keep that file below 800 lines.

use crate::parser::ast::*;
use rustc_hash::FxHashSet;

// ── AST name reference collectors ────────────────────

pub(super) fn collect_used_names_in_block<'a>(block: &'a Block, used: &mut FxHashSet<&'a str>) {
    for stmt in &block.stmts {
        match stmt {
            crate::parser::ast::Stmt::VarDecl { init, .. } => {
                if let Some(ref e) = init {
                    collect_used_names_in_expr(e, used);
                }
            }
            crate::parser::ast::Stmt::Assign { value, .. } => {
                collect_used_names_in_expr(value, used);
            }
            Stmt::ExprStmt { expr, .. } => {
                collect_used_names_in_expr(expr, used);
            }
            crate::parser::ast::Stmt::Return { value: Some(e), .. } => {
                collect_used_names_in_expr(e, used);
            }
            crate::parser::ast::Stmt::Return { .. } => {}
            crate::parser::ast::Stmt::If {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                collect_used_names_in_expr(condition, used);
                collect_used_names_in_block(then_body, used);
                for (cond, body) in else_ifs {
                    collect_used_names_in_expr(cond, used);
                    collect_used_names_in_block(body, used);
                }
                if let Some(ref body) = else_body {
                    collect_used_names_in_block(body, used);
                }
            }
            crate::parser::ast::Stmt::While {
                condition, body, ..
            } => {
                collect_used_names_in_expr(condition, used);
                collect_used_names_in_block(body, used);
            }
            crate::parser::ast::Stmt::ForIn { iterable, body, .. } => {
                collect_used_names_in_expr(iterable, used);
                collect_used_names_in_block(body, used);
            }
            crate::parser::ast::Stmt::Print { arg, .. } => {
                collect_used_names_in_expr(arg, used);
            }
            crate::parser::ast::Stmt::Break(_) | crate::parser::ast::Stmt::Continue(_) => {}
            crate::parser::ast::Stmt::EventEmit { args, .. } => {
                for arg in args {
                    collect_used_names_in_expr(arg, used);
                }
            }
            crate::parser::ast::Stmt::Throw { expr, .. } => {
                collect_used_names_in_expr(expr, used);
            }
            crate::parser::ast::Stmt::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                collect_used_names_in_block(try_body, used);
                if let Some(ref b) = catch_body {
                    collect_used_names_in_block(b, used);
                }
                if let Some(ref b) = finally_body {
                    collect_used_names_in_block(b, used);
                }
            }
            crate::parser::ast::Stmt::Error { .. } => {}
        }
    }
    if let Some(ref expr) = block.trailing_expr {
        collect_used_names_in_expr(expr, used);
    }
}

pub(super) fn collect_used_names_in_expr<'a>(expr: &'a Expr, used: &mut FxHashSet<&'a str>) {
    match expr {
        Expr::VarRef(name, _) => {
            used.insert(name.as_str());
        }
        Expr::Literal(..) => {}
        Expr::BinaryOp { left, right, .. } => {
            collect_used_names_in_expr(left, used);
            collect_used_names_in_expr(right, used);
        }
        Expr::UnaryOp { operand, .. } => {
            collect_used_names_in_expr(operand, used);
        }
        Expr::FuncCall { func, args, .. } => {
            collect_used_names_in_expr(func, used);
            for a in args {
                collect_used_names_in_expr(a, used);
            }
        }
        Expr::MethodCall { object, args, .. } => {
            collect_used_names_in_expr(object, used);
            for a in args {
                collect_used_names_in_expr(a, used);
            }
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalChain { object, .. } => {
            collect_used_names_in_expr(object, used);
        }
        Expr::IndexGet { object, index, .. } => {
            collect_used_names_in_expr(object, used);
            collect_used_names_in_expr(index, used);
        }
        Expr::ArrayLiteral { elements, .. } => {
            for e in elements {
                collect_used_names_in_expr(e, used);
            }
        }
        Expr::MapLiteral { entries, .. } => {
            for (k, v) in entries {
                collect_used_names_in_expr(k, used);
                collect_used_names_in_expr(v, used);
            }
        }
        Expr::IfExpr {
            condition,
            then_expr,
            else_expr,
            ..
        }
        | Expr::TernaryExpr {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            collect_used_names_in_expr(condition, used);
            collect_used_names_in_expr(then_expr, used);
            collect_used_names_in_expr(else_expr, used);
        }
        Expr::TemplateString { parts, .. } => {
            for part in parts {
                if let TemplatePart::Expr(e) = part {
                    collect_used_names_in_expr(e, used);
                }
            }
        }
        Expr::StructLiteral {
            type_name, fields, ..
        } => {
            // The struct type name itself is a real reference — an imported
            // struct constructed here must not be flagged unused-import.
            used.insert(type_name.as_str());
            for (_, val) in fields {
                collect_used_names_in_expr(val, used);
            }
        }
        Expr::StoreFieldAccess { .. } => {}
        Expr::ClosureExpr { body, .. } => {
            collect_used_names_in_expr(body, used);
        }
        Expr::AwaitExpr { operand, .. } => {
            collect_used_names_in_expr(operand, used);
        }
        Expr::MatchExpr { matched, arms, .. } => {
            collect_used_names_in_expr(matched, used);
            for arm in arms {
                collect_used_names_in_block(&arm.body, used);
            }
        }
        Expr::Block { .. } => {}
        Expr::IsCheck { .. } => {}
        Expr::SpreadExpr { operand, .. } => collect_used_names_in_expr(operand, used),
        Expr::NotYetImplemented { .. } => {}
        Expr::Error { .. } => {}
    }
}

/// Collect name references from a motion definition body. Keyframe
/// property values (`from { translateX: offsetX }`) are parsed as plain
/// expressions (parser/top_level/motion.rs), as are the duration
/// (`over: <expr>`) and playback delay (`delay <expr>`) — all are real
/// reference sites. Easing exprs and stop positions carry no Expr fields.
pub(super) fn collect_used_names_in_motion_def<'a>(
    motion: &'a MotionDef,
    used: &mut FxHashSet<&'a str>,
) {
    match &motion.body {
        MotionBody::Keyframe(kf) => {
            for stop in &kf.stops {
                for (_, value) in &stop.properties {
                    collect_used_names_in_expr(value, used);
                }
            }
            collect_used_names_in_expr(&kf.over, used);
            for prop in &kf.playback {
                if let PlaybackProp::Delay(e) = prop {
                    collect_used_names_in_expr(e, used);
                }
            }
        }
    }
}

pub(super) fn collect_used_names_in_view<'a>(view: &'a ViewBlock, used: &mut FxHashSet<&'a str>) {
    for node in &view.children {
        collect_used_names_in_view_node(node, used);
    }
}

pub(super) fn collect_used_names_in_view_node<'a>(
    node: &'a ViewNode,
    used: &mut FxHashSet<&'a str>,
) {
    match node {
        ViewNode::Element(el) => {
            // Element tags are real name references: HTML tags (`div`) and
            // imported component tags (`<Overview />`) alike. Inserting HTML
            // tags is harmless (no import can be named "div"); component tags
            // are exactly how imported components get used.
            used.insert(el.tag.as_str());
            for prop in &el.props {
                match prop {
                    ElementProp::Static { value, .. } => {
                        collect_used_names_in_expr(value, used);
                    }
                    // Inline style entries (`style: { color: accentColor }`) are
                    // real reference sites — their values are plain expressions.
                    ElementProp::Style { entries } => {
                        for (_, value) in entries {
                            collect_used_names_in_expr(value, used);
                        }
                    }
                    // Dynamic class maps (`class: { visible: isOpen }`) hold
                    // expression values that may reference signals.
                    ElementProp::Class {
                        value: ClassValue::ClassMap(entries),
                    } => {
                        for (_, value) in entries {
                            collect_used_names_in_expr(value, used);
                        }
                    }
                    // Design class references (`class: LearnDesign.card`) are
                    // real uses of the imported design module.
                    ElementProp::Class {
                        value: ClassValue::DesignClass { design_name, .. },
                    } => {
                        used.insert(design_name.as_str());
                    }
                    // Variant references (`variant: LearnDesign.Button(...)`)
                    // likewise name the imported design module.
                    ElementProp::Variant { design_name, .. } => {
                        used.insert(design_name.as_str());
                    }
                    // a11y blocks hold expression-valued entries
                    // (`hidden`/`alt`) that may reference signals.
                    ElementProp::A11y(spec) => {
                        if let Some(ref e) = spec.hidden {
                            collect_used_names_in_expr(e, used);
                        }
                        if let Some(ref e) = spec.alt {
                            collect_used_names_in_expr(e, used);
                        }
                    }
                    ElementProp::Bind { signal_path, .. } => {
                        if let Some(dot) = signal_path.find('.') {
                            used.insert(&signal_path[..dot]);
                        } else {
                            used.insert(signal_path.as_str());
                        }
                    }
                    ElementProp::RefBinding { signal, .. } => {
                        collect_used_names_in_expr(signal, used);
                    }
                    ElementProp::Event { handler, .. } => match handler {
                        EventHandler::InlineFn { body, .. } => {
                            collect_used_names_in_block(body, used);
                        }
                        EventHandler::FnRef(name) => {
                            used.insert(name.as_str());
                        }
                        EventHandler::StoreAction { store, action, .. } => {
                            used.insert(store.as_str());
                            used.insert(action.as_str());
                        }
                    },
                    // Consumer slot projection (ADR-053 D5): the content is a
                    // view subtree holding real references.
                    ElementProp::SlotContent { content, .. } => {
                        for node in content {
                            collect_used_names_in_view_node(node, used);
                        }
                    }
                    // Motion triggers have two forms: a named-motion reference
                    // (@enter={fadeIn} — motions are canonical in ModuleIR, not
                    // subject to the per-component dead-code check) and an
                    // inline tween body (@enter={{ translateX: offsetX }} —
                    // parsed as a MapLiteral) whose property values may
                    // reference signals. Keys are property names (no signal
                    // references), so only values are collected.
                    ElementProp::MotionTrigger(mt) => {
                        if let Expr::MapLiteral { entries, .. } = &mt.motion_expr {
                            for (_, value) in entries {
                                collect_used_names_in_expr(value, used);
                            }
                        }
                    }
                    // @drag/@dragEnd handler names are real uses of the
                    // component functions they name (ADR-073).
                    ElementProp::Drag {
                        on_drag,
                        on_drag_end,
                    } => {
                        used.insert(on_drag.as_str());
                        if let Some(ref end) = on_drag_end {
                            used.insert(end.as_str());
                        }
                    }
                    // Remaining variants carry no signal-referencing
                    // expressions: ClassValue::StyleRef/StringLiteral (local
                    // style names) and Ref (element refs).
                    _ => {}
                }
            }
            for child in &el.children {
                collect_used_names_in_view_node(child, used);
            }
        }
        ViewNode::Text(_) => {}
        ViewNode::Interpolation(expr) => {
            collect_used_names_in_expr(expr, used);
        }
        ViewNode::IfDirective(if_dir) => {
            collect_used_names_in_expr(&if_dir.condition, used);
            for node in &if_dir.body {
                collect_used_names_in_view_node(node, used);
            }
            for (cond, body) in &if_dir.else_ifs {
                collect_used_names_in_expr(cond, used);
                for node in body {
                    collect_used_names_in_view_node(node, used);
                }
            }
            if let Some(ref body) = if_dir.else_body {
                for node in body {
                    collect_used_names_in_view_node(node, used);
                }
            }
        }
        ViewNode::ForDirective(for_dir) => {
            collect_used_names_in_expr(&for_dir.iterable, used);
            for node in &for_dir.body {
                collect_used_names_in_view_node(node, used);
            }
            if let Some(ref empty_body) = for_dir.empty_body {
                for node in empty_body {
                    collect_used_names_in_view_node(node, used);
                }
            }
        }
        ViewNode::SlotDirective {
            default_content, ..
        } => {
            // View-level @slot fallback content is real view content.
            if let Some(ref content) = default_content {
                for node in content {
                    collect_used_names_in_view_node(node, used);
                }
            }
        }
    }
}
