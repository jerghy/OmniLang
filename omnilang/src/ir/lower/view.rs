//! Lowering for view blocks and view nodes.
//!
//! Includes: `lower_view_block`, `lower_view_node`,
//! and `EventFieldCollector` for inline event handlers.

use crate::ir::types::*;
use crate::ir::types_motion::*;
use crate::ir::TriggerKind;
use crate::parser::ast::*;
use std::cell::Cell;
use std::collections::HashMap;

use super::expr::lower_expr;
use super::stmt::lower_block;

// ═══════════════════════════════════════════════════════
// View lowering
// ═══════════════════════════════════════════════════════

pub(super) fn lower_view_block(
    view: &ViewBlock,
    string_signals: &[String],
    variant_map: &HashMap<String, String>,
) -> ViewTree {
    if let Some(first) = view.children.first() {
        let sig_idx = Cell::new(0usize);
        lower_view_node(first, string_signals, Some(&sig_idx), variant_map)
    } else {
        ViewTree::Text("".into())
    }
}

/// Collects `e.field` references and `e.preventDefault()`/`e.stopPropagation()`
/// calls from an inline event handler AST body.  Used during IR lowering
/// to populate `EventHandlerIR.used_fields`.
struct EventFieldCollector {
    fields: Vec<String>,
    needs_prevent_default: bool,
    needs_stop_propagation: bool,
}

impl EventFieldCollector {
    fn collect(block: &crate::parser::ast::Block) -> Self {
        let mut c = Self {
            fields: vec![],
            needs_prevent_default: false,
            needs_stop_propagation: false,
        };
        for stmt in &block.stmts {
            c.visit_stmt(stmt);
        }
        c
    }

    fn visit_stmt(&mut self, stmt: &crate::parser::ast::Stmt) {
        match stmt {
            crate::parser::ast::Stmt::ExprStmt { expr, .. } => {
                self.visit_expr(expr);
            }
            crate::parser::ast::Stmt::VarDecl {
                init: Some(init), ..
            } => {
                self.visit_expr(init);
            }
            crate::parser::ast::Stmt::Assign { value, .. } => {
                self.visit_expr(value);
            }
            crate::parser::ast::Stmt::If {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                self.visit_expr(condition);
                self.visit_block(then_body);
                for (_, elsif_block) in else_ifs {
                    self.visit_block(elsif_block);
                }
                if let Some(else_b) = else_body {
                    self.visit_block(else_b);
                }
            }
            crate::parser::ast::Stmt::While {
                condition, body, ..
            } => {
                self.visit_expr(condition);
                self.visit_block(body);
            }
            crate::parser::ast::Stmt::ForIn { iterable, body, .. } => {
                self.visit_expr(iterable);
                self.visit_block(body);
            }
            crate::parser::ast::Stmt::Return {
                value: Some(value), ..
            } => {
                self.visit_expr(value);
            }
            crate::parser::ast::Stmt::Print { arg, .. } => {
                self.visit_expr(arg);
            }
            crate::parser::ast::Stmt::EventEmit { args, .. } => {
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            crate::parser::ast::Stmt::Throw { expr, .. } => {
                self.visit_expr(expr);
            }
            crate::parser::ast::Stmt::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                self.visit_block(try_body);
                if let Some(catch_b) = catch_body {
                    self.visit_block(catch_b);
                }
                if let Some(finally_b) = finally_body {
                    self.visit_block(finally_b);
                }
            }
            _ => {}
        }
    }

    fn visit_block(&mut self, block: &crate::parser::ast::Block) {
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
    }

    fn visit_expr(&mut self, expr: &crate::parser::ast::Expr) {
        use crate::parser::ast::Expr;
        match expr {
            // e.field
            Expr::FieldAccess { object, field, .. } => {
                if let Expr::VarRef(name, _) = object.as_ref() {
                    if name == "e" && !self.fields.contains(field) {
                        self.fields.push(field.clone());
                    }
                } else {
                    self.visit_expr(object);
                }
            }
            // e.method(args)
            Expr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                if let Expr::VarRef(name, _) = object.as_ref() {
                    if name == "e" {
                        match method.as_str() {
                            "preventDefault" => self.needs_prevent_default = true,
                            "stopPropagation" => self.needs_stop_propagation = true,
                            _ => {}
                        }
                    }
                } else {
                    self.visit_expr(object);
                }
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expr::UnaryOp { operand, .. } => self.visit_expr(operand),
            Expr::FuncCall { func, args, .. } => {
                self.visit_expr(func);
                for arg in args {
                    self.visit_expr(arg);
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
                self.visit_expr(condition);
                self.visit_expr(then_expr);
                self.visit_expr(else_expr);
            }
            Expr::ArrayLiteral { elements, .. } => {
                for e in elements {
                    self.visit_expr(e);
                }
            }
            Expr::MapLiteral { entries, .. } => {
                for (k, v) in entries {
                    self.visit_expr(k);
                    self.visit_expr(v);
                }
            }
            _ => {}
        }
    }
}

pub(super) fn lower_view_node(
    node: &ViewNode,
    string_signals: &[String],
    sig_idx: Option<&Cell<usize>>,
    variant_map: &HashMap<String, String>,
) -> ViewTree {
    match node {
        ViewNode::Element(el) => {
            let mut props = HashMap::new();
            let mut bindings = HashMap::new();
            let mut events = HashMap::new();
            let mut inline_style: Option<serde_json::Value> = None;
            let mut has_value_binding = false;
            let mut class_map_bindings_from_prop: HashMap<String, ExprIR> = HashMap::new();
            let mut motion_triggers: Vec<TriggerIntent> = Vec::new();
            let mut states_map: Option<HashMap<String, StateTweenIR>> = None;
            let mut animate_signal: Option<ExprIR> = None;
            let mut paused_override: Option<ExprIR> = None;
            let mut hover_reverse: Option<String> = None;
            let mut driven_by: Option<ExprIR> = None;
            let mut in_view_once: Option<ExprIR> = None;
            let mut in_view_repeat: Option<ExprIR> = None;
            let mut in_view_threshold: Option<ExprIR> = None;
            let mut scroll_start: Option<String> = None;
            let mut scroll_end: Option<String> = None;
            let mut scroll_scrub: Option<ExprIR> = None;
            let mut drag_constraints: Option<String> = None;
            let mut drag_elastic: Option<ExprIR> = None;
            let mut drag_scrub: Option<ExprIR> = None;
            let mut drag_momentum: Option<ExprIR> = None;
            let mut drag_threshold: Option<ExprIR> = None;
            let mut drag_velocity_max: Option<ExprIR> = None;
            let mut hero_tag: Option<String> = None;
            let mut hero_transition: Option<ExprIR> = None;

            for prop in &el.props {
                match prop {
                    ElementProp::Static { name, value } => {
                        props.insert(name.clone(), lower_expr(value, variant_map));
                    }
                    ElementProp::Bind { name, signal_path } => {
                        if name == "value" {
                            has_value_binding = true;
                        }
                        bindings.insert(
                            name.clone(),
                            SignalRefIR {
                                owner: None,
                                name: signal_path.clone(),
                            },
                        );
                    }
                    ElementProp::Event { name, handler } => {
                        // Collect event data kind
                        let event_data = map_event_data_kind(name);
                        let mut needs_pd = name == "onSubmit"; // backward compat
                        let mut needs_sp = false;
                        let mut used_fields: Option<Vec<String>> = None;

                        let ir_handler = match handler {
                            EventHandler::InlineFn { params, body, .. } => {
                                if event_data.is_some() {
                                    let collector = EventFieldCollector::collect(body);
                                    used_fields = Some(collector.fields);
                                    needs_pd = needs_pd || collector.needs_prevent_default;
                                    needs_sp = collector.needs_stop_propagation;
                                }
                                EventHandlerIR {
                                    kind: "inline_fn".into(),
                                    body: Some(lower_block(body, variant_map)),
                                    r#ref: None,
                                    params: params.iter().map(|p| p.name.clone()).collect(),
                                    event_data: event_data.clone(),
                                    used_fields: used_fields.clone(),
                                    needs_prevent_default: needs_pd,
                                    needs_stop_propagation: needs_sp,
                                }
                            }
                            EventHandler::FnRef(fn_name) => {
                                // Defer used_fields to Pass 2 (fn_ref backfill)
                                EventHandlerIR {
                                    kind: "fn_ref".into(),
                                    body: None,
                                    r#ref: Some(fn_name.clone()),
                                    params: vec![],
                                    event_data: event_data.clone(),
                                    used_fields: None, // backfilled in Pass 2
                                    needs_prevent_default: needs_pd,
                                    needs_stop_propagation: needs_sp,
                                }
                            }
                            EventHandler::StoreAction { store, action } => {
                                // Store actions receive no event data
                                EventHandlerIR {
                                    kind: "store_action".into(),
                                    body: None,
                                    r#ref: Some(format!("{}.{}", store, action)),
                                    params: vec![],
                                    event_data: None,
                                    used_fields: None,
                                    needs_prevent_default: false,
                                    needs_stop_propagation: false,
                                }
                            }
                        };
                        events.insert(name.clone(), ir_handler);
                    }
                    ElementProp::Class { value } => {
                        // class: styleRef or class: "literal-class"
                        let mut class_map_bindings: HashMap<String, ExprIR> = HashMap::new();
                        let class_str = match value {
                            ClassValue::StyleRef(s) => s.clone(),
                            ClassValue::StringLiteral(s) => s.clone(),
                            ClassValue::ClassMap(entries) => {
                                // Include classes based on their literal value:
                                //   Bool(true)  → static truth  → include the class
                                //   Bool(false) → static falsy → skip the class
                                //   dynamic     → unknown at compile time → JS handles
                                let mut static_classes: Vec<String> = Vec::new();
                                for (key, val_expr) in entries {
                                    let mut skip = false;
                                    if let Expr::Literal(Literal::Bool(false), _) = val_expr {
                                        // Explicitly false: exclude the class entirely
                                        skip = true;
                                    }
                                    // For true or dynamic values, include the class.
                                    // Dynamic toggling will be handled by JS later.
                                    if !skip {
                                        static_classes.push(key.clone());
                                    }
                                    // Collect dynamic bindings (non-literal values) for reactive JS toggling
                                    if !matches!(val_expr, Expr::Literal(Literal::Bool(_), _)) {
                                        class_map_bindings
                                            .insert(key.clone(), lower_expr(val_expr, variant_map));
                                    }
                                }
                                static_classes.join(" ")
                            }
                        };
                        props.insert(
                            "class".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::String(class_str),
                            },
                        );
                        // Pass class_map_bindings through the element
                        class_map_bindings_from_prop = class_map_bindings;
                    }
                    ElementProp::Style { entries } => {
                        // style: { key: value, ... } — passed as inline_style
                        // Lower each entry to a JSON representation for codegen
                        let style_map: serde_json::Map<String, serde_json::Value> = entries
                            .iter()
                            .map(|(key, expr)| {
                                let ir_expr = lower_expr(expr, variant_map);
                                let json_val = super::expr::lower_expr_to_json_from_ir(&ir_expr);
                                (key.clone(), json_val)
                            })
                            .collect();
                        inline_style = Some(serde_json::Value::Object(style_map));
                    }
                    ElementProp::TestId(id) => {
                        // test-id="..." attribute
                        props.insert(
                            "data-testid".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::String(id.clone()),
                            },
                        );
                    }
                    ElementProp::Ref(r) => {
                        // ref="..." attribute
                        props.insert(
                            "data-ref".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::String(r.clone()),
                            },
                        );
                    }
                    // Motion animation triggers: on:hover={fadeIn}, etc.
                    ElementProp::MotionTrigger(mt) => {
                        let motion_ref = match &mt.motion_expr {
                            Expr::VarRef(name, _) => Some(name.clone()),
                            _ => None,
                        };
                        motion_triggers.push(TriggerIntent {
                            kind: match &mt.trigger {
                                crate::parser::ast::TriggerKind::Enter => TriggerKind::Enter,
                                crate::parser::ast::TriggerKind::Leave => TriggerKind::Leave,
                                crate::parser::ast::TriggerKind::Hover => TriggerKind::Hover,
                                crate::parser::ast::TriggerKind::Unhover => TriggerKind::Unhover,
                                crate::parser::ast::TriggerKind::Press => TriggerKind::Press,
                                crate::parser::ast::TriggerKind::Focus => TriggerKind::Focus,
                                crate::parser::ast::TriggerKind::Animate => TriggerKind::Animate,
                                crate::parser::ast::TriggerKind::InView => TriggerKind::InView,
                                crate::parser::ast::TriggerKind::Scroll => TriggerKind::Scroll,
                                crate::parser::ast::TriggerKind::DragHorizontal => {
                                    TriggerKind::DragHorizontal
                                }
                                crate::parser::ast::TriggerKind::DragVertical => {
                                    TriggerKind::DragVertical
                                }
                                crate::parser::ast::TriggerKind::Timeline => TriggerKind::Timeline,
                            },
                            motion_ref,
                            inline_motion: None,
                        });
                    }
                    // States map: states: { visible: { ... }, hidden: { ... } }
                    ElementProp::StatesMap(states) => {
                        let mut states_hm: HashMap<String, StateTweenIR> = HashMap::new();
                        for (name, state_def) in states {
                            let properties: HashMap<String, ExprIR> = state_def
                                .properties
                                .iter()
                                .map(|(k, v)| (k.clone(), lower_expr(v, variant_map)))
                                .collect();
                            let over = state_def.over.as_ref().map(|e| lower_expr(e, variant_map));
                            let ease = state_def
                                .ease
                                .as_ref()
                                .map(super::motion::lower_easing);
                            states_hm.insert(
                                name.clone(),
                                StateTweenIR {
                                    properties,
                                    over,
                                    ease,
                                },
                            );
                        }
                        states_map = Some(states_hm);
                    }
                    // Animate signal: @state={signal}
                    ElementProp::AnimateSignal(expr) => {
                        animate_signal = Some(lower_expr(expr, variant_map));
                    }
                    // Paused override: @paused={expr}
                    ElementProp::PausedOverride(expr) => {
                        paused_override = Some(lower_expr(expr, variant_map));
                    }
                    // Hover reverse animation name
                    ElementProp::HoverReverse(s) => {
                        hover_reverse = Some(s.clone());
                    }
                    // Driven-by expression for external progress control
                    ElementProp::DrivenBy(expr) => {
                        driven_by = Some(lower_expr(expr, variant_map));
                    }
                    // InView trigger: once
                    ElementProp::InViewOnce(expr) => {
                        in_view_once = Some(lower_expr(expr, variant_map));
                    }
                    // InView trigger: repeat
                    ElementProp::InViewRepeat(expr) => {
                        in_view_repeat = Some(lower_expr(expr, variant_map));
                    }
                    // InView trigger: threshold
                    ElementProp::InViewThreshold(expr) => {
                        in_view_threshold = Some(lower_expr(expr, variant_map));
                    }
                    // Parent-child orchestration
                    ElementProp::When(_)
                    | ElementProp::DelayChildren(_)
                    | ElementProp::StaggerChildren(_) => {
                        // Lowered at trigger level; no direct IR field on ElementIR
                    }
                    // Scroll sub-attributes
                    ElementProp::ScrollStart(s) => {
                        scroll_start = Some(s.clone());
                    }
                    ElementProp::ScrollEnd(s) => {
                        scroll_end = Some(s.clone());
                    }
                    ElementProp::ScrollScrub(expr) => {
                        scroll_scrub = Some(lower_expr(expr, variant_map));
                    }
                    // Drag sub-attributes
                    ElementProp::DragConstraints(s) => {
                        drag_constraints = Some(s.clone());
                    }
                    ElementProp::DragElastic(expr) => {
                        drag_elastic = Some(lower_expr(expr, variant_map));
                    }
                    ElementProp::DragScrub(expr) => {
                        drag_scrub = Some(lower_expr(expr, variant_map));
                    }
                    ElementProp::DragMomentum(expr) => {
                        drag_momentum = Some(lower_expr(expr, variant_map));
                    }
                    ElementProp::DragThreshold(expr) => {
                        drag_threshold = Some(lower_expr(expr, variant_map));
                    }
                    ElementProp::DragVelocityMax(expr) => {
                        drag_velocity_max = Some(lower_expr(expr, variant_map));
                    }
                    // Hero attributes
                    ElementProp::HeroTag(s) => {
                        hero_tag = Some(s.clone());
                    }
                    ElementProp::HeroTransition(expr) => {
                        hero_transition = Some(lower_expr(expr, variant_map));
                    }
                }
            }

            // Auto-bind <input> elements to String signals if no explicit bind:value
            if let Some(sig_counter) = sig_idx {
                let idx = sig_counter.get();
                if el.tag == "input" && !has_value_binding && idx < string_signals.len() {
                    let sig_name = string_signals[idx].clone();
                    sig_counter.set(idx + 1);
                    bindings.insert(
                        "value".into(),
                        SignalRefIR {
                            owner: None,
                            name: sig_name,
                        },
                    );
                }
            }

            let children: Vec<ViewTree> = el
                .children
                .iter()
                .map(|c| lower_view_node(c, string_signals, sig_idx, variant_map))
                .collect();

            ViewTree::Element(ElementIR {
                tag: el.tag.clone(),
                props,
                bindings,
                events,
                style_classes: None,
                inline_style,
                children,
                test_id: None,
                ref_name: None,
                construction_kind: "tag-mapping".into(),
                class_map_bindings: class_map_bindings_from_prop,
                source_line: el.span.line,
                source_col: el.span.col,
                motion_triggers,
                states: states_map.map(|s| StatesIR {
                    states: s,
                    signal: animate_signal,
                }),
                paused_override,
                driven_by,
                hover_reverse,
                in_view_once,
                in_view_repeat,
                in_view_threshold,
                scroll_start,
                scroll_end,
                scroll_scrub,
                drag_constraints,
                drag_elastic,
                drag_scrub,
                drag_momentum,
                drag_threshold,
                drag_velocity_max,
                hero_tag,
                hero_transition,
            })
        }
        ViewNode::Text(s) => ViewTree::Text(s.clone()),
        ViewNode::Interpolation(expr) => {
            // Lower the inner expression to ExprIR so codegen can emit proper JS
            ViewTree::Interpolation(Box::new(lower_expr(expr, variant_map)))
        }
        ViewNode::IfDirective(if_dir) => {
            let branches: Vec<ConditionalBranch> = std::iter::once(ConditionalBranch {
                condition: lower_expr(&if_dir.condition, variant_map),
                body: Some(ViewTree::Element(ElementIR {
                    tag: "fragment".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: HashMap::new(),
                    style_classes: None,
                    inline_style: None,
                    children: if_dir
                        .body
                        .iter()
                        .map(|n| lower_view_node(n, string_signals, None, variant_map))
                        .collect(),
                    class_map_bindings: HashMap::new(),
                    test_id: None,
                    ref_name: None,
                    construction_kind: "conditional".into(),
                    source_line: 0,
                    source_col: 0,
                    motion_triggers: vec![],
                    states: None,
                    paused_override: None,
                    driven_by: None,
                    hover_reverse: None,
                    in_view_once: None,
                    in_view_repeat: None,
                    in_view_threshold: None,
                    scroll_start: None,
                    scroll_end: None,
                    scroll_scrub: None,
                    drag_constraints: None,
                    drag_elastic: None,
                    drag_scrub: None,
                    drag_momentum: None,
                    drag_threshold: None,
                    drag_velocity_max: None,
                    hero_tag: None,
                    hero_transition: None,
                })),
            })
            .chain(if_dir.else_ifs.iter().map(|(cond, body)| {
                ConditionalBranch {
                    condition: lower_expr(cond, variant_map),
                    body: Some(ViewTree::Element(ElementIR {
                        tag: "fragment".into(),
                        props: HashMap::new(),
                        bindings: HashMap::new(),
                        events: HashMap::new(),
                        style_classes: None,
                        inline_style: None,
                        class_map_bindings: HashMap::new(),
                        children: body
                            .iter()
                            .map(|n| lower_view_node(n, string_signals, None, variant_map))
                            .collect(),
                        test_id: None,
                        ref_name: None,
                        construction_kind: "conditional".into(),
                        source_line: 0,
                        source_col: 0,
                        motion_triggers: vec![],
                        states: None,
                        paused_override: None,
                        driven_by: None,
                        hover_reverse: None,
                        in_view_once: None,
                        in_view_repeat: None,
                        in_view_threshold: None,
                        scroll_start: None,
                        scroll_end: None,
                        scroll_scrub: None,
                        drag_constraints: None,
                        drag_elastic: None,
                        drag_scrub: None,
                        drag_momentum: None,
                        drag_threshold: None,
                        drag_velocity_max: None,
                        hero_tag: None,
                        hero_transition: None,
                    })),
                }
            }))
            .collect();

            let fallback = if_dir.else_body.as_ref().map(|body| {
                ViewTree::Element(ElementIR {
                    tag: "fragment".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: HashMap::new(),
                    style_classes: None,
                    inline_style: None,
                    children: body
                        .iter()
                        .map(|n| lower_view_node(n, string_signals, None, variant_map))
                        .collect(),
                    test_id: None,
                    ref_name: None,
                    class_map_bindings: HashMap::new(),
                    construction_kind: "conditional".into(),
                    source_line: 0,
                    source_col: 0,
                    motion_triggers: vec![],
                    states: None,
                    paused_override: None,
                    driven_by: None,
                    hover_reverse: None,
                    in_view_once: None,
                    in_view_repeat: None,
                    in_view_threshold: None,
                    scroll_start: None,
                    scroll_end: None,
                    scroll_scrub: None,
                    drag_constraints: None,
                    drag_elastic: None,
                    drag_scrub: None,
                    drag_momentum: None,
                    drag_threshold: None,
                    drag_velocity_max: None,
                    hero_tag: None,
                    hero_transition: None,
                })
            });

            ViewTree::Conditional(Box::new(ConditionalIntent {
                branches,
                fallback,
                transition_mode: if_dir.transition_mode.clone(),
            }))
        }
        ViewNode::ForDirective(for_dir) => {
            let lower_body = |nodes: &[ViewNode]| -> Option<ViewTree> {
                if nodes.is_empty() {
                    None
                } else {
                    Some(ViewTree::Element(ElementIR {
                        tag: "fragment".into(),
                        props: HashMap::new(),
                        bindings: HashMap::new(),
                        events: HashMap::new(),
                        style_classes: None,
                        inline_style: None,
                        class_map_bindings: HashMap::new(),
                        children: nodes
                            .iter()
                            .map(|n| lower_view_node(n, string_signals, None, variant_map))
                            .collect(),
                        test_id: None,
                        ref_name: None,
                        construction_kind: "list".into(),
                        source_line: 0,
                        source_col: 0,
                        motion_triggers: vec![],
                        states: None,
                        paused_override: None,
                        driven_by: None,
                        hover_reverse: None,
                        in_view_once: None,
                        in_view_repeat: None,
                        in_view_threshold: None,
                        scroll_start: None,
                        scroll_end: None,
                        scroll_scrub: None,
                        drag_constraints: None,
                        drag_elastic: None,
                        drag_scrub: None,
                        drag_momentum: None,
                        drag_threshold: None,
                        drag_velocity_max: None,
                        hero_tag: None,
                        hero_transition: None,
                    }))
                }
            };

            ViewTree::ListRendering(Box::new(ListRenderingIntent {
                iterable: lower_expr(&for_dir.iterable, variant_map),
                item_binding: BindingIR {
                    name: for_dir.item_name.clone(),
                    r#type: None,
                },
                body: lower_body(&for_dir.body),
                key_expr: for_dir
                    .key_expr
                    .as_ref()
                    .map(|e| lower_expr(e, variant_map)),
                empty_body: for_dir
                    .empty_body
                    .as_ref()
                    .and_then(|nodes| lower_body(nodes.as_slice())),
            }))
        }
        ViewNode::SlotDirective {
            name,
            default_content: _,
        } => ViewTree::Slot(name.clone()),
    }
}
