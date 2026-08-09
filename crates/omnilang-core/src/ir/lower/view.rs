//! Lowering for view blocks and view nodes.
//!
//! Includes: `lower_view_block`, `lower_view_node`.

use super::expr::lower_expr;
use super::stmt::lower_block;
use crate::error::{Diagnostic, DiagnosticKind};
use crate::ir::logic::FunctionIR;
use crate::ir::types::*;
use crate::ir::types_motion::*;
use crate::ir::walk;
use crate::ir::TriggerKind;
use crate::lexer::Span;
use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

// ═══════════════════════════════════════════════════════
// View lowering
// ═══════════════════════════════════════════════════════

pub(super) fn lower_view_block(
    view: &ViewBlock,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    fns: &[FunctionIR],
    signal_like_names: &FxHashSet<String>,
) -> ViewTree {
    if view.children.is_empty() {
        return ViewTree::Text("".into());
    }
    let mut lowered: Vec<ViewTree> = Vec::with_capacity(view.children.len());
    for child in &view.children {
        lowered.push(lower_view_node(
            child,
            variant_map,
            semantics,
            fns,
            signal_like_names,
        ));
    }
    // A single child lowers to that child directly; multiple top-level view
    // nodes lower to a fragment so no node is silently dropped (previously
    // only `children.first()` was lowered — multi-root views lost everything
    // after the first element).
    if lowered.len() == 1 {
        lowered.pop().expect("len == 1")
    } else {
        ViewTree::Fragment(lowered)
    }
}

/// Scan an event handler AST block for `e.preventDefault()` and
/// `e.stopPropagation()` method calls.  Returns (needs_prevent_default,
/// needs_stop_propagation).
///
/// This replaces the field-collection half of the old `EventFieldCollector`;
/// event field names are now collected during `lower_expr` traversal.
fn scan_event_methods(block: &Block, event_param: &str) -> (bool, bool) {
    let mut needs_pd = false;
    let mut needs_sp = false;
    for stmt in &block.stmts {
        scan_stmt_for_event_methods(stmt, &mut needs_pd, &mut needs_sp, event_param);
    }
    if let Some(ref expr) = block.trailing_expr {
        scan_expr_for_event_methods(expr, &mut needs_pd, &mut needs_sp, event_param);
    }
    (needs_pd, needs_sp)
}

fn scan_stmt_for_event_methods(stmt: &Stmt, pd: &mut bool, sp: &mut bool, event_param: &str) {
    match stmt {
        Stmt::ExprStmt { expr, .. } => scan_expr_for_event_methods(expr, pd, sp, event_param),
        Stmt::VarDecl {
            init: Some(init), ..
        } => scan_expr_for_event_methods(init, pd, sp, event_param),
        Stmt::Assign { value, .. } => scan_expr_for_event_methods(value, pd, sp, event_param),
        Stmt::If {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            scan_expr_for_event_methods(condition, pd, sp, event_param);
            for s in &then_body.stmts {
                scan_stmt_for_event_methods(s, pd, sp, event_param);
            }
            for (_, elsif_block) in else_ifs {
                for s in &elsif_block.stmts {
                    scan_stmt_for_event_methods(s, pd, sp, event_param);
                }
            }
            if let Some(else_b) = else_body {
                for s in &else_b.stmts {
                    scan_stmt_for_event_methods(s, pd, sp, event_param);
                }
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            scan_expr_for_event_methods(condition, pd, sp, event_param);
            for s in &body.stmts {
                scan_stmt_for_event_methods(s, pd, sp, event_param);
            }
        }
        Stmt::ForIn { iterable, body, .. } => {
            scan_expr_for_event_methods(iterable, pd, sp, event_param);
            for s in &body.stmts {
                scan_stmt_for_event_methods(s, pd, sp, event_param);
            }
        }
        Stmt::Return { value: Some(v), .. } => scan_expr_for_event_methods(v, pd, sp, event_param),
        Stmt::Print { arg, .. } => scan_expr_for_event_methods(arg, pd, sp, event_param),
        Stmt::Throw { expr, .. } => scan_expr_for_event_methods(expr, pd, sp, event_param),
        Stmt::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.stmts {
                scan_stmt_for_event_methods(s, pd, sp, event_param);
            }
            if let Some(catch_b) = catch_body {
                for s in &catch_b.stmts {
                    scan_stmt_for_event_methods(s, pd, sp, event_param);
                }
            }
            if let Some(finally_b) = finally_body {
                for s in &finally_b.stmts {
                    scan_stmt_for_event_methods(s, pd, sp, event_param);
                }
            }
        }
        _ => {}
    }
}

fn scan_expr_for_event_methods(expr: &Expr, pd: &mut bool, sp: &mut bool, event_param: &str) {
    match expr {
        Expr::MethodCall { object, method, .. } => {
            if let Expr::VarRef(name, _) = object.as_ref() {
                if name == event_param {
                    match method.as_str() {
                        "preventDefault" => *pd = true,
                        "stopPropagation" => *sp = true,
                        _ => {}
                    }
                }
            }
        }
        // Recurse into expressions that may contain nested method calls
        Expr::BinaryOp { left, right, .. } => {
            scan_expr_for_event_methods(left, pd, sp, event_param);
            scan_expr_for_event_methods(right, pd, sp, event_param);
        }
        Expr::UnaryOp { operand, .. } => scan_expr_for_event_methods(operand, pd, sp, event_param),
        Expr::FuncCall { func, args, .. } => {
            scan_expr_for_event_methods(func, pd, sp, event_param);
            for arg in args {
                scan_expr_for_event_methods(arg, pd, sp, event_param);
            }
        }
        Expr::Block { body, .. } => {
            for s in &body.stmts {
                scan_stmt_for_event_methods(s, pd, sp, event_param);
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
            scan_expr_for_event_methods(condition, pd, sp, event_param);
            scan_expr_for_event_methods(then_expr, pd, sp, event_param);
            scan_expr_for_event_methods(else_expr, pd, sp, event_param);
        }
        _ => {}
    }
}

pub(super) fn lower_view_node(
    node: &ViewNode,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    fns: &[FunctionIR],
    signal_like_names: &FxHashSet<String>,
) -> ViewTree {
    match node {
        ViewNode::Element(el) => lower_element(el, variant_map, semantics, fns, signal_like_names),
        ViewNode::Text(s) => ViewTree::Text(s.clone()),
        ViewNode::Interpolation(expr) => {
            // Lower the inner expression to ExprIR so codegen can emit proper JS
            ViewTree::Interpolation(Box::new(lower_expr(expr, variant_map, None, semantics)))
        }
        ViewNode::IfDirective(if_dir) => {
            lower_if_directive(if_dir, variant_map, semantics, fns, signal_like_names)
        }
        ViewNode::ForDirective(for_dir) => {
            lower_for_directive(for_dir, variant_map, semantics, fns, signal_like_names)
        }
        ViewNode::SlotDirective {
            name,
            default_content,
        } => {
            let shape = SlotDeclShape {
                name,
                default_content,
            };
            lower_slot_common(&shape, variant_map, semantics, fns, signal_like_names)
                .into_view_tree()
        }
        // @load={fnName} (data three-state, spec 2026-08-08 §4.1) is
        // lowered to LoadIntentIR in Task 2 — until then it contributes
        // no view output (empty fragment keeps codegen running).
        ViewNode::LoadDirective { .. } => ViewTree::Fragment(vec![]),
    }
}

/// Shared slot declaration shape passed to `lower_slot_common` — used by
/// both view-level `@slot` directives (`ViewNode::SlotDirective`) and
/// component body slot declarations (`SlotDecl`), ADR-053 D5.
pub(super) struct SlotDeclShape<'a> {
    pub name: &'a Option<String>,
    pub default_content: &'a Option<Vec<ViewNode>>,
}

/// Lower the shared slot declaration shape (name + fallback) into
/// `SlotDeclIR`. v2 (design D4): no type constraint / data_param — slot =
/// parent-provided content only.
pub(super) fn lower_slot_common(
    slot: &SlotDeclShape,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    fns: &[FunctionIR],
    signal_like_names: &FxHashSet<String>,
) -> SlotDeclIR {
    let fallback = slot
        .default_content
        .as_ref()
        .map(|nodes| {
            nodes
                .iter()
                .map(|n| lower_view_node(n, variant_map, semantics, fns, signal_like_names))
                .collect()
        })
        .unwrap_or_default();
    SlotDeclIR {
        name: slot.name.clone(),
        fallback,
    }
}

// ── Element lowering ──────────────────────────────────────

/// Lowers an `Element` AST node into `ViewTree::Element(ElementIR)`.
/// Handles props, bindings, events, class/style, motion, state, scroll, drag,
/// and hero attributes.
/// Class names reaching codegen are restricted to `[a-zA-Z0-9_-]` — the same
/// charset policy as the design parser's identifier rule. Quoted
/// `class: "..."` strings are free-form text, so any payload that would break
/// out of the emitted `styles['...']` / `"..."` literals (e.g.
/// `class: "x',alert(1),'"` → `styles['x',alert(1),'']`, comma operator) is
/// rejected here with a hard error — code-injection double-insurance,
/// layer 1 (codegen escaping is layer 2). Invalid tokens are dropped from
/// the IR so even a failed compile's output never carries the payload.
fn validate_class_names(class_str: &str, span: &Span, semantics: &mut SemanticResult) -> String {
    let mut valid: Vec<&str> = Vec::new();
    for token in class_str.split_whitespace() {
        if token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            valid.push(token);
        } else {
            semantics.diagnostics.push(Diagnostic::error(
                DiagnosticKind::E0006,
                &format!("类名仅允许字母数字下划线连字符：\"{}\"", token),
                span,
            ));
        }
    }
    valid.join(" ")
}

/// Full WHATWG HTML event-handler content attribute set (spec §8.1.6.1
/// "Event handlers", incl. the window/body-only handlers). The rejection
/// gate matches these names ONLY — a generic `on[a-z]+=` regex would
/// false-positive on legitimate values like `done=` / `gone=` (which
/// contain the `one=` substring).
const HTML_HANDLER_ATTRS: &[&str] = &[
    "onabort",
    "onauxclick",
    "onbeforeinput",
    "onbeforematch",
    "onbeforetoggle",
    "onblur",
    "oncancel",
    "oncanplay",
    "oncanplaythrough",
    "onchange",
    "onclick",
    "onclose",
    "oncontextlost",
    "oncontextmenu",
    "oncontextrestored",
    "oncopy",
    "oncuechange",
    "oncut",
    "ondblclick",
    "ondrag",
    "ondragend",
    "ondragenter",
    "ondragleave",
    "ondragover",
    "ondragstart",
    "ondrop",
    "ondurationchange",
    "onemptied",
    "onended",
    "onerror",
    "onfocus",
    "onformdata",
    "oninput",
    "oninvalid",
    "onkeydown",
    "onkeypress",
    "onkeyup",
    "onload",
    "onloadeddata",
    "onloadedmetadata",
    "onloadstart",
    "onmousedown",
    "onmouseenter",
    "onmouseleave",
    "onmousemove",
    "onmouseout",
    "onmouseover",
    "onmouseup",
    "onpaste",
    "onpause",
    "onplay",
    "onplaying",
    "onprogress",
    "onratechange",
    "onreset",
    "onresize",
    "onscroll",
    "onscrollend",
    "onsecuritypolicyviolation",
    "onseeked",
    "onseeking",
    "onselect",
    "onselectionchange",
    "onselectstart",
    "onslotchange",
    "onstalled",
    "onsubmit",
    "onsuspend",
    "ontimeupdate",
    "ontoggle",
    "onvolumechange",
    "onwaiting",
    "onwheel",
    "onanimationcancel",
    "onanimationend",
    "onanimationiteration",
    "onanimationstart",
    "onpointercancel",
    "onpointerdown",
    "onpointerenter",
    "onpointerleave",
    "onpointermove",
    "onpointerout",
    "onpointerover",
    "onpointerrawupdate",
    "onpointerup",
    "ontouchcancel",
    "ontouchend",
    "ontouchmove",
    "ontouchstart",
    "ontransitioncancel",
    "ontransitionend",
    "ontransitionrun",
    "ontransitionstart",
    "onafterprint",
    "onbeforeprint",
    "onbeforeunload",
    "onhashchange",
    "onlanguagechange",
    "onmessage",
    "onmessageerror",
    "onoffline",
    "ononline",
    "onpagehide",
    "onpageshow",
    "onpopstate",
    "onrejectionhandled",
    "onstorage",
    "onunhandledrejection",
    "onunload",
];

/// Diagnostic message fragment for the unquoted-handler rejection gate.
pub(crate) const HANDLER_BREAKOUT_MSG: &str = "静态属性值禁止包含 HTML 事件处理器形状";

/// A STATIC attribute value carrying an `onXxx=` sequence (a WHATWG event
/// handler attribute name followed by `=`, boundary-checked) would, if the
/// emission were ever unquoted, parse as a REAL inline handler attribute —
/// and even quoted it keeps the exploitable shape in the HTML artifact
/// (security HIGH-2, `<div title="x onmouseover=alert(1)"/>`). Escaping
/// cannot eliminate it (the payload has no chars beyond the `&"'< >` escape
/// set), so the gate REJECTS such static values at compile time — a
/// compile error is the accepted safe outcome. Runs in shared lowering, so
/// every backend gets the same rejection (no platform asymmetry). Only
/// static string literals are gated; dynamic values (expression
/// interpolation) are escaped at runtime by a different mechanism. Mirrors
/// `validate_class_names`: the offending value is sanitized to the empty
/// string so even a failed compile's output never carries the shape.
fn validate_attr_no_handler_breakout(
    value: &str,
    span: &Span,
    semantics: &mut SemanticResult,
) -> String {
    let bytes = value.as_bytes();
    for handler in HTML_HANDLER_ATTRS {
        let name = handler.as_bytes();
        let mut i = 0;
        while i + name.len() <= bytes.len() {
            if &bytes[i..i + name.len()] == name {
                // Boundary: the name must not be part of a longer identifier
                // (`nonmouseover=`, `onmouseoveralert=`) — only a
                // whitespace-delimited attribute name followed by `=`
                // (optional whitespace allowed, as in HTML) is a handler.
                let before_ok = i == 0
                    || !(bytes[i - 1].is_ascii_alphanumeric()
                        || bytes[i - 1] == b'_'
                        || bytes[i - 1] == b'-');
                let mut j = i + name.len();
                while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                if before_ok && j < bytes.len() && bytes[j] == b'=' {
                    semantics.diagnostics.push(Diagnostic::error(
                        DiagnosticKind::E0006,
                        &format!("{}：\"{}\"", HANDLER_BREAKOUT_MSG, handler),
                        span,
                    ));
                    return String::new();
                }
            }
            i += 1;
        }
    }
    value.to_string()
}

fn lower_element(
    el: &Element,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    fns: &[FunctionIR],
    signal_like_names: &FxHashSet<String>,
) -> ViewTree {
    let mut props: Option<FxHashMap<String, ExprIR>> = None;
    let mut bindings: Option<FxHashMap<String, SignalRefIR>> = None;
    let mut events: Option<FxHashMap<String, EventHandlerIR>> = None;
    let mut inline_style: Option<serde_json::Value> = None;
    let mut class_map_bindings_from_prop: Option<FxHashMap<String, ExprIR>> = None;
    let mut motion_triggers: Vec<TriggerIntent> = Vec::new();
    // The old @-prefixed sub-attribute tokens (in-view once/repeat/threshold,
    // scroll start/end/scrub, drag constraints/elastic/scrub/momentum/
    // threshold/velocity-max) were removed with the semantic-level DSL
    // (§3.2, ADR-053). The IR behavior structs keep their fields; Task 3.4
    // fills the in-view ones from the TRIGGER SET after the prop loop below
    // (an @in-view trigger implies once-semantics — plays on first
    // visibility, mirroring React's IntersectionObserver disconnect and the
    // Flutter runtime's once: true default). paused_override was deleted
    // outright (ADR-053).
    // Note: the former scroll_start/scroll_end/scroll_scrub/drag_* locals
    // (fed into ScrollBehavior/DragBehavior) were deleted with those IR
    // structs (Task 5.1) — the v1 control dimensions are carried directly on
    // TriggerKind::Scroll / Drag* and lowered below.
    let mut ref_binding: Option<String> = None;
    let mut semantic_bindings: Option<FxHashMap<String, ExprIR>> = None;
    let mut a11y: Option<A11yIR> = None;
    let mut drag_binding: Option<DragBindingIR> = None;
    // Named slot projections (ADR-053 D5): `header={<div/>}` on a component
    // call lowers to `ViewTree::Slot { name: Some(name), fallback: content }`
    // inserted into the element's children — codegen consumes the projection
    // from the consumer position exactly like a view-level `@slot name {...}`.
    let mut slot_projections: Vec<ViewTree> = Vec::new();
    // 批次 1 反馈模态绑定（域 10，spec 2026-08-08）：@modal/@skeleton/
    // @errorBoundary 的记录局部。open_signal 恒为信号名——解析器在
    // directives.rs 强制 `@modal={IDENT}` 形状（ElementProp::Modal 的
    // open_signal 只可能是 Expr::VarRef）；@errorBoundary 的命名槽
    // `@slot fallback` 在 children 收尾扫描中提取。
    let mut modal_open_signal: Option<String> = None;
    let mut skeleton_shape: Option<String> = None;
    // 裸 `@skeleton`（无 shape，默认 text）与「无 @skeleton 属性」必须可区分：
    // shape 为 None 是合法有语义的 IR 值（SkeletonBindingIR 文档），故单独记
    // 出现标记，禁止用 Option::map 把裸 @skeleton 折叠成 None。
    let mut has_skeleton = false;
    let mut has_error_boundary = false;

    for prop in &el.props {
        match prop {
            ElementProp::Static { name, value } => {
                let lowered = match value {
                    Expr::Literal(Literal::String(s), span) => ExprIR::LiteralExpr {
                        value: serde_json::Value::String(validate_attr_no_handler_breakout(
                            s, span, semantics,
                        )),
                    },
                    _ => lower_expr(value, variant_map, None, semantics),
                };
                props
                    .get_or_insert_with(Default::default)
                    .insert(name.clone(), lowered);
            }
            ElementProp::Bind { name, signal_path } => {
                bindings.get_or_insert_with(Default::default).insert(
                    name.clone(),
                    SignalRefIR {
                        name: signal_path.clone(),
                    },
                );
            }
            ElementProp::Event { name, handler } => {
                // Collect event data kind
                let event_data = map_event_data_kind(name);
                let mut needs_pd = false;
                let mut needs_sp = false;
                let ir_handler = match handler {
                    EventHandler::InlineFn { params, body, .. } => {
                        let event_param = params.first().map(|p| p.name.as_str()).unwrap_or("e");
                        if event_data.is_some() {
                            let (needs_pd_from_scan, needs_sp_from_scan) =
                                scan_event_methods(body, event_param);
                            needs_pd = needs_pd || needs_pd_from_scan;
                            needs_sp = needs_sp_from_scan;

                            // Collect event field names during lowering instead
                            // of a separate pre-walk.
                            let mut event_fields = FxHashSet::default();
                            // Save and set event param name so lower_expr can detect field access
                            let prev_param = semantics.event_param_name.take();
                            semantics.event_param_name = Some(event_param.to_string());
                            // 批次 1 执行体标记（spec §2.2 规则 3）：内联事件处理器
                            // 是 showToast 的合法调用处，lowering 期间置位，结束后恢复。
                            let prev_in_execution_body = semantics.in_execution_body;
                            semantics.in_execution_body = true;
                            let block_ir =
                                lower_block(body, variant_map, Some(&mut event_fields), semantics);
                            // Restore previous event param name
                            semantics.event_param_name = prev_param;
                            semantics.in_execution_body = prev_in_execution_body;

                            EventHandlerIR::InlineFn {
                                body: block_ir,
                                params: params.iter().map(|p| p.name.clone()).collect(),
                                event_data: event_data.clone(),
                                used_fields: Some(event_fields.into_iter().collect()),
                                needs_prevent_default: needs_pd,
                                needs_stop_propagation: needs_sp,
                            }
                        } else {
                            // 批次 1 执行体标记：同 if 分支——事件处理器体是
                            // showToast 的合法调用处。
                            let prev_in_execution_body = semantics.in_execution_body;
                            semantics.in_execution_body = true;
                            let body_ir = lower_block(body, variant_map, None, semantics);
                            semantics.in_execution_body = prev_in_execution_body;
                            EventHandlerIR::InlineFn {
                                body: body_ir,
                                params: params.iter().map(|p| p.name.clone()).collect(),
                                event_data: None,
                                used_fields: None,
                                needs_prevent_default: needs_pd,
                                needs_stop_propagation: needs_sp,
                            }
                        }
                    }
                    EventHandler::FnRef(fn_name) => {
                        // Look up the referenced function's body to find which
                        // event fields are used (e.g. `e.value`, `e.key`) and
                        // whether it calls e.stopPropagation()/preventDefault()
                        // (same scan as InlineFn — without it, `@onPress={fn}`
                        // where fn calls e.stopPropagation() emits neither the
                        // listener-level flag nor a working call, so the event
                        // keeps bubbling and the handler throws on `{}`).
                        let func_ir = fns.iter().find(|f| f.name == *fn_name).or_else(|| {
                            fns.iter().find(|f| {
                                f.name == *fn_name || f.name.ends_with(&format!("_{}", fn_name))
                            })
                        });

                        let used_fields = event_data.as_ref().and_then(|kind| {
                            let schema = event_data_schema(kind);
                            let valid_names: Vec<&str> =
                                schema.iter().map(|f| f.name.as_str()).collect();

                            func_ir.map(|func| {
                                let mut used = Vec::new();
                                // Extract event parameter name (default "e" if no params)
                                let fn_param_name =
                                    func.params.first().map(|p| p.name.as_str()).unwrap_or("e");
                                collect_ir_event_fields(&func.body, &mut used, fn_param_name);
                                used.into_iter()
                                    .filter(|f| valid_names.contains(&f.as_str()))
                                    .collect()
                            })
                        });

                        // Scan the referenced fn body for event method calls
                        // (stopPropagation / preventDefault) like InlineFn does.
                        if event_data.is_some() {
                            if let Some(func) = func_ir {
                                let fn_param_name =
                                    func.params.first().map(|p| p.name.as_str()).unwrap_or("e");
                                let (pd_from_scan, sp_from_scan) =
                                    scan_ir_event_methods(&func.body, fn_param_name);
                                needs_pd = needs_pd || pd_from_scan;
                                needs_sp = needs_sp || sp_from_scan;
                            }
                        }

                        EventHandlerIR::FnRef {
                            r#ref: fn_name.clone(),
                            params: vec![],
                            event_data: event_data.clone(),
                            used_fields,
                            needs_prevent_default: needs_pd,
                            needs_stop_propagation: needs_sp,
                        }
                    }
                    EventHandler::StoreAction { store, action } => {
                        // Store actions receive no event data
                        EventHandlerIR::StoreAction {
                            r#ref: format!("{}.{}", store, action),
                        }
                    }
                };
                events
                    .get_or_insert_with(Default::default)
                    .insert(name.clone(), ir_handler);
            }
            ElementProp::Class { value } => {
                // class: styleRef or class: "literal-class"
                let mut class_map_bindings: FxHashMap<String, ExprIR> = FxHashMap::default();
                let class_str = match value {
                    ClassValue::StyleRef(s) => validate_class_names(s, &el.span, semantics),
                    ClassValue::StringLiteral(s) => validate_class_names(s, &el.span, semantics),
                    ClassValue::DesignClass {
                        design_name: _,
                        class_name,
                    } => {
                        // Design class names are emitted directly (CSS is scoped
                        // via [data-design~="Name"] on the root element, not via
                        // component-level class name mangling).
                        class_name.clone()
                    }
                    ClassValue::ClassMap(entries) => {
                        // Include classes based on their literal value:
                        //   Bool(true)  → static truth  → include the class
                        //   Bool(false) → static falsy → skip the class
                        //   dynamic     → unknown at compile time → JS handles
                        let mut static_classes: Vec<String> = Vec::new();
                        for (key, val_expr) in entries {
                            // Bool(true)  → static truth  → include in HTML
                            // Bool(false) → static falsy → exclude entirely
                            // dynamic     → unknown at compile time → JS handles with classList.toggle
                            if let Expr::Literal(Literal::Bool(true), _) = val_expr {
                                static_classes.push(key.clone());
                            }
                            // Collect dynamic bindings (non-literal values) for reactive JS toggling
                            // Collect dynamic bindings (non-literal values) for reactive JS toggling
                            if !matches!(val_expr, Expr::Literal(Literal::Bool(_), _)) {
                                class_map_bindings.insert(
                                    key.clone(),
                                    lower_expr(val_expr, variant_map, None, semantics),
                                );
                            }
                        }
                        static_classes.join(" ")
                    }
                };
                props.get_or_insert_with(Default::default).insert(
                    "class".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::Value::String(class_str),
                    },
                );
                // Pass class_map_bindings through the element
                class_map_bindings_from_prop = Some(class_map_bindings);
            }
            ElementProp::Style { entries } => {
                // style: { key: value, ... } — passed as inline_style
                // Lower each entry to a JSON representation for codegen.
                // String-literal values land inside the emitted `style="…"`
                // attribute — the handler-breakout gate applies.
                let style_map: serde_json::Map<String, serde_json::Value> = entries
                    .iter()
                    .map(|(key, expr)| {
                        let ir_expr = match expr {
                            Expr::Literal(Literal::String(s), span) => ExprIR::LiteralExpr {
                                value: serde_json::Value::String(
                                    validate_attr_no_handler_breakout(s, span, semantics),
                                ),
                            },
                            _ => lower_expr(expr, variant_map, None, semantics),
                        };
                        let json_val = super::expr::lower_expr_to_json_from_ir(
                            &ir_expr,
                            semantics,
                            signal_like_names,
                        );
                        (key.clone(), json_val)
                    })
                    .collect();
                inline_style = Some(serde_json::Value::Object(style_map));
            }
            ElementProp::A11y(spec) => {
                // a11y: { label/role/hidden/live/alt } — semantic
                // accessibility block (design doc 域 10). Lowered into
                // ElementIR.a11y; validated by semantic/validate_a11y;
                // consumed by all three P0 backends. The string fields emit
                // into aria-* / role attributes — gate them too.
                a11y = Some(A11yIR {
                    label: spec
                        .label
                        .as_ref()
                        .map(|s| validate_attr_no_handler_breakout(s, &el.span, semantics)),
                    role: spec
                        .role
                        .as_ref()
                        .map(|s| validate_attr_no_handler_breakout(s, &el.span, semantics)),
                    hidden: spec
                        .hidden
                        .as_ref()
                        .map(|e| lower_expr(e, variant_map, None, semantics)),
                    live: spec
                        .live
                        .as_ref()
                        .map(|s| validate_attr_no_handler_breakout(s, &el.span, semantics)),
                    alt: spec
                        .alt
                        .as_ref()
                        .map(|e| lower_expr(e, variant_map, None, semantics)),
                });
            }
            ElementProp::Variant {
                design_name,
                variant_name,
                dimension_values,
            } => {
                // Encode variant binding as a synthetic prop that codegen will
                // pick up and expand via `expand_variant()`.
                let dim_map: FxHashMap<String, String> = dimension_values.iter().cloned().collect();
                let json_val = serde_json::json!({
                    "design_name": design_name,
                    "variant_name": variant_name,
                    "dimension_values": dim_map,
                });
                props.get_or_insert_with(Default::default).insert(
                    "_design_variant".into(),
                    ExprIR::LiteralExpr { value: json_val },
                );
            }
            ElementProp::Ref(r) => {
                ref_binding = Some(r.clone());
            }
            // @drag={move} @dragEnd={end} — draggable-intent directive
            // (ADR-073). Lowered into ElementIR.drag_binding; validated by
            // semantic/validate_drag (@ref requirement, handler existence,
            // arity); consumed by codegen backends (Phase 2 Tasks 8-10 wire
            // pointer capture per target). Handlers become FnRef IR with
            // event_data = DragIntent so the per-event field extraction
            // (used_fields) applies unchanged.
            ElementProp::Drag {
                on_drag,
                on_drag_end,
            } => {
                drag_binding = Some(DragBindingIR {
                    on_drag: drag_handler_ir(fns, on_drag),
                    on_drag_end: on_drag_end
                        .as_deref()
                        .map(|name| drag_handler_ir(fns, name)),
                });
            }
            // Semantic ref bindings: focus={sig}, scroll-x={sig}, scroll-y={sig}.
            // Extracted into ElementIR.semantic_bindings (consumed by codegen
            // backends in later phases); the semantic pass enforces that they
            // appear only on @ref-bound elements. The v1 value={sig} 双向形状
            // (only @ref'd elements keep it — v4 5.4 parser discriminator) is
            // removed (design D2) — validate_semantic_bindings hard-errors it.
            ElementProp::RefBinding { binding, signal } => {
                semantic_bindings
                    .get_or_insert_with(Default::default)
                    .insert(
                        binding.clone(),
                        lower_expr(signal, variant_map, None, semantics),
                    );
            }
            // Motion animation triggers: on:hover={fadeIn}, etc.
            ElementProp::MotionTrigger(mt) => {
                // Two forms: a named motion reference (@enter={fadeIn}) or an
                // inline tween body (@enter={{ opacity: 0, scale: 0.8 }} —
                // parsed as a MapLiteral). Inline bodies are lowered to a
                // KeyframeIR here and hoisted into the module motion table by
                // the lower_inline_motions pass.
                let (motion_ref, inline_motion) = match &mt.motion_expr {
                    Expr::VarRef(name, _) => (Some(name.clone()), None),
                    Expr::MapLiteral { entries, .. } => (
                        None,
                        Some(lower_inline_motion_map(entries, variant_map, semantics)),
                    ),
                    // The legacy `states:`/`on:` branches used to land here as
                    // the "__states_removed" sentinel literal and silently
                    // become an inert (None, None) trigger — every backend
                    // skipped it with zero diagnostics (domain HIGH-1). The
                    // parser now hard-errors on those (Task 4.1); any other
                    // non-motion expression is a malformed inline body and
                    // must be diagnosed, never silently dropped.
                    _ => {
                        semantics.diagnostics.push(Diagnostic::error(
                            DiagnosticKind::E0007,
                            "motion 表达式必须是命名 motion 引用或内联 tween 体 { ... }",
                            crate::parser::expr::span_of(&mt.motion_expr),
                        ));
                        (None, None)
                    }
                };
                motion_triggers.push(TriggerIntent {
                    kind: lower_trigger_kind(&mt.trigger, variant_map, semantics),
                    motion_ref,
                    inline_motion,
                    source_span: None,
                });
            } // The old sub-attribute ElementProps (@paused, in-view once/repeat/
            // threshold, scroll start/end/scrub, drag constraints/elastic/scrub/
            // momentum/threshold/velocity-max) no longer exist. @scroll/@drag-*
            // control dimensions are restored on the trigger keyword itself as
            // colon-form sub-params (Task 5.1, v2 BNF) and lowered via
            // lower_trigger_kind above.
            // Named slot projection: <Modal header={<div>Title</div>} />
            // The projected content is the fallback of the consumer-position
            // ViewTree::Slot (the parent will override the declaration's
            // fallback by slot name).
            ElementProp::SlotContent { name, content } => {
                let fallback = content
                    .iter()
                    .map(|n| lower_view_node(n, variant_map, semantics, fns, signal_like_names))
                    .collect();
                slot_projections.push(ViewTree::Slot {
                    name: Some(name.clone()),
                    fallback,
                });
            }
            // @form is a bare marker and @validate rule values lower with
            // the form intent — Task 2 (FormIntentIR/ValidateRuleIR)
            // owns both; nothing to lower here yet.
            ElementProp::Form | ElementProp::Validate { .. } => {}
            // 批次 1 反馈模态（域 10，spec 2026-08-08）：@modal 记录开启信号
            // 名（解析器保证 open_signal 恒为 VarRef——directives.rs 强制
            // `@modal={IDENT}`，非 VarRef 分支为不可达防御）；@skeleton 记录
            // 原始形状字符串（含非法值——语义层 V10 依原始串报
            // 「未知骨架形状 "x"」，故此处不做白名单）；@errorBoundary 置标记，
            // fallback 槽在 children 收尾扫描中提取。
            ElementProp::Modal { open_signal } => {
                match open_signal {
                    Expr::VarRef(name, _) => modal_open_signal = Some(name.clone()),
                    // 解析器强制 VarRef，此处不可达；保留分支以免误吞未来形状。
                    other => {
                        semantics.diagnostics.push(Diagnostic::error(
                            DiagnosticKind::E0006,
                            &format!("@modal 的开启信号必须是标识符引用：{:?}", other),
                            &el.span,
                        ));
                    }
                }
            }
            ElementProp::Skeleton { shape } => {
                has_skeleton = true;
                skeleton_shape = shape.clone();
            }
            ElementProp::ErrorBoundary => {
                has_error_boundary = true;
            }
        }
    }

    // Task 3.4: fill the legacy in-view behavior fields from the trigger
    // set — the v2 DSL's `@in-view={motion}` carries no sub-params, so an
    // InView trigger IS the config: once-semantics (fires on first
    // visibility) with the default threshold. Backends that read the legacy
    // fields (Flutter's has_inview/once flag, OmniView's disconnect
    // condition, React's threshold) now see a truthful IR instead of
    // always-None.
    let in_view_once: Option<ExprIR> = if motion_triggers
        .iter()
        .any(|t| matches!(t.kind, TriggerKind::InView))
    {
        Some(ExprIR::LiteralExpr {
            value: serde_json::Value::Bool(true),
        })
    } else {
        None
    };
    let in_view_repeat: Option<ExprIR> = None;
    let in_view_threshold: Option<ExprIR> = None;

    let mut children: Vec<ViewTree> = el
        .children
        .iter()
        .map(|c| lower_view_node(c, variant_map, semantics, fns, signal_like_names))
        .collect();
    // Slot projections are appended after regular children — codegen looks
    // them up by slot name, so relative order does not affect projection.
    children.extend(slot_projections);

    // 批次 1 反馈模态（域 10，spec 2026-08-09 §2.1）：children 收尾扫描。
    // @modal 的全部 `@slot` 声明（命名槽 + 匿名槽，按文档序）收集进
    // ModalIntentIR.slots；@errorBoundary 的 `@slot fallback` 收集进
    // ErrorBoundaryBindingIR.fallback（无 fallback 槽 → None，语义层 V12
    // 据此告警，不在 lowering 时报错）。槽指令仍保留在 children 中作为
    // ViewTree::Slot（既有渲染机制原样运行），此处仅为绑定 IR 建立同名
    // 投影供语义验证与 codegen 消费——两处内容一致，无重复渲染风险。
    let modal: Option<ModalIntentIR> = modal_open_signal.map(|open_signal| {
        let slots = el
            .children
            .iter()
            .filter_map(|c| match c {
                ViewNode::SlotDirective {
                    name,
                    default_content,
                } => Some(SlotDeclShape {
                    name,
                    default_content,
                }),
                _ => None,
            })
            .map(|shape| lower_slot_common(&shape, variant_map, semantics, fns, signal_like_names))
            .collect();
        ModalIntentIR { open_signal, slots }
    });
    let error_boundary: Option<ErrorBoundaryBindingIR> = if has_error_boundary {
        let fallback = el
            .children
            .iter()
            .filter_map(|c| match c {
                ViewNode::SlotDirective {
                    name,
                    default_content,
                } if name.as_deref() == Some("fallback") => Some(SlotDeclShape {
                    name,
                    default_content,
                }),
                _ => None,
            })
            .next()
            .map(|shape| lower_slot_common(&shape, variant_map, semantics, fns, signal_like_names));
        Some(ErrorBoundaryBindingIR { fallback })
    } else {
        None
    };

    ViewTree::Element(Box::new(ElementIR {
        tag: el.tag.clone(),
        props: props.unwrap_or_default(),
        bindings: bindings.unwrap_or_default(),
        events: events.unwrap_or_default(),
        style_classes: None,
        inline_style,
        children,
        construction_kind: ConstructionKind::Component,
        class_map_bindings: class_map_bindings_from_prop.unwrap_or_default(),
        source_span: Some(el.span),
        motion: {
            let has_any = !motion_triggers.is_empty()
                || in_view_once.is_some()
                || in_view_repeat.is_some()
                || in_view_threshold.is_some();
            if has_any {
                Some(Box::new(MotionBehavior {
                    triggers: motion_triggers,
                    in_view_once,
                    in_view_repeat,
                    in_view_threshold,
                    source_span: None,
                }))
            } else {
                None
            }
        },
        ref_binding,
        semantic_bindings: semantic_bindings.unwrap_or_default(),
        a11y,
        drag_binding,
        modal,
        skeleton: if has_skeleton {
            Some(SkeletonBindingIR {
                shape: skeleton_shape,
            })
        } else {
            None
        },
        error_boundary,
    }))
}

/// Lower an AST trigger kind to its IR form, carrying the v1 control
/// dimensions of `@scroll` / `@drag-*` (Task 5.1) as lowered Option exprs.
fn lower_trigger_kind(
    kind: &crate::parser::ast::TriggerKind,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> TriggerKind {
    use crate::parser::ast::TriggerKind as AstKind;
    match kind {
        AstKind::Enter => TriggerKind::Enter,
        AstKind::Hover => TriggerKind::Hover,
        AstKind::Press => TriggerKind::Press,
        AstKind::Focus => TriggerKind::Focus,
        AstKind::InView => TriggerKind::InView,
        AstKind::Scroll { start, end, scrub } => TriggerKind::Scroll {
            start: start
                .as_ref()
                .map(|e| lower_expr(e, variant_map, None, semantics)),
            end: end
                .as_ref()
                .map(|e| lower_expr(e, variant_map, None, semantics)),
            scrub: scrub
                .as_ref()
                .map(|e| lower_expr(e, variant_map, None, semantics)),
        },
        AstKind::DragHorizontal {
            constraints,
            elastic,
            threshold,
        } => TriggerKind::DragHorizontal {
            constraints: lower_constraints(constraints, variant_map, semantics),
            elastic: elastic
                .as_ref()
                .map(|e| lower_expr(e, variant_map, None, semantics)),
            threshold: threshold
                .as_ref()
                .map(|e| lower_expr(e, variant_map, None, semantics)),
        },
        AstKind::DragVertical {
            constraints,
            elastic,
            threshold,
        } => TriggerKind::DragVertical {
            constraints: lower_constraints(constraints, variant_map, semantics),
            elastic: elastic
                .as_ref()
                .map(|e| lower_expr(e, variant_map, None, semantics)),
            threshold: threshold
                .as_ref()
                .map(|e| lower_expr(e, variant_map, None, semantics)),
        },
    }
}

/// Lower the `constraints: [min, max]` pair.
fn lower_constraints(
    constraints: &Option<(Expr, Expr)>,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> Option<(ExprIR, ExprIR)> {
    constraints.as_ref().map(|(min, max)| {
        (
            lower_expr(min, variant_map, None, semantics),
            lower_expr(max, variant_map, None, semantics),
        )
    })
}

/// Lower an inline motion tween body (`@enter={{ opacity: 0, scale: 0.8 }}`)
/// into a `KeyframeIR`.
///
/// The map is a single "from" stop at position 0.0: the element animates from
/// these values to its natural state when the trigger fires. Inline tweens
/// carry no `over`/`easing` clauses, so the duration defaults to 400ms — the
/// same default used by the inline-motion codegen fixtures. The
/// `lower_inline_motions` pass later hoists the body into the module motion
/// table under a synthesized name.
fn lower_inline_motion_map(
    entries: &[(Expr, Expr)],
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> KeyframeIR {
    let mut properties = FxHashMap::default();
    for (key, value) in entries {
        if let Expr::Literal(Literal::String(name), _) = key {
            properties.insert(
                name.clone(),
                lower_expr(value, variant_map, None, semantics),
            );
        }
    }
    KeyframeIR {
        stops: vec![KeyframeStopIR {
            position: 0.0,
            properties,
            ease: None,
            source_span: None,
        }],
        over: ExprIR::LiteralExpr {
            value: serde_json::json!(400),
        },
        ease: None,
        playback: PlaybackIR::default(),
        source_span: None,
    }
}

// ── If-directive lowering ─────────────────────────────────

/// Lowers an `IfDirective` AST node into `ViewTree::Conditional`.
/// Creates `ConditionalBranch` entries for the primary condition,
/// any `else if` chains, and the optional `else` fallback.
fn lower_if_directive(
    if_dir: &IfDirective,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    fns: &[FunctionIR],
    signal_like_names: &FxHashSet<String>,
) -> ViewTree {
    let branches: Vec<ConditionalBranch> = std::iter::once(ConditionalBranch {
        condition: lower_expr(&if_dir.condition, variant_map, None, semantics),
        body: Some(make_fragment_element(
            if_dir
                .body
                .iter()
                .map(|n| lower_view_node(n, variant_map, semantics, fns, signal_like_names))
                .collect(),
            ConstructionKind::Conditional,
        )),
    })
    .chain(if_dir.else_ifs.iter().map(|(cond, body)| {
        ConditionalBranch {
            condition: lower_expr(cond, variant_map, None, semantics),
            body: Some(make_fragment_element(
                body.iter()
                    .map(|n| lower_view_node(n, variant_map, semantics, fns, signal_like_names))
                    .collect(),
                ConstructionKind::Conditional,
            )),
        }
    }))
    .collect();

    let fallback = if_dir.else_body.as_ref().map(|body| {
        make_fragment_element(
            body.iter()
                .map(|n| lower_view_node(n, variant_map, semantics, fns, signal_like_names))
                .collect(),
            ConstructionKind::Conditional,
        )
    });

    ViewTree::Conditional(Box::new(ConditionalIntent {
        branches,
        fallback,
        transition_mode: if_dir.transition_mode.clone(),
    }))
}

// ── For-directive lowering ────────────────────────────────

/// Lowers a `ForDirective` AST node into `ViewTree::ListRendering`.
/// Wraps body nodes in a fragment with `ConstructionKind::List`.
fn lower_for_directive(
    for_dir: &ForDirective,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
    fns: &[FunctionIR],
    signal_like_names: &FxHashSet<String>,
) -> ViewTree {
    let lower_body = |nodes: &[ViewNode], s: &mut SemanticResult| -> Option<ViewTree> {
        if nodes.is_empty() {
            None
        } else {
            Some(make_fragment_element(
                nodes
                    .iter()
                    .map(|n| lower_view_node(n, variant_map, s, fns, signal_like_names))
                    .collect(),
                ConstructionKind::List,
            ))
        }
    };

    ViewTree::ListRendering(Box::new(ListRenderingIntent {
        iterable: lower_expr(&for_dir.iterable, variant_map, None, semantics),
        item_binding: BindingIR {
            name: for_dir.item_name.clone(),
            r#type: None,
        },
        body: lower_body(&for_dir.body, semantics),
        key_expr: for_dir
            .key_expr
            .as_ref()
            .map(|e| lower_expr(e, variant_map, None, semantics)),
        empty_body: for_dir
            .empty_body
            .as_ref()
            .and_then(|nodes| lower_body(nodes.as_slice(), semantics)),
        scroll_hint: for_dir.scroll_hint,
    }))
}

// ═══════════════════════════════════════════════════════
// Event field helpers (moved from component.rs — inline
// fn_ref backfill replaces the old two-phase mutation pass)
// ═══════════════════════════════════════════════════════

/// Walk a BlockIR statement list for FieldAccessExpr where object is IdentifierExpr(param_name).
/// Uses the shared `walk_stmt` to traverse all expressions within the block.
fn collect_ir_event_fields(block: &BlockIR, fields: &mut Vec<String>, param_name: &str) {
    for stmt in &block.statements {
        walk::walk_stmt(stmt, &mut |expr| {
            if let ExprIR::FieldAccessExpr { object, field } = expr {
                if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    if name == param_name && !fields.contains(field) {
                        fields.push(field.clone());
                    }
                }
            }
        });
    }
}

/// Build the FnRef handler IR for an `@drag` / `@dragEnd` handler name
/// (ADR-073): `event_data = DragIntent` with `used_fields` scanned from the
/// fn body (same lookup/scan as the `ElementProp::Event` FnRef path), so
/// codegen's per-event field extraction works unchanged for drag handlers.
fn drag_handler_ir(fns: &[FunctionIR], fn_name: &str) -> EventHandlerIR {
    let func_ir = fns.iter().find(|f| f.name == *fn_name).or_else(|| {
        fns.iter()
            .find(|f| f.name == *fn_name || f.name.ends_with(&format!("_{}", fn_name)))
    });
    let schema = event_data_schema(&EventDataKind::DragIntent);
    let valid_names: Vec<&str> = schema.iter().map(|f| f.name.as_str()).collect();
    let used_fields = func_ir.map(|func| {
        let mut used = Vec::new();
        let fn_param_name = func.params.first().map(|p| p.name.as_str()).unwrap_or("e");
        collect_ir_event_fields(&func.body, &mut used, fn_param_name);
        used.into_iter()
            .filter(|f| valid_names.contains(&f.as_str()))
            .collect()
    });
    let (needs_pd, needs_sp) = match func_ir {
        Some(func) => {
            let fn_param_name = func.params.first().map(|p| p.name.as_str()).unwrap_or("e");
            scan_ir_event_methods(&func.body, fn_param_name)
        }
        None => (false, false),
    };
    EventHandlerIR::FnRef {
        r#ref: fn_name.to_string(),
        params: vec![],
        event_data: Some(EventDataKind::DragIntent),
        used_fields,
        needs_prevent_default: needs_pd,
        needs_stop_propagation: needs_sp,
    }
}

/// IR-level scan for `e.stopPropagation()` / `e.preventDefault()` calls
/// inside a lowered function body (the AST-level `scan_event_methods` only
/// sees inline handler bodies). FnRef handlers get their needs_* flags from
/// this scan — without it the listener never emits the flag and the handler
/// receives `{}` with no such methods (TypeError on click-through).
fn scan_ir_event_methods(block: &BlockIR, param_name: &str) -> (bool, bool) {
    let mut needs_pd = false;
    let mut needs_sp = false;
    for stmt in &block.statements {
        walk::walk_stmt(stmt, &mut |expr| {
            if let ExprIR::MethodCallExpr { object, method, .. } = expr {
                if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    if name == param_name {
                        if method == "preventDefault" {
                            needs_pd = true;
                        } else if method == "stopPropagation" {
                            needs_sp = true;
                        }
                    }
                }
            }
        });
    }
    (needs_pd, needs_sp)
}

// ── Helper factories ─────────────────────────────────────

/// Creates a "fragment" `ElementIR` that wraps child view nodes.
/// Used by conditional/list directives that need a synthetic wrapper element.
fn make_fragment_element(children: Vec<ViewTree>, construction_kind: ConstructionKind) -> ViewTree {
    ViewTree::Element(Box::new(ElementIR {
        tag: "fragment".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children,
        construction_kind,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Span;
    use crate::semantic::symbols::SymbolTable;

    #[test]
    fn test_lower_element_with_ref_binding() {
        // Verify that ElementProp::Ref("box") sets ref_binding on ElementIR
        // instead of emitting a "data-ref" prop.
        let el = Element {
            tag: "div".into(),
            props: vec![ElementProp::Ref("box".to_string())],
            children: vec![],
            self_closing: true,
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                col: 1,
            },
        };
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
            in_execution_body: false,
        };
        let result = lower_element(
            &el,
            &FxHashMap::default(),
            &mut semantics,
            &[],
            &FxHashSet::default(),
        );
        match result {
            ViewTree::Element(el_ir) => {
                assert_eq!(
                    el_ir.ref_binding,
                    Some("box".to_string()),
                    "ref_binding should be set from ElementProp::Ref"
                );
                assert!(
                    !el_ir.props.contains_key("data-ref"),
                    "no data-ref prop should be emitted"
                );
            }
            _ => panic!("Expected ViewTree::Element"),
        }
    }

    #[test]
    fn lower_modal_intent_captures_open_signal_and_slots() {
        // @modal={isOpen} lowers into ElementIR.modal (批次 1 域 10): the
        // open-signal reference is captured verbatim and named slot
        // children are collected as modal slots for the dialog shell.
        let span = Span {
            start: 0,
            end: 0,
            line: 1,
            col: 1,
        };
        let el = Element {
            tag: "dialog".into(),
            props: vec![ElementProp::Modal {
                open_signal: Expr::VarRef("isOpen".into(), span),
            }],
            children: vec![
                ViewNode::SlotDirective {
                    name: Some("header".into()),
                    default_content: None,
                },
                ViewNode::SlotDirective {
                    name: Some("body".into()),
                    default_content: None,
                },
            ],
            self_closing: false,
            span,
        };
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
            in_execution_body: false,
        };
        let result = lower_element(
            &el,
            &FxHashMap::default(),
            &mut semantics,
            &[],
            &FxHashSet::default(),
        );
        match result {
            ViewTree::Element(el_ir) => {
                let modal = el_ir
                    .modal
                    .as_ref()
                    .expect("modal binding should be set from ElementProp::Modal");
                assert_eq!(
                    modal.open_signal, "isOpen",
                    "open_signal should capture the @modal signal reference"
                );
                let slot_names: Vec<Option<&str>> =
                    modal.slots.iter().map(|s| s.name.as_deref()).collect();
                assert!(
                    slot_names.contains(&Some("header")),
                    "modal slots must include header, got {:?}",
                    slot_names
                );
                assert!(
                    slot_names.contains(&Some("body")),
                    "modal slots must include body, got {:?}",
                    slot_names
                );
            }
            _ => panic!("Expected ViewTree::Element"),
        }
    }

    #[test]
    fn lower_skeleton_and_error_boundary_bindings() {
        // @skeleton="card" lowers into ElementIR.skeleton with the raw
        // shape string; @errorBoundary collects the named "fallback" slot
        // as its fallback content (批次 1 域 10).
        let span = Span {
            start: 0,
            end: 0,
            line: 1,
            col: 1,
        };
        let skeleton_el = Element {
            tag: "div".into(),
            props: vec![ElementProp::Skeleton {
                shape: Some("card".into()),
            }],
            children: vec![],
            self_closing: true,
            span,
        };
        let boundary_el = Element {
            tag: "div".into(),
            props: vec![ElementProp::ErrorBoundary],
            children: vec![ViewNode::SlotDirective {
                name: Some("fallback".into()),
                default_content: None,
            }],
            self_closing: false,
            span,
        };
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
            in_execution_body: false,
        };
        let result = lower_element(
            &skeleton_el,
            &FxHashMap::default(),
            &mut semantics,
            &[],
            &FxHashSet::default(),
        );
        match result {
            ViewTree::Element(el_ir) => {
                let skeleton = el_ir
                    .skeleton
                    .as_ref()
                    .expect("skeleton binding should be set from ElementProp::Skeleton");
                assert_eq!(
                    skeleton.shape.as_deref(),
                    Some("card"),
                    "skeleton shape should preserve the raw @skeleton value"
                );
            }
            _ => panic!("Expected ViewTree::Element"),
        }
        let result = lower_element(
            &boundary_el,
            &FxHashMap::default(),
            &mut semantics,
            &[],
            &FxHashSet::default(),
        );
        match result {
            ViewTree::Element(el_ir) => {
                let boundary = el_ir
                    .error_boundary
                    .as_ref()
                    .expect("error_boundary should be set from ElementProp::ErrorBoundary");
                let fallback = boundary
                    .fallback
                    .as_ref()
                    .expect("fallback slot should be captured for @errorBoundary");
                assert_eq!(
                    fallback.name.as_deref(),
                    Some("fallback"),
                    "fallback slot name must be preserved"
                );
                // 无 @skeleton 属性的元素必须保持 skeleton: None——与下方
                // 裸 @skeleton（shape: None）严格区分。
                assert!(
                    el_ir.skeleton.is_none(),
                    "element without @skeleton must not carry a skeleton binding"
                );
            }
            _ => panic!("Expected ViewTree::Element"),
        }
        // 裸 @skeleton（无 shape 参数）→ Some(SkeletonBindingIR { shape: None })：
        // shape: None 表示默认 text 形状，是合法有语义的 IR 值，不得被静默丢弃。
        let bare_skeleton_el = Element {
            tag: "div".into(),
            props: vec![ElementProp::Skeleton { shape: None }],
            children: vec![],
            self_closing: true,
            span,
        };
        let result = lower_element(
            &bare_skeleton_el,
            &FxHashMap::default(),
            &mut semantics,
            &[],
            &FxHashSet::default(),
        );
        match result {
            ViewTree::Element(el_ir) => {
                let skeleton = el_ir
                    .skeleton
                    .as_ref()
                    .expect("bare @skeleton must still set a skeleton binding");
                assert!(
                    skeleton.shape.is_none(),
                    "bare @skeleton must lower to shape: None (default text)"
                );
            }
            _ => panic!("Expected ViewTree::Element"),
        }
    }

    #[test]
    fn test_lower_element_ref_binding_lowers_to_semantic_bindings() {
        // RefBinding props lower into ElementIR.semantic_bindings — not into
        // the props map. Codegen backends consume semantic_bindings directly
        // (focus={sig}, scroll-x={sig}, scroll-y={sig}, value={sig}).
        let span = Span {
            start: 0,
            end: 0,
            line: 1,
            col: 1,
        };
        let el = Element {
            tag: "div".into(),
            props: vec![ElementProp::RefBinding {
                binding: "scroll-y".into(),
                signal: Expr::VarRef("pos".into(), span),
            }],
            children: vec![],
            self_closing: true,
            span,
        };
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
            in_execution_body: false,
        };
        let result = lower_element(
            &el,
            &FxHashMap::default(),
            &mut semantics,
            &[],
            &FxHashSet::default(),
        );
        match result {
            ViewTree::Element(el_ir) => {
                assert!(
                    el_ir.semantic_bindings.contains_key("scroll-y"),
                    "scroll-y should be lowered into semantic_bindings"
                );
                assert!(
                    matches!(
                        el_ir.semantic_bindings.get("scroll-y"),
                        Some(ExprIR::IdentifierExpr { name, .. }) if name == "pos"
                    ),
                    "signal expr should lower to IdentifierExpr(pos)"
                );
                assert!(
                    !el_ir.props.contains_key("scroll-y"),
                    "scroll-y should NOT be lowered into props"
                );
            }
            _ => panic!("Expected ViewTree::Element"),
        }
    }

    #[test]
    fn test_no_auto_bind_for_bare_input() {
        // design D2 (值绑定收敛): the implicit auto-bind is removed — an
        // `<input>` without any value prop/binding gets NO binding at all.
        // `value` is always exactly what the author writes: explicit
        // bind:value (two-way), explicit value prop (one-way), or nothing.
        let span = Span {
            start: 0,
            end: 0,
            line: 1,
            col: 1,
        };
        let el = Element {
            tag: "input".into(),
            props: vec![],
            children: vec![],
            self_closing: true,
            span,
        };
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
            in_execution_body: false,
        };
        let result = lower_element(
            &el,
            &FxHashMap::default(),
            &mut semantics,
            &[],
            &FxHashSet::default(),
        );
        match result {
            ViewTree::Element(el_ir) => {
                assert!(
                    !el_ir.bindings.contains_key("value"),
                    "auto-bind removed — a bare <input> must carry no implicit value binding"
                );
            }
            _ => panic!("Expected ViewTree::Element"),
        }
    }

    // ═══════════════════════════════════════════════════════
    // FnRef stopPropagation/preventDefault scan regression
    // (7330e783 — the IR-level scan for fn_ref bodies; this path
    // previously had ZERO regression tests)
    // ═══════════════════════════════════════════════════════

    /// Lower a component source and return the first (only) component.
    fn lower_first_component(source: &str) -> ComponentIR {
        let tokens = crate::lexer::tokenize(source);
        let (program, errors) = crate::parser::parse(tokens, source);
        assert!(
            errors.is_empty(),
            "unexpected parse errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
            in_execution_body: false,
        };
        let ir = crate::ir::lower(&program, &mut semantics, None);
        let module = ir.modules.get("input.omni").expect("should have module");
        let (_, comp) = module
            .components
            .iter()
            .next()
            .expect("should have a component");
        comp.clone()
    }

    #[test]
    fn fnref_stop_propagation_scan_regression() {
        // A FnRef handler that calls e.stopPropagation() must set the
        // needs_stop_propagation flag on the lowered handler — without it
        // the listener never emits the flag and the handler receives `{}`
        // with no such method (TypeError on click-through). Regression for
        // the 7330e783 fix (the scan used to only see inline bodies).
        let source = r#"
import omniview;
component FnRefSpDemo {
    fn stopIt(e) {
        e.stopPropagation();
    }
    view {
        <div @onPress={stopIt}>x</div>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                let handler = el.events.get("onPress").expect("should have onPress event");
                match handler {
                    EventHandlerIR::FnRef {
                        needs_stop_propagation,
                        needs_prevent_default,
                        ..
                    } => {
                        assert!(
                            needs_stop_propagation,
                            "FnRef handler calling e.stopPropagation() must set needs_stop_propagation"
                        );
                        assert!(
                            !needs_prevent_default,
                            "no preventDefault call in body — flag must stay unset"
                        );
                    }
                    other => panic!("expected FnRef handler, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn fnref_without_event_methods_flags_unset() {
        // Handlers with neither stopPropagation nor preventDefault must
        // leave both flags unset.
        let source = r#"
import omniview;
component FnRefPlainDemo {
    fn plainHandler(e) {
    }
    view {
        <div @onKeyDown={plainHandler}>x</div>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                let handler = el
                    .events
                    .get("onKeyDown")
                    .expect("should have onKeyDown event");
                match handler {
                    EventHandlerIR::FnRef {
                        needs_stop_propagation,
                        needs_prevent_default,
                        ..
                    } => {
                        assert!(
                            !needs_stop_propagation,
                            "flag must stay unset without e.stopPropagation()"
                        );
                        assert!(
                            !needs_prevent_default,
                            "flag must stay unset without e.preventDefault()"
                        );
                    }
                    other => panic!("expected FnRef handler, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn scan_ir_event_methods_detects_stop_propagation_directly() {
        let block = BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::MethodCallExpr {
                    object: Box::new(ExprIR::IdentifierExpr {
                        name: "e".into(),
                        type_ann: None,
                    }),
                    method: "stopPropagation".into(),
                    args: vec![],
                    is_optional: false,
                },
            }],
        };
        let (needs_pd, needs_sp) = scan_ir_event_methods(&block, "e");
        assert!(needs_sp, "stopPropagation call must set the flag");
        assert!(!needs_pd, "preventDefault flag must stay unset");
    }

    #[test]
    fn scan_ir_event_methods_ignores_other_params_and_methods() {
        let block = BlockIR {
            statements: vec![
                StatementIR::ExprStmtIR {
                    expr: ExprIR::MethodCallExpr {
                        object: Box::new(ExprIR::IdentifierExpr {
                            name: "e".into(),
                            type_ann: None,
                        }),
                        method: "preventDefault".into(),
                        args: vec![],
                        is_optional: false,
                    },
                },
                StatementIR::ExprStmtIR {
                    expr: ExprIR::MethodCallExpr {
                        object: Box::new(ExprIR::IdentifierExpr {
                            name: "other".into(),
                            type_ann: None,
                        }),
                        method: "stopPropagation".into(),
                        args: vec![],
                        is_optional: false,
                    },
                },
            ],
        };
        let (needs_pd, needs_sp) = scan_ir_event_methods(&block, "e");
        assert!(needs_pd, "preventDefault on e must set the flag");
        assert!(
            !needs_sp,
            "stopPropagation on a different object must NOT set the flag"
        );
    }

    /// Regression: a view block with multiple top-level nodes must lower ALL
    /// of them (previously only `children.first()` was lowered, silently
    /// dropping every node after the first element).
    #[test]
    fn test_multi_root_view_block_preserves_all_children() {
        let source = r#"
component MultiRoot {
    view {
        <div class: a>A</div>
        <div class: b @hover={pulse}>B</div>
    }
}
"#;
        let tokens = crate::lexer::tokenize(source);
        let (program, errors) = crate::parser::parse(tokens, source);
        assert!(
            errors.is_empty(),
            "unexpected parse errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
            in_execution_body: false,
        };
        let ir = crate::ir::lower(&program, &mut semantics, None);
        let module = ir.modules.get("input.omni").expect("module");
        let comp = module.components.get("MultiRoot").expect("component");
        let view = comp.view.as_ref().expect("view");
        match view {
            ViewTree::Fragment(children) => {
                assert_eq!(children.len(), 2, "both top-level nodes must survive");
                let texts: Vec<&str> = children
                    .iter()
                    .filter_map(|c| match c {
                        ViewTree::Element(el) => el.children.iter().find_map(|g| match g {
                            ViewTree::Text(t) => Some(t.as_str()),
                            _ => None,
                        }),
                        _ => None,
                    })
                    .collect();
                assert_eq!(texts, vec!["A", "B"], "both elements must be present");
            }
            _ => panic!("Expected ViewTree::Fragment for a multi-root view"),
        }
    }

    /// Build an element with a single `class:` prop and lower it, returning
    /// the lowered ViewTree plus the diagnostics produced.
    fn lower_class_prop(value: ClassValue) -> (ViewTree, Vec<crate::error::Diagnostic>) {
        let el = Element {
            tag: "div".into(),
            props: vec![ElementProp::Class { value }],
            children: vec![],
            self_closing: true,
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                col: 1,
            },
        };
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
            in_execution_body: false,
        };
        let tree = lower_element(
            &el,
            &FxHashMap::default(),
            &mut semantics,
            &[],
            &FxHashSet::default(),
        );
        (tree, semantics.diagnostics)
    }

    /// Lowered class prop value from a ViewTree::Element, if present.
    fn lowered_class(tree: &ViewTree) -> Option<String> {
        match tree {
            ViewTree::Element(el_ir) => el_ir.props.get("class").and_then(|v| match v {
                ExprIR::LiteralExpr { value } => value.as_str().map(String::from),
                _ => None,
            }),
            _ => None,
        }
    }

    #[test]
    fn class_name_injection_rejected_at_lowering() {
        // Review-verified exploit: `class: "x',alert(document.cookie),'"`
        // used to flow through verbatim into `styles['x',alert(...),'']`.
        // Layer 1 of the double-insurance: the lowering rejects any class
        // name outside `[a-zA-Z0-9_-]` with a hard error and drops the
        // invalid token from the IR.
        let (tree, diagnostics) = lower_class_prop(ClassValue::StringLiteral(
            "x',alert(document.cookie),'".into(),
        ));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("类名仅允许字母数字下划线连字符")),
            "expected charset-validation diagnostic, got: {:?}",
            diagnostics
        );
        assert!(
            diagnostics
                .iter()
                .all(|d| d.severity == crate::error::Severity::Error),
            "class-name validation must be a hard error"
        );
        assert_eq!(
            lowered_class(&tree),
            Some(String::new()),
            "invalid class token must be dropped from the lowered IR"
        );
    }

    #[test]
    fn class_name_injection_rejected_for_style_refs_and_mixed_classes() {
        // StyleRef class names are parser idents, but they are validated with
        // the same charset policy as quoted literals; mixed valid+invalid
        // keeps the valid names and reports the invalid one.
        let (tree, diagnostics) =
            lower_class_prop(ClassValue::StyleRef("btn x',alert(1),' ok-btn".into()));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("类名仅允许字母数字下划线连字符")),
            "expected charset-validation diagnostic, got: {:?}",
            diagnostics
        );
        assert_eq!(
            lowered_class(&tree),
            Some("btn ok-btn".into()),
            "valid tokens must pass through, invalid ones are dropped"
        );
    }

    #[test]
    fn valid_class_names_pass_through_unchanged() {
        // Hyphenated, underscore, multi-class and digit-suffixed names are
        // all legal and must reach the IR untouched with no diagnostics.
        for (value, expected) in [
            (
                ClassValue::StringLiteral("btn btn-primary card--active".into()),
                "btn btn-primary card--active",
            ),
            (
                ClassValue::StyleRef("theme-bar demo1-r-0 _hidden".into()),
                "theme-bar demo1-r-0 _hidden",
            ),
            (ClassValue::StringLiteral("".into()), ""),
        ] {
            let (tree, diagnostics) = lower_class_prop(value);
            assert!(
                diagnostics.is_empty(),
                "valid class names must not diagnose: {:?}",
                diagnostics
            );
            assert_eq!(lowered_class(&tree), Some(expected.into()));
        }
    }

    #[test]
    fn design_class_and_class_map_keys_are_not_revalidated() {
        // DesignClass names and ClassMap keys come from expect_ident and can
        // never carry quote/backtick payloads — they must pass through with
        // no new diagnostics (class_map static truthy keys still included).
        let span = Span {
            start: 0,
            end: 0,
            line: 1,
            col: 1,
        };
        let el = Element {
            tag: "div".into(),
            props: vec![
                ElementProp::Class {
                    value: ClassValue::DesignClass {
                        design_name: "AppDesign".into(),
                        class_name: "Card".into(),
                    },
                },
                ElementProp::Class {
                    value: ClassValue::ClassMap(vec![
                        ("active".into(), Expr::Literal(Literal::Bool(true), span)),
                        ("hidden".into(), Expr::Literal(Literal::Bool(false), span)),
                    ]),
                },
            ],
            children: vec![],
            self_closing: true,
            span,
        };
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
            in_execution_body: false,
        };
        let _tree = lower_element(
            &el,
            &FxHashMap::default(),
            &mut semantics,
            &[],
            &FxHashSet::default(),
        );
        assert!(
            semantics.diagnostics.is_empty(),
            "design classes and class-map keys must not diagnose: {:?}",
            semantics.diagnostics
        );
    }

    #[test]
    fn test_lower_element_drag_directive() {
        // @drag/@dragEnd lower into ElementIR.drag_binding as FnRef handlers
        // with event_data = DragIntent (ADR-073) — codegen backends read
        // drag_binding and wire capture per target (Tasks 8-10).
        let span = Span {
            start: 0,
            end: 0,
            line: 1,
            col: 1,
        };
        let el = Element {
            tag: "div".into(),
            props: vec![ElementProp::Drag {
                on_drag: "move".into(),
                on_drag_end: Some("end".into()),
            }],
            children: vec![],
            self_closing: true,
            span,
        };
        let mut semantics = SemanticResult {
            symbols: SymbolTable::default(),
            dependencies: FxHashMap::default(),
            diagnostics: vec![],
            event_param_name: None,
            in_execution_body: false,
        };
        let fns = vec![FunctionIR {
            name: "move".into(),
            visibility: Visibility::Private,
            params: vec![ParamIR {
                name: "e".into(),
                r#type: None,
            }],
            return_type: None,
            body: BlockIR { statements: vec![] },
            is_async: false,
            source_span: None,
        }];
        let result = lower_element(
            &el,
            &FxHashMap::default(),
            &mut semantics,
            &fns,
            &FxHashSet::default(),
        );
        let ViewTree::Element(el_ir) = result else {
            panic!("Expected ViewTree::Element");
        };
        let drag = el_ir
            .drag_binding
            .as_ref()
            .expect("drag_binding should be set from ElementProp::Drag");
        let EventHandlerIR::FnRef {
            r#ref,
            event_data,
            used_fields,
            ..
        } = &drag.on_drag
        else {
            panic!("on_drag must lower to FnRef, got: {:?}", drag.on_drag);
        };
        assert_eq!(r#ref, "move");
        assert_eq!(event_data, &Some(EventDataKind::DragIntent));
        assert_eq!(used_fields, &Some(vec![]), "empty fn body uses no fields");
        let EventHandlerIR::FnRef { r#ref: end_ref, .. } = drag
            .on_drag_end
            .as_ref()
            .expect("on_drag_end should be set")
        else {
            panic!("on_drag_end must lower to FnRef");
        };
        assert_eq!(end_ref, "end");
    }
}
