//! Motion trigger validation for Pass 4.
//!
//! Validates view tree motion triggers and related rules.

use super::motion::has_enter_leave_animation;
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
                        self.validate_view_motion_triggers(
                            else_body,
                            all_motion_names,
                        );
                    }
                }
                ViewNode::ForDirective(for_dir) => {
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
                    self.validate_view_motion_triggers(
                        &for_dir.body,
                        all_motion_names,
                    );
                    if let Some(ref empty_body) = for_dir.empty_body {
                        self.validate_view_motion_triggers(
                            empty_body,
                            all_motion_names,
                        );
                    }
                }
                ViewNode::Text(_) | ViewNode::Interpolation(_) | ViewNode::SlotDirective { .. } => {
                }
            }
        }
    }
}
