//! IR optimization passes.
//!
//! Pass 1: Dead computed-property elimination.
//! Walks all expressions, blocks, and view trees to find which computed
//! properties are actually referenced, then removes the unreferenced ones.

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
    for motion in &comp.motions {
        walk_motion_body(&motion.body, &mut used);
    }
    for timeline in &comp.timelines {
        walk_timeline_items(&timeline.items, &mut used);
    }

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

/// Run all optimization passes on a merged IR program.
pub fn optimize(ir: &mut IRProgram) {
    for module in ir.modules.values_mut() {
        for comp in module.components.values_mut() {
            eliminate_dead_computeds(comp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_unused_computed() {
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![
                ComputedIntent {
                    name: "unused".to_string(),
                    r#type: TypeIR {
                        kind: TypeKind::Int,
                        type_args: vec![],
                        is_nullable: false,
                    },
                    body: ExprIR::LiteralExpr {
                        value: serde_json::Value::Number(42.into()),
                    },
                    deps: vec![],
                },
                ComputedIntent {
                    name: "used".to_string(),
                    r#type: TypeIR {
                        kind: TypeKind::Int,
                        type_args: vec![],
                        is_nullable: false,
                    },
                    body: ExprIR::LiteralExpr {
                        value: serde_json::Value::Number(0.into()),
                    },
                    deps: vec![],
                },
            ],
            // The view references "used" via IdentifierExpr
            view: Some(ViewTree::Interpolation(Box::new(ExprIR::IdentifierExpr {
                name: "used".to_string(),
                type_ann: None,
            }))),
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);

        assert_eq!(comp.computeds.len(), 1);
        assert_eq!(comp.computeds[0].name, "used");
    }

    #[test]
    fn removes_unused_computed_in_motion_behavior() {
        // A computed referenced only in component-level @motion keyframe over
        // should NOT be eliminated.
        let mut comp = ComponentIR {
            name: "TestComp".to_string(),
            computeds: vec![ComputedIntent {
                name: "motion_val".to_string(),
                r#type: TypeIR {
                    kind: TypeKind::Int,
                    type_args: vec![],
                    is_nullable: false,
                },
                body: ExprIR::LiteralExpr {
                    value: serde_json::Value::Number(42.into()),
                },
                deps: vec![],
            }],
            motions: vec![MotionIntent {
                name: "myMotion".to_string(),
                body: MotionBodyIR::Keyframe(KeyframeIR {
                    over: ExprIR::IdentifierExpr {
                        name: "motion_val".to_string(),
                        type_ann: None,
                    },
                    stops: vec![],
                    ease: None,
                    color_space: None,
                    playback: PlaybackIR::default(),
                    path: None,
                    path_rotate: None,
                    callbacks: vec![],
                    source_span: None,
                }),
                source_span: None,
            }],
            ..ComponentIR::default()
        };

        eliminate_dead_computeds(&mut comp);

        assert_eq!(comp.computeds.len(), 1);
        assert_eq!(comp.computeds[0].name, "motion_val");
    }
}
