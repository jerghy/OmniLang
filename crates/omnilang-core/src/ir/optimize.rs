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

/// Walk an ExprIR tree and collect all IdentifierExpr names into `used`.
fn walk_expr_for_idents(expr: &ExprIR, used: &mut FxHashSet<String>) {
    match expr {
        ExprIR::IdentifierExpr { name, .. } => {
            used.insert(name.clone());
        }
        ExprIR::FieldAccessExpr { object, .. } => walk_expr_for_idents(object, used),
        ExprIR::OptionalChainExpr { object, .. } => walk_expr_for_idents(object, used),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            walk_expr_for_idents(left, used);
            walk_expr_for_idents(right, used);
        }
        ExprIR::UnaryOpExpr { operand, .. } => walk_expr_for_idents(operand, used),
        ExprIR::CallExpr { args, .. } => {
            for arg in args {
                walk_expr_for_idents(arg, used);
            }
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            walk_expr_for_idents(object, used);
            for arg in args {
                walk_expr_for_idents(arg, used);
            }
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            walk_expr_for_idents(cond, used);
            walk_expr_for_idents(then_expr, used);
            walk_expr_for_idents(else_expr, used);
        }
        ExprIR::IndexExpr { object, index } => {
            walk_expr_for_idents(object, used);
            walk_expr_for_idents(index, used);
        }
        ExprIR::ArrayExpr { elements } => {
            for elem in elements {
                walk_expr_for_idents(elem, used);
            }
        }
        ExprIR::SpreadExpr { operand } => walk_expr_for_idents(operand, used),
        ExprIR::StructExpr { fields, .. } => {
            for (_, field_expr) in fields {
                walk_expr_for_idents(field_expr, used);
            }
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for arg in call_args {
                walk_expr_for_idents(arg, used);
            }
        }
        ExprIR::BlockExpr { statements, tail } => {
            for stmt in statements {
                walk_stmt_for_idents(stmt, used);
            }
            walk_expr_for_idents(tail, used);
        }
        ExprIR::ClosureExpr { body, .. } => walk_expr_for_idents(body, used),
        ExprIR::AwaitExpr { operand } => walk_expr_for_idents(operand, used),
        ExprIR::VariantLiteral { fields, .. } => {
            for (_, field_expr) in fields {
                walk_expr_for_idents(field_expr, used);
            }
        }
        ExprIR::MatchExpr { matched, arms } => {
            walk_expr_for_idents(matched, used);
            for arm in arms {
                walk_block_for_idents(&arm.body, used);
            }
        }
        ExprIR::MapExpr { entries } => {
            for (key, value) in entries {
                walk_expr_for_idents(key, used);
                walk_expr_for_idents(value, used);
            }
        }
        // Literals contain no identifiers
        ExprIR::LiteralExpr { .. } => {}
    }
}

/// Walk a single StatementIR and collect all identifier names.
fn walk_stmt_for_idents(stmt: &StatementIR, used: &mut FxHashSet<String>) {
    match stmt {
        StatementIR::ExprStmtIR { expr } => walk_expr_for_idents(expr, used),
        StatementIR::LetStmtIR { value, .. } => {
            if let Some(v) = value {
                walk_expr_for_idents(v, used);
            }
        }
        StatementIR::ReturnStmtIR { value } => {
            if let Some(v) = value {
                walk_expr_for_idents(v, used);
            }
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            walk_expr_for_idents(condition, used);
            walk_block_for_idents(then_body, used);
            for (cond, body) in else_ifs {
                walk_expr_for_idents(cond, used);
                walk_block_for_idents(body, used);
            }
            if let Some(b) = else_body {
                walk_block_for_idents(b, used);
            }
        }
        StatementIR::MutationStmt { intent } => {
            if let Some(intent) = intent {
                walk_mutation_intent(intent, used);
            }
        }
        StatementIR::EventEmitStmtIR { args, .. } => {
            for arg in args {
                walk_expr_for_idents(arg, used);
            }
        }
        StatementIR::BlockStmtIR { body } => walk_block_for_idents(body, used),
        StatementIR::WhileStmtIR { condition, body } => {
            walk_expr_for_idents(condition, used);
            walk_block_for_idents(body, used);
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            walk_expr_for_idents(iterable, used);
            walk_block_for_idents(body, used);
        }
        StatementIR::ThrowStmtIR { expr } => walk_expr_for_idents(expr, used),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            walk_block_for_idents(try_body, used);
            if let Some(b) = catch_body {
                walk_block_for_idents(b, used);
            }
            if let Some(b) = finally_body {
                walk_block_for_idents(b, used);
            }
        }
        // Break and continue have no expressions
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
    }
}

/// Walk a BlockIR and collect all identifier names.
fn walk_block_for_idents(block: &BlockIR, used: &mut FxHashSet<String>) {
    for stmt in &block.statements {
        walk_stmt_for_idents(stmt, used);
    }
}

/// Walk a MutationIntent and collect all identifier names from expressions it wraps.
fn walk_mutation_intent(intent: &MutationIntent, used: &mut FxHashSet<String>) {
    match intent {
        MutationIntent::DirectAssign { value, .. } => {
            if let Some(v) = value {
                walk_expr_for_idents(v, used);
            }
        }
        MutationIntent::Increment { amount, .. }
        | MutationIntent::Decrement { amount, .. }
        | MutationIntent::Multiply { amount, .. }
        | MutationIntent::Divide { amount, .. }
        | MutationIntent::Modulo { amount, .. } => {
            if let Some(v) = amount {
                walk_expr_for_idents(v, used);
            }
        }
        MutationIntent::Push { item, .. } => {
            if let Some(v) = item {
                walk_expr_for_idents(v, used);
            }
        }
        MutationIntent::Pop { .. } => {}
    }
}

/// Walk a ViewTree and collect all identifier names.
fn walk_view_for_idents(view: &ViewTree, used: &mut FxHashSet<String>) {
    match view {
        ViewTree::Element(el) => {
            // Element bindings reference signals by name (e.g. in React codegen)
            for binding in el.bindings.values() {
                used.insert(binding.name.clone());
            }
            // Props are full expressions
            for prop in el.props.values() {
                walk_expr_for_idents(prop, used);
            }
            // Class map bindings are expressions
            for expr in el.class_map_bindings.values() {
                walk_expr_for_idents(expr, used);
            }
            // Event handlers — walk inline function bodies
            for handler in el.events.values() {
                match handler {
                    EventHandlerIR::InlineFn { body, .. } => {
                        walk_block_for_idents(body, used);
                    }
                    EventHandlerIR::FnRef { .. } | EventHandlerIR::StoreAction { .. } => {}
                }
            }
            // Recurse into child view trees
            for child in &el.children {
                walk_view_for_idents(child, used);
            }

            // Walk element-level behavior fields that can reference computed properties
            if let Some(ref motion) = el.motion {
                // Direct ExprIR fields on MotionBehavior
                if let Some(ref expr) = motion.paused_override {
                    walk_expr_for_idents(expr, used);
                }
                if let Some(ref expr) = motion.driven_by {
                    walk_expr_for_idents(expr, used);
                }
                if let Some(ref expr) = motion.in_view_once {
                    walk_expr_for_idents(expr, used);
                }
                if let Some(ref expr) = motion.in_view_repeat {
                    walk_expr_for_idents(expr, used);
                }
                if let Some(ref expr) = motion.in_view_threshold {
                    walk_expr_for_idents(expr, used);
                }
                // StatesIR on MotionBehavior
                if let Some(ref states) = motion.states {
                    if let Some(ref expr) = states.signal {
                        walk_expr_for_idents(expr, used);
                    }
                    for tween in states.states.values() {
                        if let Some(ref over) = tween.over {
                            walk_expr_for_idents(over, used);
                        }
                        for expr in tween.properties.values() {
                            walk_expr_for_idents(expr, used);
                        }
                    }
                }
                // Triggers on MotionBehavior
                for trigger in &motion.triggers {
                    if let Some(ref body) = trigger.inline_motion {
                        walk_motion_body(body, used);
                    }
                }
            }
            if let Some(ref scroll) = el.scroll {
                if let Some(ref expr) = scroll.scrub {
                    walk_expr_for_idents(expr, used);
                }
            }
            if let Some(ref drag) = el.drag {
                if let Some(ref expr) = drag.elastic {
                    walk_expr_for_idents(expr, used);
                }
                if let Some(ref expr) = drag.scrub {
                    walk_expr_for_idents(expr, used);
                }
                if let Some(ref expr) = drag.momentum {
                    walk_expr_for_idents(expr, used);
                }
                if let Some(ref expr) = drag.threshold {
                    walk_expr_for_idents(expr, used);
                }
                if let Some(ref expr) = drag.velocity_max {
                    walk_expr_for_idents(expr, used);
                }
            }
            if let Some(ref hero) = el.hero {
                if let Some(ref expr) = hero.transition {
                    walk_expr_for_idents(expr, used);
                }
            }
        }
        ViewTree::Interpolation(expr) => walk_expr_for_idents(expr, used),
        ViewTree::ListRendering(lr) => {
            walk_expr_for_idents(&lr.iterable, used);
            if let Some(ref body) = lr.body {
                walk_view_for_idents(body, used);
            }
            if let Some(ref key) = lr.key_expr {
                walk_expr_for_idents(key, used);
            }
            if let Some(ref empty) = lr.empty_body {
                walk_view_for_idents(empty, used);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                walk_expr_for_idents(&branch.condition, used);
                if let Some(ref body) = branch.body {
                    walk_view_for_idents(body, used);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                walk_view_for_idents(fallback, used);
            }
        }
        ViewTree::Text(_) | ViewTree::Slot(_) => {}
    }
}

/// Walk PlaybackIR ExprIR fields and collect all identifier names.
fn walk_playback(pb: &PlaybackIR, used: &mut FxHashSet<String>) {
    if let Some(ref expr) = pb.delay {
        walk_expr_for_idents(expr, used);
    }
    if let Some(ref expr) = pb.repeat_delay {
        walk_expr_for_idents(expr, used);
    }
    if let Some(ref expr) = pb.time_scale {
        walk_expr_for_idents(expr, used);
    }
    if let Some(ref expr) = pb.delay_children {
        walk_expr_for_idents(expr, used);
    }
    if let Some(ref expr) = pb.stagger_children {
        walk_expr_for_idents(expr, used);
    }
}

/// Walk a TimelineItemIR items and collect all identifier names from ExprIR fields.
fn walk_timeline_items(items: &[TimelineItemIR], used: &mut FxHashSet<String>) {
    for item in items {
        match &item.kind {
            TimelineItemKindIR::Direction {
                properties,
                over,
                delay,
                ..
            } => {
                for expr in properties.values() {
                    walk_expr_for_idents(expr, used);
                }
                walk_expr_for_idents(over, used);
                if let Some(ref expr) = delay {
                    walk_expr_for_idents(expr, used);
                }
            }
            TimelineItemKindIR::Set(entries) => {
                for (_, expr) in entries {
                    walk_expr_for_idents(expr, used);
                }
            }
            TimelineItemKindIR::MotionRef(_) => {}
            TimelineItemKindIR::Stagger(motion_expr, stagger_timing, _) => {
                match motion_expr.as_ref() {
                    MotionExprIR::Ref(_) => {}
                    MotionExprIR::Inline(body) => walk_motion_body(body, used),
                }
                walk_expr_for_idents(&stagger_timing.each, used);
            }
            TimelineItemKindIR::Label(_, delay) => {
                if let Some(ref expr) = delay {
                    walk_expr_for_idents(expr, used);
                }
            }
        }
    }
}

/// Walk a MotionBodyIR and collect all identifier names from ExprIR fields.
fn walk_motion_body(body: &MotionBodyIR, used: &mut FxHashSet<String>) {
    match body {
        MotionBodyIR::Keyframe(kf) => {
            walk_expr_for_idents(&kf.over, used);
            for (_, expr) in &kf.callbacks {
                walk_expr_for_idents(expr, used);
            }
            for stop in &kf.stops {
                for expr in stop.properties.values() {
                    walk_expr_for_idents(expr, used);
                }
            }
            walk_playback(&kf.playback, used);
        }
        MotionBodyIR::Timeline(tl) => {
            walk_timeline_items(&tl.items, used);
            walk_playback(&tl.playback, used);
        }
        MotionBodyIR::CallbackOnly(cb) => {
            walk_expr_for_idents(&cb.over, used);
            for (_, expr) in &cb.callbacks {
                walk_expr_for_idents(expr, used);
            }
            walk_playback(&cb.playback, used);
        }
    }
}

/// Eliminate computed properties never referenced by any function, effect,
/// lifecycle, computed, or view tree within the component.
pub fn eliminate_dead_computeds(comp: &mut ComponentIR) {
    let mut used: FxHashSet<String> = FxHashSet::default();

    // Walk all known reference sites within the component
    for f in &comp.fns {
        walk_block_for_idents(&f.body, &mut used);
    }
    for e in &comp.effects {
        walk_block_for_idents(&e.body, &mut used);
    }
    for lc in &comp.lifecycles {
        walk_block_for_idents(&lc.body, &mut used);
    }
    for c in &comp.computeds {
        walk_expr_for_idents(&c.body, &mut used);
    }
    if let Some(ref view) = comp.view {
        walk_view_for_idents(view, &mut used);
    }

    // Walk component-level fields that can reference computed properties
    for expr in comp.prop_defaults.values() {
        walk_expr_for_idents(expr, &mut used);
    }
    for style in &comp.styles {
        for expr in style.properties.values() {
            walk_expr_for_idents(expr, &mut used);
        }
    }
    for hero in &comp.hero_defs {
        walk_expr_for_idents(&hero.over, &mut used);
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
            view: Some(ViewTree::Element(ElementIR {
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
                test_id: None,
                ref_name: None,
                construction_kind: ConstructionKind::Normal,
                class_map_bindings: FxHashMap::default(),
                source_span: None,
                motion: None,
                scroll: None,
                drag: None,
                hero: None,
            })),
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
            view: Some(ViewTree::Element(ElementIR {
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
                test_id: None,
                ref_name: None,
                construction_kind: ConstructionKind::Normal,
                class_map_bindings: FxHashMap::default(),
                source_span: None,
                motion: None,
                scroll: None,
                drag: None,
                hero: None,
            })),
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
            view: Some(ViewTree::Element(ElementIR {
                tag: "div".to_string(),
                props: FxHashMap::default(),
                bindings: FxHashMap::default(),
                events: FxHashMap::default(),
                style_classes: None,
                inline_style: None,
                children: vec![],
                test_id: None,
                ref_name: None,
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
                hero: None,
            })),
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
            view: Some(ViewTree::Element(ElementIR {
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
                test_id: None,
                ref_name: None,
                construction_kind: ConstructionKind::Normal,
                class_map_bindings: FxHashMap::default(),
                source_span: None,
                motion: None,
                scroll: None,
                drag: None,
                hero: None,
            })),
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
