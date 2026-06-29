//! Motion trigger and timeline validation for Pass 4.
//!
//! Validates timeline definitions, circular motion references, and view tree
//! motion triggers.  Extracted from `validate_motion` to keep that file below
//! 800 lines.

use std::collections::{HashMap, HashSet};

use super::helpers::is_non_interpolatable;
use super::motion::has_enter_leave_animation;
use super::SemanticAnalyzer;
use crate::error::Diagnostic;
use crate::lexer::Span;
use crate::parser::ast::*;

impl SemanticAnalyzer {
    /// Validate a single timeline definition (undefined refs, non-interpolatable props).
    pub(super) fn validate_timeline_def(
        &mut self,
        timeline: &TimelineDef,
        all_motion_names: &HashSet<&str>,
    ) {
        for item in &timeline.items {
            match &item.kind {
                TimelineItemKind::MotionRef(ref_name) => {
                    if !all_motion_names.contains(ref_name.as_str()) {
                        self.diagnostics.push(Diagnostic::error(
                            "E0006",
                            &format!(
                                "@timeline '{}': references undefined motion '{}'",
                                timeline.name, ref_name
                            ),
                            &timeline.span,
                        ));
                    }
                }
                TimelineItemKind::Direction(dir) => {
                    for (prop, _) in &dir.properties {
                        if is_non_interpolatable(prop) {
                            self.diagnostics.push(Diagnostic::error(
                                "E0006",
                                &format!(
                                    "@timeline '{}': '{}' is not interpolatable \
                                     and cannot be animated",
                                    timeline.name, prop
                                ),
                                &timeline.span,
                            ));
                        }
                    }
                }
                TimelineItemKind::Stagger(stagger) => {
                    if let Expr::VarRef(ref_name, _) = stagger.motion.as_ref() {
                        if !all_motion_names.contains(ref_name.as_str()) {
                            self.diagnostics.push(Diagnostic::error(
                                "E0006",
                                &format!(
                                    "@timeline '{}': references undefined motion '{}'",
                                    timeline.name, ref_name
                                ),
                                &timeline.span,
                            ));
                        }
                    }
                }
                TimelineItemKind::SetBlock(_)
                | TimelineItemKind::Label(_, _)
                | TimelineItemKind::StartPos(_) => {}
            }
        }
    }

    /// Detect circular motion references in timeline definitions.
    pub(super) fn detect_timeline_cycles(&mut self, refs: &[(&str, Vec<&str>)]) {
        let graph: HashMap<&str, &[&str]> = refs
            .iter()
            .map(|(name, deps)| (*name, deps.as_slice()))
            .collect();

        let mut reported: HashSet<&str> = HashSet::new();

        for &(start_name, _) in refs {
            if reported.contains(start_name) {
                continue;
            }
            // DFS from start, following only timeline nodes
            let mut visited: HashSet<&str> = HashSet::new();
            let mut stack: Vec<&str> = vec![start_name];
            visited.insert(start_name);

            while let Some(current) = stack.pop() {
                if let Some(neighbors) = graph.get(current) {
                    for &next in *neighbors {
                        if !graph.contains_key(next) {
                            // Only follow timeline nodes for cycle detection
                            continue;
                        }
                        if next == start_name {
                            reported.insert(start_name);
                            self.diagnostics.push(Diagnostic::error(
                                "E0006",
                                &format!(
                                    "Timeline '{}' contains a circular motion reference",
                                    start_name
                                ),
                                &Span {
                                    start: 0,
                                    end: 0,
                                    line: 0,
                                    col: 0,
                                },
                            ));
                            stack.clear();
                            break;
                        }
                        if !visited.contains(next) {
                            visited.insert(next);
                            stack.push(next);
                        }
                    }
                }
            }
        }
    }

    /// Validate motion triggers in view tree (expanded with @for key checking,
    /// element ID collection).
    pub(super) fn validate_view_motion_triggers(
        &mut self,
        nodes: &[ViewNode],
        all_motion_names: &HashSet<&str>,
        element_ids: &HashSet<String>,
    ) {
        for node in nodes {
            match node {
                ViewNode::Element(el) => {
                    self.validate_element_motion_props(el, all_motion_names);

                    // Validate @if mode on conditional branches within element
                    self.validate_view_motion_triggers(&el.children, all_motion_names, element_ids);
                }
                ViewNode::IfDirective(if_dir) => {
                    // Validate @if mode string if present
                    if let Some(ref mode) = if_dir.transition_mode {
                        self.validate_if_mode(mode, &if_dir.span);
                    }
                    self.validate_view_motion_triggers(&if_dir.body, all_motion_names, element_ids);
                    for (_, body) in &if_dir.else_ifs {
                        self.validate_view_motion_triggers(body, all_motion_names, element_ids);
                    }
                    if let Some(ref else_body) = if_dir.else_body {
                        self.validate_view_motion_triggers(
                            else_body,
                            all_motion_names,
                            element_ids,
                        );
                    }
                }
                ViewNode::ForDirective(for_dir) => {
                    // Check: @for with enter/leave/stagger triggers but no key={}
                    let has_anim = has_enter_leave_animation(&for_dir.body);
                    let has_key = for_dir.key_expr.is_some();
                    if has_anim && !has_key {
                        self.diagnostics.push(Diagnostic::error(
                            "E0006",
                            "@for with @enter/@leave/stagger animation requires key={} \
                             for identity tracking",
                            &for_dir.span,
                        ));
                    }
                    self.validate_view_motion_triggers(
                        &for_dir.body,
                        all_motion_names,
                        element_ids,
                    );
                    if let Some(ref empty_body) = for_dir.empty_body {
                        self.validate_view_motion_triggers(
                            empty_body,
                            all_motion_names,
                            element_ids,
                        );
                    }
                }
                ViewNode::Text(_) | ViewNode::Interpolation(_) | ViewNode::SlotDirective { .. } => {
                }
            }
        }
    }
}
