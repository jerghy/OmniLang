//! IR optimization passes.
//!
//! Pass 1: Dead computed-property elimination.
//! Walks all expressions, blocks, and view trees to find which computed
//! properties are actually referenced, then removes the unreferenced ones.
//!
//! Pass 2: Unreachable code elimination.
//! After encountering an unconditional return/break/continue/throw statement,
//! removes all subsequent statements in that block. Also recursively cleans
//! nested blocks inside if/while/for/try-catch statements.

use crate::ir::*;
use rustc_hash::FxHashSet;

/// Walk a serde_json::Value tree to find `$signal` references and collect their
/// names into `used`. These are inline style values lowered to JSON by the lowering
/// phase (e.g. `"5px solid " + activeStyle` → `{"$concat": ["5px solid ", {"$signal": "activeStyle"}]}`).
fn walk_json_for_signal_refs(value: &serde_json::Value, used: &mut FxHashSet<String>) {
    match value {
        serde_json::Value::Object(obj) => {
            if let Some(sig_name) = obj.get("$signal").and_then(|v| v.as_str()) {
                used.insert(sig_name.to_string());
            }
            for val in obj.values() {
                walk_json_for_signal_refs(val, used);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                walk_json_for_signal_refs(item, used);
            }
        }
        _ => {}
    }
}

/// Collect all IdentifierExpr names from an expression tree.
fn collect_idents(expr: &ExprIR, used: &mut FxHashSet<String>) {
    walk::walk_expr(expr, &mut |e| {
        if let ExprIR::IdentifierExpr { name, .. } = e {
            used.insert(name.clone());
        }
    });
}

/// Collect all IdentifierExpr names from a statement.
fn collect_idents_from_stmt(stmt: &StatementIR, used: &mut FxHashSet<String>) {
    walk::walk_stmt(stmt, &mut |e| {
        if let ExprIR::IdentifierExpr { name, .. } = e {
            used.insert(name.clone());
        }
    });
}

/// Collect all IdentifierExpr names from a block.
fn collect_idents_from_block(block: &BlockIR, used: &mut FxHashSet<String>) {
    for stmt in &block.statements {
        collect_idents_from_stmt(stmt, used);
    }
}

/// Walk PlaybackIR ExprIR fields and collect all identifier names.
fn collect_idents_from_playback(pb: &PlaybackIR, used: &mut FxHashSet<String>) {
    if let Some(ref expr) = pb.delay {
        collect_idents(expr, used);
    }
}

/// Walk a MotionBodyIR and collect all identifier names from ExprIR fields.
fn collect_idents_from_motion_body(body: &MotionBodyIR, used: &mut FxHashSet<String>) {
    let kf = body;
    collect_idents(&kf.over, used);
    for stop in &kf.stops {
        for expr in stop.properties.values() {
            collect_idents(expr, used);
        }
    }
    collect_idents_from_playback(&kf.playback, used);
}

/// Collect all identifier names from a view tree.
/// Uses `walk_view` for element iteration and handles non-element expressions
/// (Interpolation, ListRendering, Conditional) via direct traversal.
fn collect_idents_from_view(view: &ViewTree, used: &mut FxHashSet<String>) {
    // Walk elements via the shared walker
    walk::walk_view(view, &mut |el| {
        // Element bindings reference signals by name (e.g. in React codegen)
        for binding in el.bindings.values() {
            used.insert(binding.name.clone());
        }
        // Props are full expressions
        for prop in el.props.values() {
            collect_idents(prop, used);
        }
        // Class map bindings are expressions
        for expr in el.class_map_bindings.values() {
            collect_idents(expr, used);
        }
        // Event handlers — walk inline function bodies
        for handler in el.events.values() {
            if let EventHandlerIR::InlineFn { body, .. } = handler {
                collect_idents_from_block(body, used);
            }
        }
        // Walk inline style — may contain $signal references to computed properties
        if let Some(ref inline_style) = el.inline_style {
            walk_json_for_signal_refs(inline_style, used);
        }
        // Walk element-level behavior fields that can reference computed properties
        if let Some(ref motion) = el.motion {
            if let Some(ref expr) = motion.paused_override {
                collect_idents(expr, used);
            }
            if let Some(ref expr) = motion.in_view_once {
                collect_idents(expr, used);
            }
            if let Some(ref expr) = motion.in_view_repeat {
                collect_idents(expr, used);
            }
            if let Some(ref expr) = motion.in_view_threshold {
                collect_idents(expr, used);
            }
            for trigger in &motion.triggers {
                if let Some(ref body) = trigger.inline_motion {
                    collect_idents_from_motion_body(body, used);
                }
            }
        }
        if let Some(ref scroll) = el.scroll {
            if let Some(ref expr) = scroll.scrub {
                collect_idents(expr, used);
            }
        }
        if let Some(ref drag) = el.drag {
            if let Some(ref expr) = drag.elastic {
                collect_idents(expr, used);
            }
            if let Some(ref expr) = drag.threshold {
                collect_idents(expr, used);
            }
            if let Some(ref expr) = drag.velocity_max {
                collect_idents(expr, used);
            }
        }
    });

    // Collect identifiers from non-element expressions in the view tree
    // (walk_view only visits ElementIR nodes)
    collect_idents_from_view_exprs(view, used);
}

/// Collect identifiers from expression-only view nodes (Interpolation, ListRendering,
/// Conditional conditions) that are not covered by `walk_view`.
fn collect_idents_from_view_exprs(view: &ViewTree, used: &mut FxHashSet<String>) {
    match view {
        ViewTree::Interpolation(expr) => collect_idents(expr, used),
        ViewTree::ListRendering(lr) => {
            collect_idents(&lr.iterable, used);
            if let Some(ref key) = lr.key_expr {
                collect_idents(key, used);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                collect_idents(&branch.condition, used);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_idents_from_view_exprs(child, used);
            }
        }
        ViewTree::Element(el) => {
            for child in &el.children {
                collect_idents_from_view_exprs(child, used);
            }
        }
        ViewTree::Text(_) | ViewTree::Slot(_) => {}
    }
}

/// Eliminate computed properties never referenced by any function, effect,
/// lifecycle, computed, or view tree within the component.
pub fn eliminate_dead_computeds(comp: &mut ComponentIR) {
    let mut used: FxHashSet<String> = FxHashSet::default();

    // Walk all known reference sites within the component
    for f in &comp.fns {
        collect_idents_from_block(&f.body, &mut used);
    }
    for e in &comp.effects {
        collect_idents_from_block(&e.body, &mut used);
    }
    for lc in &comp.lifecycles {
        collect_idents_from_block(&lc.body, &mut used);
    }
    for c in &comp.computeds {
        collect_idents(&c.body, &mut used);
    }
    if let Some(ref view) = comp.view {
        collect_idents_from_view(view, &mut used);
    }

    // Walk component-level fields that can reference computed properties
    for expr in comp.prop_defaults.values() {
        collect_idents(expr, &mut used);
    }
    for style in &comp.styles {
        for expr in style.properties.values() {
            collect_idents(expr, &mut used);
        }
    }
    // motions and timelines are now canonical in ModuleIR, not ComponentIR

    let before = comp.computeds.len();
    comp.computeds.retain(|c| used.contains(&c.name));
    let removed = before - comp.computeds.len();

    if removed > 0 {
        log::info!(
            "Eliminated {removed} unused computed(s) from component '{}'",
            comp.name
        );
    }
}

// ── Pass 2: Unreachable code elimination ──

/// Returns `true` if the statement is an unconditional terminator (return, break,
/// continue, or throw). Statements after one of these in the same block are dead.
fn is_unconditional_terminator(stmt: &StatementIR) -> bool {
    matches!(
        stmt,
        StatementIR::ReturnStmtIR { .. }
            | StatementIR::BreakStmtIR
            | StatementIR::ContinueStmtIR
            | StatementIR::ThrowStmtIR { .. }
    )
}

/// Recursively eliminate unreachable code inside a single statement's nested blocks.
/// Returns `true` if any change was made.
fn eliminate_unreachable_in_stmt(stmt: &mut StatementIR) -> bool {
    let mut changed = false;
    match stmt {
        StatementIR::IfStmtIR {
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            changed |= eliminate_unreachable_block(then_body);
            for (_, body) in else_ifs.iter_mut() {
                changed |= eliminate_unreachable_block(body);
            }
            if let Some(body) = else_body.as_mut() {
                changed |= eliminate_unreachable_block(body);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            changed |= eliminate_unreachable_block(body);
        }
        StatementIR::WhileStmtIR { body, .. } => {
            changed |= eliminate_unreachable_block(body);
        }
        StatementIR::ForInStmtIR { body, .. } => {
            changed |= eliminate_unreachable_block(body);
        }
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            changed |= eliminate_unreachable_block(try_body);
            if let Some(body) = catch_body.as_mut() {
                changed |= eliminate_unreachable_block(body);
            }
            if let Some(body) = finally_body.as_mut() {
                changed |= eliminate_unreachable_block(body);
            }
        }
        _ => {}
    }
    changed
}

/// Remove all statements in `block` that follow an unconditional terminator
/// (return/break/continue/throw). Also recursively cleans nested blocks
/// (if/while/for/try-catch) for their own unreachable code.
///
/// Returns `true` if any statement was removed or any nested block was changed.
fn eliminate_unreachable_block(block: &mut BlockIR) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < block.statements.len() {
        // First, recursively clean any nested blocks inside this statement
        changed |= eliminate_unreachable_in_stmt(&mut block.statements[i]);

        // Then check if this statement is an unconditional terminator
        if is_unconditional_terminator(&block.statements[i]) {
            if i + 1 < block.statements.len() {
                block.statements.truncate(i + 1);
                changed = true;
            }
            break;
        }
        i += 1;
    }
    changed
}

/// Walk the view tree and clean inline event handler bodies.
fn eliminate_unreachable_in_view(view: &mut ViewTree) -> bool {
    let mut changed = false;
    match view {
        ViewTree::Element(el) => {
            for handler in el.events.values_mut() {
                if let EventHandlerIR::InlineFn { body, .. } = handler {
                    changed |= eliminate_unreachable_block(body);
                }
            }
            for child in &mut el.children {
                changed |= eliminate_unreachable_in_view(child);
            }
        }
        ViewTree::ListRendering(lr) => {
            if let Some(ref mut body) = lr.body {
                changed |= eliminate_unreachable_in_view(body);
            }
            if let Some(ref mut empty) = lr.empty_body {
                changed |= eliminate_unreachable_in_view(empty);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &mut cond.branches {
                if let Some(ref mut body) = branch.body {
                    changed |= eliminate_unreachable_in_view(body);
                }
            }
            if let Some(ref mut fallback) = cond.fallback {
                changed |= eliminate_unreachable_in_view(fallback);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children.iter_mut() {
                changed |= eliminate_unreachable_in_view(child);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) | ViewTree::Slot(_) => {}
    }
    changed
}

/// Run all optimization passes on a merged IR program.
pub fn optimize(ir: &mut IRProgram) {
    for module in ir.modules.values_mut() {
        for comp in module.components.values_mut() {
            eliminate_dead_computeds(comp);

            // Pass 2: Eliminate unreachable code in all code bodies
            for f in &mut comp.fns {
                eliminate_unreachable_block(&mut f.body);
            }
            for effect in &mut comp.effects {
                eliminate_unreachable_block(&mut effect.body);
            }
            for lc in &mut comp.lifecycles {
                eliminate_unreachable_block(&mut lc.body);
            }
            if let Some(ref mut view) = comp.view {
                eliminate_unreachable_in_view(view);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──

    fn int_computed(name: &str) -> ComputedIntent {
        ComputedIntent {
            name: name.to_string(),
            r#type: TypeIR {
                kind: TypeKind::Int,
                type_args: vec![],
                is_nullable: false,
            },
            body: ExprIR::LiteralExpr {
                value: serde_json::Value::Number(42.into()),
            },
            deps: vec![],
        }
    }

    fn ident(name: &str) -> ExprIR {
        ExprIR::IdentifierExpr {
            name: name.to_string(),
            type_ann: None,
        }
    }

    // ── existing tests ──

    #[test]
    fn removes_unused_computed() {
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("unused"), int_computed("used")],
            view: Some(ViewTree::Interpolation(Box::new(ident("used")))),
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);

        assert_eq!(comp.computeds.len(), 1);
        assert_eq!(comp.computeds[0].name, "used");
    }

    #[test]
    fn removes_unused_computed_without_motion_ref() {
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("motion_val")],
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);

        assert_eq!(comp.computeds.len(), 0);
    }

    // ── new tests: reference-site coverage ──

    #[test]
    fn computed_referenced_from_function_survives() {
        // A computed referenced by an IdentifierExpr inside a function body
        // should survive dead-computed elimination.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("helper")],
            fns: vec![FunctionIR {
                name: "myFn".to_string(),
                visibility: Visibility::Private,
                params: vec![],
                return_type: None,
                body: BlockIR {
                    statements: vec![StatementIR::ReturnStmtIR {
                        value: Some(ident("helper")),
                    }],
                },
                is_async: false,
                source_span: None,
            }],
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 1);
    }

    #[test]
    fn computed_referenced_from_effect_survives() {
        // A computed referenced inside an effect body should survive.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("tracked")],
            effects: vec![EffectIntent {
                deps: vec![],
                body: BlockIR {
                    statements: vec![StatementIR::ExprStmtIR {
                        expr: ident("tracked"),
                    }],
                },
            }],
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 1);
    }

    #[test]
    fn computed_referenced_from_lifecycle_survives() {
        // A computed referenced inside a lifecycle hook (mount) should survive.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("init_val")],
            lifecycles: vec![LifecycleIntent {
                kind: LifecycleKind::Mount,
                body: BlockIR {
                    statements: vec![StatementIR::ExprStmtIR {
                        expr: ident("init_val"),
                    }],
                },
            }],
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 1);
    }

    #[test]
    fn computed_referenced_from_another_computed_survives() {
        // A computed that depends on another computed (chain) should keep
        // its dependency alive.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![
                ComputedIntent {
                    name: "base".to_string(),
                    r#type: TypeIR {
                        kind: TypeKind::Int,
                        type_args: vec![],
                        is_nullable: false,
                    },
                    body: ExprIR::LiteralExpr {
                        value: serde_json::Value::Number(1.into()),
                    },
                    deps: vec![],
                },
                ComputedIntent {
                    name: "derived".to_string(),
                    r#type: TypeIR {
                        kind: TypeKind::Int,
                        type_args: vec![],
                        is_nullable: false,
                    },
                    body: ident("base"),
                    deps: vec!["base".to_string()],
                },
            ],
            view: Some(ViewTree::Interpolation(Box::new(ident("derived")))),
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 2);
    }

    #[test]
    fn computed_referenced_from_event_handler_survives() {
        // A computed referenced inside an inline event handler body should survive.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("click_handler")],
            view: Some(ViewTree::Element(Box::new(ElementIR {
                tag: "button".to_string(),
                props: FxHashMap::default(),
                bindings: FxHashMap::default(),
                events: {
                    let mut events = FxHashMap::default();
                    events.insert(
                        "click".to_string(),
                        EventHandlerIR::InlineFn {
                            params: vec![],
                            body: BlockIR {
                                statements: vec![StatementIR::ExprStmtIR {
                                    expr: ident("click_handler"),
                                }],
                            },
                            event_data: None,
                            used_fields: None,
                            needs_prevent_default: false,
                            needs_stop_propagation: false,
                        },
                    );
                    events
                },
                style_classes: None,
                inline_style: None,
                children: vec![],
                construction_kind: ConstructionKind::Normal,
                class_map_bindings: FxHashMap::default(),
                source_span: None,
                motion: None,
                scroll: None,
                drag: None,
                ref_binding: None,
            }))),
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 1);
    }

    #[test]
    fn computed_referenced_from_element_prop_survives() {
        // A computed used as an element prop value should survive.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("dynamic_src")],
            view: Some(ViewTree::Element(Box::new(ElementIR {
                tag: "img".to_string(),
                props: {
                    let mut props = FxHashMap::default();
                    props.insert("src".to_string(), ident("dynamic_src"));
                    props
                },
                bindings: FxHashMap::default(),
                events: FxHashMap::default(),
                style_classes: None,
                inline_style: None,
                children: vec![],
                construction_kind: ConstructionKind::Normal,
                class_map_bindings: FxHashMap::default(),
                source_span: None,
                motion: None,
                scroll: None,
                drag: None,
                ref_binding: None,
            }))),
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 1);
    }

    #[test]
    fn computed_referenced_from_class_map_survives() {
        // A computed used as a class-map binding expression should survive.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("is_active")],
            view: Some(ViewTree::Element(Box::new(ElementIR {
                tag: "div".to_string(),
                props: FxHashMap::default(),
                bindings: FxHashMap::default(),
                events: FxHashMap::default(),
                style_classes: None,
                inline_style: None,
                children: vec![],
                construction_kind: ConstructionKind::Normal,
                class_map_bindings: {
                    let mut map = FxHashMap::default();
                    map.insert("active".to_string(), ident("is_active"));
                    map
                },
                source_span: None,
                motion: None,
                scroll: None,
                drag: None,
                ref_binding: None,
            }))),
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 1);
    }

    #[test]
    fn computed_referenced_from_conditional_survives() {
        // A computed used as a conditional branch condition should survive.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("show_banner")],
            view: Some(ViewTree::Conditional(Box::new(ConditionalIntent {
                branches: vec![ConditionalBranch {
                    condition: ident("show_banner"),
                    body: Some(ViewTree::Text("visible".to_string())),
                }],
                fallback: None,
                transition_mode: None,
            }))),
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 1);
    }

    #[test]
    fn computed_referenced_from_list_iterable_survives() {
        // A computed used as the iterable expression of a list rendering
        // should survive.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("items")],
            view: Some(ViewTree::ListRendering(Box::new(ListRenderingIntent {
                iterable: ident("items"),
                item_binding: BindingIR {
                    name: "item".to_string(),
                    r#type: None,
                },
                body: None,
                key_expr: None,
                empty_body: None,
            }))),
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 1);
    }

    #[test]
    fn computed_referenced_from_prop_default_survives() {
        // A computed used as a prop default value should survive.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("default_title")],
            prop_defaults: {
                let mut defaults = FxHashMap::default();
                defaults.insert("title".to_string(), ident("default_title"));
                defaults
            },
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 1);
    }

    #[test]
    fn computed_referenced_from_style_property_survives() {
        // A computed used as a style property value should survive.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("dynamic_color")],
            styles: vec![StyleRuleIR {
                class_name: "box".to_string(),
                properties: {
                    let mut props = FxHashMap::default();
                    props.insert("color".to_string(), ident("dynamic_color"));
                    props
                },
                output_mode: "css".to_string(),
                is_tag_selector: false,
            }],
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 1);
    }

    #[test]
    fn multiple_unused_computeds_all_eliminated() {
        // When multiple computeds exist and none are referenced, all should
        // be eliminated.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("a"), int_computed("b"), int_computed("c")],
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 0);
    }

    #[test]
    fn all_used_computeds_survive() {
        // When all computeds are referenced from the view, none should be
        // eliminated.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![int_computed("x"), int_computed("y")],
            view: Some(ViewTree::Element(Box::new(ElementIR {
                tag: "div".to_string(),
                props: {
                    let mut props = FxHashMap::default();
                    props.insert("data-x".to_string(), ident("x"));
                    props.insert("data-y".to_string(), ident("y"));
                    props
                },
                bindings: FxHashMap::default(),
                events: FxHashMap::default(),
                style_classes: None,
                inline_style: None,
                children: vec![],
                construction_kind: ConstructionKind::Normal,
                class_map_bindings: FxHashMap::default(),
                source_span: None,
                motion: None,
                scroll: None,
                drag: None,
                ref_binding: None,
            }))),
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 2);
    }

    #[test]
    fn optimize_iterates_all_modules_and_components() {
        // The top-level `optimize()` function should walk every component
        // in every module and eliminate dead computeds from each.
        let mut prog = IRProgram {
            version: "0.1".to_string(),
            modules: {
                let mut modules = FxHashMap::default();
                for (mod_name, comp_name) in [("mod_a", "CompA"), ("mod_b", "CompB")] {
                    let mut comps = FxHashMap::default();
                    comps.insert(
                        comp_name.to_string(),
                        ComponentIR {
                            name: comp_name.to_string(),
                            computeds: vec![int_computed("dead")],
                            ..ComponentIR::default()
                        },
                    );
                    modules.insert(
                        mod_name.to_string(),
                        ModuleIR {
                            name: mod_name.to_string(),
                            components: comps,
                            ..Default::default()
                        },
                    );
                }
                modules
            },
            ..Default::default()
        };

        optimize(&mut prog);

        for module in prog.modules.values() {
            for comp in module.components.values() {
                assert!(
                    comp.computeds.is_empty(),
                    "component '{}' in module '{}' should have no computeds left",
                    comp.name,
                    module.name
                );
            }
        }
    }

    #[test]
    fn no_computeds_no_panic() {
        // optimize() should not panic when a component has zero computeds.
        let mut comp = ComponentIR {
            name: "Empty".to_string(),
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);
        assert_eq!(comp.computeds.len(), 0);
    }

    // ── unreachable code elimination tests ──

    fn make_expr_stmt() -> StatementIR {
        StatementIR::ExprStmtIR {
            expr: ExprIR::LiteralExpr {
                value: serde_json::Value::Number(0.into()),
            },
        }
    }

    fn make_return_stmt(value: u32) -> StatementIR {
        StatementIR::ReturnStmtIR {
            value: Some(ExprIR::LiteralExpr {
                value: serde_json::Value::Number(value.into()),
            }),
        }
    }

    fn make_break_stmt() -> StatementIR {
        StatementIR::BreakStmtIR
    }

    fn make_continue_stmt() -> StatementIR {
        StatementIR::ContinueStmtIR
    }

    fn make_throw_stmt(value: u32) -> StatementIR {
        StatementIR::ThrowStmtIR {
            expr: ExprIR::LiteralExpr {
                value: serde_json::Value::Number(value.into()),
            },
        }
    }

    #[test]
    fn return_kills_rest() {
        // { stmt1; return x; stmt2; } -> { stmt1; return x; } (stmt2 removed)
        let mut block = BlockIR {
            statements: vec![make_expr_stmt(), make_return_stmt(42), make_expr_stmt()],
        };

        let changed = eliminate_unreachable_block(&mut block);

        assert!(changed);
        assert_eq!(block.statements.len(), 2);
        assert!(matches!(
            block.statements[1],
            StatementIR::ReturnStmtIR { .. }
        ));
    }

    #[test]
    fn break_kills_rest() {
        // { stmt1; break; stmt2; } -> { stmt1; break; }
        let mut block = BlockIR {
            statements: vec![make_expr_stmt(), make_break_stmt(), make_expr_stmt()],
        };

        let changed = eliminate_unreachable_block(&mut block);

        assert!(changed);
        assert_eq!(block.statements.len(), 2);
        assert!(matches!(block.statements[1], StatementIR::BreakStmtIR));
    }

    #[test]
    fn continue_kills_rest() {
        // { stmt1; continue; stmt2; } -> { stmt1; continue; }
        let mut block = BlockIR {
            statements: vec![make_expr_stmt(), make_continue_stmt(), make_expr_stmt()],
        };

        let changed = eliminate_unreachable_block(&mut block);

        assert!(changed);
        assert_eq!(block.statements.len(), 2);
        assert!(matches!(block.statements[1], StatementIR::ContinueStmtIR));
    }

    #[test]
    fn throw_kills_rest() {
        // { stmt1; throw x; stmt2; } -> { stmt1; throw x; }
        let mut block = BlockIR {
            statements: vec![make_expr_stmt(), make_throw_stmt(99), make_expr_stmt()],
        };

        let changed = eliminate_unreachable_block(&mut block);

        assert!(changed);
        assert_eq!(block.statements.len(), 2);
        assert!(matches!(
            block.statements[1],
            StatementIR::ThrowStmtIR { .. }
        ));
    }

    #[test]
    fn no_terminator_no_change() {
        // { stmt1; stmt2; stmt3; } -> unchanged
        let mut block = BlockIR {
            statements: vec![make_expr_stmt(), make_expr_stmt(), make_expr_stmt()],
        };

        let changed = eliminate_unreachable_block(&mut block);

        assert!(!changed);
        assert_eq!(block.statements.len(), 3);
    }

    #[test]
    fn terminator_at_end_no_change() {
        // { stmt1; return x; } -> unchanged (nothing to remove)
        let mut block = BlockIR {
            statements: vec![make_expr_stmt(), make_return_stmt(1)],
        };

        let changed = eliminate_unreachable_block(&mut block);

        assert!(!changed);
        assert_eq!(block.statements.len(), 2);
    }

    #[test]
    fn nested_block_cleaned() {
        // Inside the if-body there is a return followed by dead code.
        // The outer block's statement after the if should remain.
        // { if cond { return x; dead_stmt; } surviving_stmt; }
        let mut block = BlockIR {
            statements: vec![
                StatementIR::IfStmtIR {
                    condition: ExprIR::LiteralExpr {
                        value: serde_json::Value::Bool(true),
                    },
                    then_body: BlockIR {
                        statements: vec![make_return_stmt(1), make_expr_stmt()],
                    },
                    else_ifs: vec![],
                    else_body: None,
                },
                make_expr_stmt(), // should survive
            ],
        };

        let changed = eliminate_unreachable_block(&mut block);

        assert!(changed);
        // Outer block stays 2 stmts (if + surviving)
        assert_eq!(block.statements.len(), 2);
        // Inner block should have been cleaned to 1 stmt (just the return)
        if let StatementIR::IfStmtIR { then_body, .. } = &block.statements[0] {
            assert_eq!(then_body.statements.len(), 1);
            assert!(matches!(
                then_body.statements[0],
                StatementIR::ReturnStmtIR { .. }
            ));
        } else {
            panic!("expected IfStmtIR");
        }
    }

    #[test]
    fn conditional_return_does_not_affect_outer_block() {
        // { if cond { return; } surviving; } -> surviving stays because
        // the return is inside a conditional branch.
        let mut block = BlockIR {
            statements: vec![
                StatementIR::IfStmtIR {
                    condition: ExprIR::LiteralExpr {
                        value: serde_json::Value::Bool(true),
                    },
                    then_body: BlockIR {
                        statements: vec![make_return_stmt(1)],
                    },
                    else_ifs: vec![],
                    else_body: None,
                },
                make_expr_stmt(), // must survive -- return was inside if, not flat
            ],
        };

        let changed = eliminate_unreachable_block(&mut block);

        assert!(!changed, "outer block should not change");
        assert_eq!(block.statements.len(), 2);
    }

    #[test]
    fn while_body_cleaned() {
        // { while cond { return; dead_stmt; } surviving; }
        let mut block = BlockIR {
            statements: vec![
                StatementIR::WhileStmtIR {
                    condition: ExprIR::LiteralExpr {
                        value: serde_json::Value::Bool(true),
                    },
                    body: BlockIR {
                        statements: vec![make_return_stmt(1), make_expr_stmt()],
                    },
                },
                make_expr_stmt(),
            ],
        };

        let changed = eliminate_unreachable_block(&mut block);

        assert!(changed);
        // Outer block unchanged (while + surviving)
        assert_eq!(block.statements.len(), 2);
        // While body should be cleaned
        if let StatementIR::WhileStmtIR { body, .. } = &block.statements[0] {
            assert_eq!(body.statements.len(), 1);
        } else {
            panic!("expected WhileStmtIR");
        }
    }

    #[test]
    fn for_in_body_cleaned() {
        // { for item in iter { return; dead_stmt; } surviving; }
        let mut block = BlockIR {
            statements: vec![
                StatementIR::ForInStmtIR {
                    var_name: "item".to_string(),
                    iterable: ExprIR::LiteralExpr {
                        value: serde_json::Value::String("test".to_string()),
                    },
                    body: BlockIR {
                        statements: vec![make_return_stmt(1), make_expr_stmt()],
                    },
                },
                make_expr_stmt(),
            ],
        };

        let changed = eliminate_unreachable_block(&mut block);

        assert!(changed);
        // Outer block unchanged
        assert_eq!(block.statements.len(), 2);
        // For-in body should be cleaned
        if let StatementIR::ForInStmtIR { body, .. } = &block.statements[0] {
            assert_eq!(body.statements.len(), 1);
        } else {
            panic!("expected ForInStmtIR");
        }
    }

    #[test]
    fn optimize_function_bodies() {
        // Integration test: `optimize()` should walk function bodies and
        // eliminate unreachable code across all components.
        let mut prog = IRProgram {
            version: "0.1".to_string(),
            modules: {
                let mut modules = FxHashMap::default();
                let mut comps = FxHashMap::default();
                comps.insert(
                    "Comp".to_string(),
                    ComponentIR {
                        name: "Comp".to_string(),
                        fns: vec![FunctionIR {
                            name: "bad".to_string(),
                            visibility: Visibility::Private,
                            params: vec![],
                            return_type: None,
                            body: BlockIR {
                                statements: vec![
                                    make_expr_stmt(),
                                    make_return_stmt(1),
                                    make_expr_stmt(), // dead
                                ],
                            },
                            is_async: false,
                            source_span: None,
                        }],
                        effects: vec![EffectIntent {
                            deps: vec![],
                            body: BlockIR {
                                statements: vec![
                                    make_expr_stmt(),
                                    make_throw_stmt(2),
                                    make_expr_stmt(), // dead
                                ],
                            },
                        }],
                        ..ComponentIR::default()
                    },
                );
                modules.insert(
                    "mod".to_string(),
                    ModuleIR {
                        name: "mod".to_string(),
                        components: comps,
                        ..Default::default()
                    },
                );
                modules
            },
            ..Default::default()
        };

        optimize(&mut prog);

        // Function body should have 2 statements (expr + return)
        let comp = &prog.modules["mod"].components["Comp"];
        assert_eq!(comp.fns[0].body.statements.len(), 2);
        // Effect body should have 2 statements (expr + throw)
        assert_eq!(comp.effects[0].body.statements.len(), 2);
    }

    #[test]
    fn multiple_returns_only_first_triggers() {
        // { return 1; return 2; return 3; } -> { return 1; }
        // Only the first return (and anything before it) survives.
        let mut block = BlockIR {
            statements: vec![
                make_return_stmt(1),
                make_return_stmt(2),
                make_return_stmt(3),
            ],
        };

        let changed = eliminate_unreachable_block(&mut block);

        assert!(changed);
        assert_eq!(block.statements.len(), 1);
        assert!(matches!(
            &block.statements[0],
            StatementIR::ReturnStmtIR { .. }
        ));
    }
}
