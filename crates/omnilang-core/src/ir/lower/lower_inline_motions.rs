//! Lowering pass: hoist inline motion bodies into named MotionIntents.
//!
//! Inline motion bodies appear on element triggers:
//!
//! ```omni
//! <div @enter={{ opacity: 0, scale: 0.8 }}></div>
//! ```
//!
//! During view lowering these become `TriggerIntent::inline_motion`
//! (a `KeyframeIR` built from the map literal). This pass extracts them,
//! synthesizes a name (`__anon_m_N`), and hoists the body into the module's
//! motion table so codegen sees only named motions.

use crate::ir::*;

/// Lower inline motion bodies into named MotionIntents.
///
/// Each trigger with an inline body gets a synthesized name (`__anon_m_N`)
/// and the body is appended to `module.motions`; the trigger is rewritten to
/// reference the synthesized name. Runs after component lowering (so
/// `inline_motion` fields are populated) and before the module `motion_index`
/// is built, so the synthesized motions are indexed too.
pub(super) fn lower_inline_motions(module: &mut ModuleIR) {
    let mut counter = 0u32;
    let mut new_motions: Vec<MotionIntent> = Vec::new();

    for component in module.components.values_mut() {
        if let Some(ref mut view) = component.view {
            walk_and_lower_inline_motions(view, &mut counter, &mut new_motions);
        }
        // Body-level slot fallbacks (ADR-053 D5) may contain inline motion
        // triggers too — hoist them so codegen sees only named motions.
        for slot in &mut component.slots {
            for child in &mut slot.fallback {
                walk_and_lower_inline_motions(child, &mut counter, &mut new_motions);
            }
        }
    }
    module.motions.extend(new_motions);
}

fn walk_and_lower_inline_motions(
    tree: &mut ViewTree,
    counter: &mut u32,
    new_motions: &mut Vec<MotionIntent>,
) {
    match tree {
        ViewTree::Element(el) => {
            if let Some(behavior) = el.motion.as_deref_mut() {
                for trigger in &mut behavior.triggers {
                    if let Some(body) = trigger.inline_motion.take() {
                        let name = format!("__anon_m_{}", counter);
                        *counter += 1;
                        new_motions.push(MotionIntent {
                            name: name.clone(),
                            body,
                            source_span: trigger.source_span,
                        });
                        trigger.motion_ref = Some(name);
                    }
                }
            }
            for child in &mut el.children {
                walk_and_lower_inline_motions(child, counter, new_motions);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &mut cond.branches {
                if let Some(ref mut body) = branch.body {
                    walk_and_lower_inline_motions(body, counter, new_motions);
                }
            }
            if let Some(ref mut fallback) = cond.fallback {
                walk_and_lower_inline_motions(fallback, counter, new_motions);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref mut body) = list.body {
                walk_and_lower_inline_motions(body, counter, new_motions);
            }
            if let Some(ref mut empty) = list.empty_body {
                walk_and_lower_inline_motions(empty, counter, new_motions);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                walk_and_lower_inline_motions(child, counter, new_motions);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                walk_and_lower_inline_motions(child, counter, new_motions);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use crate::semantic::analyze;

    fn lower_source(source: &str) -> IRProgram {
        let tokens = tokenize(source);
        let (program, _) = parse(tokens, source);
        let mut semantics = analyze(&program);
        crate::ir::lower(&program, &mut semantics, None)
    }

    /// Collect all (motion_ref, has_inline) trigger pairs from a view tree.
    fn collect_triggers(tree: &ViewTree, out: &mut Vec<(Option<String>, bool)>) {
        match tree {
            ViewTree::Element(el) => {
                if let Some(motion) = el.motion.as_deref() {
                    for t in &motion.triggers {
                        out.push((t.motion_ref.clone(), t.inline_motion.is_some()));
                    }
                }
                for child in &el.children {
                    collect_triggers(child, out);
                }
            }
            ViewTree::Conditional(cond) => {
                for branch in &cond.branches {
                    if let Some(body) = &branch.body {
                        collect_triggers(body, out);
                    }
                }
                if let Some(fallback) = &cond.fallback {
                    collect_triggers(fallback, out);
                }
            }
            ViewTree::ListRendering(list) => {
                if let Some(body) = &list.body {
                    collect_triggers(body, out);
                }
                if let Some(empty) = &list.empty_body {
                    collect_triggers(empty, out);
                }
            }
            ViewTree::Slot { fallback, .. } => {
                for child in fallback {
                    collect_triggers(child, out);
                }
            }
            ViewTree::Fragment(children) => {
                for child in children {
                    collect_triggers(child, out);
                }
            }
            ViewTree::Interpolation(_) | ViewTree::Text(_) => {}
        }
    }

    #[test]
    fn hoists_inline_enter_motion() {
        let source = r#"
import omniview;
component FadeIn {
    view {
        <div @enter={{ opacity: 0, scale: 0.8 }}>Hello</div>
    }
}
"#;
        let ir = lower_source(source);
        let module = ir.modules.get("input.omni").expect("should have module");

        // Inline body hoisted to a named MotionIntent
        let anon = module
            .motions
            .iter()
            .find(|m| m.name == "__anon_m_0")
            .expect("should have synthesized __anon_m_0 motion");
        assert_eq!(
            anon.body.stops.len(),
            1,
            "inline tween is a single from-stop"
        );
        let stop = &anon.body.stops[0];
        assert_eq!(stop.position, 0.0, "inline tween starts at position 0");
        let opacity = stop
            .properties
            .get("opacity")
            .expect("stop should have opacity");
        assert!(
            matches!(opacity, ExprIR::LiteralExpr { value } if value == &serde_json::json!(0)),
            "opacity should be 0, got {:?}",
            opacity
        );
        let scale = stop
            .properties
            .get("scale")
            .expect("stop should have scale");
        assert!(
            matches!(scale, ExprIR::LiteralExpr { value } if value == &serde_json::json!(0.8)),
            "scale should be 0.8, got {:?}",
            scale
        );
        // Default duration for inline tweens (no `over` clause in the source)
        assert!(
            matches!(&anon.body.over, ExprIR::LiteralExpr { value } if value == &serde_json::json!(400)),
            "default inline duration should be 400ms, got {:?}",
            anon.body.over
        );

        // Synthesized motion is indexed alongside named ones
        assert!(
            ir.motion_index.contains_key("__anon_m_0"),
            "motion_index should include synthesized motion"
        );

        // Trigger now references the synthesized motion by name
        let comp = module
            .components
            .get("FadeIn")
            .expect("should have FadeIn component");
        let view = comp.view.as_ref().expect("component should have a view");
        let mut triggers = Vec::new();
        collect_triggers(view, &mut triggers);
        assert_eq!(
            triggers,
            vec![(Some("__anon_m_0".to_string()), false)],
            "trigger should reference the named motion with no inline body left"
        );
    }

    #[test]
    fn keeps_named_motion_refs_untouched() {
        let source = r#"
import omniview;
component NamedMotion {
    @motion fadeIn {
        from { opacity: 0 } to { opacity: 1 }
        over: 300ms
    }
    view {
        <div @enter={fadeIn}>Hello</div>
    }
}
"#;
        let ir = lower_source(source);
        let module = ir.modules.get("input.omni").expect("should have module");
        assert!(
            module
                .motions
                .iter()
                .all(|m| !m.name.starts_with("__anon_m_")),
            "named motion refs should not synthesize anonymous motions"
        );

        let comp = module
            .components
            .get("NamedMotion")
            .expect("should have NamedMotion component");
        let view = comp.view.as_ref().expect("component should have a view");
        let mut triggers = Vec::new();
        collect_triggers(view, &mut triggers);
        assert_eq!(
            triggers,
            vec![(Some("fadeIn".to_string()), false)],
            "named trigger should be unchanged"
        );
    }

    #[test]
    fn numbers_anonymous_motions_sequentially_across_nested_views() {
        let source = r#"
import omniview;
component Multi {
    view {
        <div>
            <p @enter={{ opacity: 0 }}>One</p>
            @if true {
                <p @hover={{ scale: 0.5 }}>Two</p>
            }
            @for item in [] {
                <p @press={{ rotate: 90 }}>Three</p>
            }
        </div>
    }
}
"#;
        let ir = lower_source(source);
        let module = ir.modules.get("input.omni").expect("should have module");

        let names: Vec<&str> = module.motions.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["__anon_m_0", "__anon_m_1", "__anon_m_2"],
            "inline bodies get sequential synthesized names across the whole view tree"
        );

        let comp = module
            .components
            .get("Multi")
            .expect("should have Multi component");
        let view = comp.view.as_ref().expect("component should have a view");
        let mut triggers = Vec::new();
        collect_triggers(view, &mut triggers);
        assert_eq!(
            triggers,
            vec![
                (Some("__anon_m_0".to_string()), false),
                (Some("__anon_m_1".to_string()), false),
                (Some("__anon_m_2".to_string()), false),
            ],
            "all three triggers should be rewritten to named refs"
        );
    }
}
