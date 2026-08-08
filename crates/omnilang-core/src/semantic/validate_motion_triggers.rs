//! Motion trigger validation for Pass 4.
//!
//! Validates view tree motion triggers and related rules.

use super::motion::{body_has_motion_trigger, has_enter_leave_animation};
use super::SemanticAnalyzer;
use crate::error::{Diagnostic, DiagnosticKind};
use crate::parser::ast::*;
use rustc_hash::FxHashSet;

impl SemanticAnalyzer {
    /// Validate motion triggers in view tree.
    pub(super) fn validate_view_motion_triggers(
        &mut self,
        nodes: &[ViewNode],
        all_motion_names: &FxHashSet<&str>,
    ) {
        for node in nodes {
            match node {
                ViewNode::Element(el) => {
                    self.validate_element_motion_props(el, all_motion_names);

                    self.validate_view_motion_triggers(&el.children, all_motion_names);
                }
                ViewNode::IfDirective(if_dir) => {
                    // Validate @if mode string if present
                    if let Some(ref mode) = if_dir.transition_mode {
                        self.validate_if_mode(mode, &if_dir.span);
                    }
                    self.validate_view_motion_triggers(&if_dir.body, all_motion_names);
                    for (_, body) in &if_dir.else_ifs {
                        self.validate_view_motion_triggers(body, all_motion_names);
                    }
                    if let Some(ref else_body) = if_dir.else_body {
                        self.validate_view_motion_triggers(else_body, all_motion_names);
                    }
                }
                ViewNode::ForDirective(for_dir) => {
                    // Task 4.9 (perf M2 + D-H): motion triggers on elements
                    // inside the @for body silently diverge per backend —
                    // React shares a single motion ref (only the last
                    // mounted item animates), Flutter allocates one ordinal
                    // per template element (all N items replay the same
                    // controller). Per-item allocation is a long-term item;
                    // warn once per @for (honest degradation, never silent).
                    if body_has_motion_trigger(&for_dir.body) {
                        self.diagnostics.push(Diagnostic::warning(
                            DiagnosticKind::WMotion,
                            "motion 在 @for 子元素尚未支持逐项实例化；当前退化为单一共享实例（React）或全部项同时播放（Flutter）；如需逐项动画，请将动画触发器移到 @for 外层的容器元素（逐项实例化列入长期计划）",
                            &for_dir.span,
                        ));
                    }
                    // Check: @for with enter animation but no key={}
                    let has_anim = has_enter_leave_animation(&for_dir.body);
                    let has_key = for_dir.key_expr.is_some();
                    if has_anim && !has_key {
                        self.diagnostics.push(Diagnostic::error(
                            DiagnosticKind::E0006,
                            "@for with @enter animation requires key={} \
                             for identity tracking",
                            &for_dir.span,
                        ));
                    }
                    self.validate_view_motion_triggers(&for_dir.body, all_motion_names);
                    if let Some(ref empty_body) = for_dir.empty_body {
                        self.validate_view_motion_triggers(empty_body, all_motion_names);
                    }
                }
                // @load is a leaf directive — no motion content inside.
                ViewNode::Text(_)
                | ViewNode::Interpolation(_)
                | ViewNode::SlotDirective { .. }
                | ViewNode::LoadDirective { .. } => {}
            }
        }
    }
}
