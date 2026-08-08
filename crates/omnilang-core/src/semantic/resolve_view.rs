//! Pass 2 (merged): view-tree resolution — walks interpolations, element
//! props (style/class/events/ref-bindings/slot content/a11y), directives
//! (`@if`/`@for`), and inline event-handler scopes.

use super::helpers::type_to_string;
use super::symbols::{Symbol, SymbolKind};
use super::validate_inline::ValidationContext;
use super::SemanticAnalyzer;
use crate::parser::ast::*;

// ── View tree resolution ────────────────────────────

impl SemanticAnalyzer {
    /// Recursively resolve all expressions within a ViewNode tree.
    pub(super) fn resolve_view_node_refs(&mut self, node: &ViewNode, vctx: &mut ValidationContext) {
        match node {
            ViewNode::Interpolation(expr) => {
                self.resolving_interpolation = true;
                self.resolve_expr_refs(expr, vctx);
                self.resolving_interpolation = false;
            }
            ViewNode::Element(element) => {
                self.resolve_element_refs(element, vctx);
            }
            ViewNode::IfDirective(dir) => {
                self.resolve_if_directive_refs(dir, vctx);
            }
            ViewNode::ForDirective(dir) => {
                self.resolve_for_directive_refs(dir, vctx);
            }
            // @load has no expressions to resolve (bare fn name; lowering
            // wires the call in Task 2). @form/@validate are resolved via
            // their Static/value exprs — handled below.
            ViewNode::LoadDirective { .. } | ViewNode::SlotDirective { .. } | ViewNode::Text(_) => {
            }
        }
    }

    fn resolve_element_refs(&mut self, element: &Element, vctx: &mut ValidationContext) {
        for prop in &element.props {
            match prop {
                ElementProp::Static { value, .. } => {
                    self.resolve_expr_refs(value, vctx);
                }
                ElementProp::Style { entries, .. } => {
                    for (_, expr) in entries {
                        self.resolve_expr_refs(expr, vctx);
                    }
                }
                ElementProp::Class { value } => {
                    if let ClassValue::ClassMap(entries) = value {
                        for (_, expr) in entries {
                            self.resolve_expr_refs(expr, vctx);
                        }
                    }
                }
                ElementProp::Event { handler, .. } => {
                    self.resolve_event_handler_refs(handler, vctx);
                }
                // The driving signal expression is resolved like any other
                // prop expression (ref-existence validation is Phase 2.1).
                ElementProp::RefBinding { signal, .. } => {
                    self.resolve_expr_refs(signal, vctx);
                }
                // Named slot projection content (ADR-053 D5) is a view
                // subtree — resolve references inside it like children.
                ElementProp::SlotContent { content, .. } => {
                    for node in content {
                        self.resolve_view_node_refs(node, vctx);
                    }
                }
                // a11y block expressions (hidden/alt) reference signals
                // like any other prop expression.
                ElementProp::A11y(spec) => {
                    if let Some(ref hidden) = spec.hidden {
                        self.resolve_expr_refs(hidden, vctx);
                    }
                    if let Some(ref alt) = spec.alt {
                        self.resolve_expr_refs(alt, vctx);
                    }
                }
                // @drag/@dragEnd handler names are bare component-function
                // refs (ADR-073) — resolved via validate_drag, not here.
                // validate: rule values are plain exprs — resolved in Task 3's
                // validate_form_load pass together with @form/@load.
                ElementProp::Bind { .. }
                | ElementProp::Ref(_)
                | ElementProp::MotionTrigger(_)
                | ElementProp::Variant { .. }
                | ElementProp::Drag { .. }
                | ElementProp::Form
                | ElementProp::Validate { .. } => {}
            }
        }
        // Children of a component instantiation are consumer-side
        // projections — a `{slot}` interpolation among them is an attempt
        // to forward this component's slot into the child component, which
        // codegen cannot do today (the materialized slot would be treated
        // as the child's own projection). Track the position so
        // resolve_expr_refs can warn.
        let is_component = self.module_component_names.contains(element.tag.as_str());
        let saved = self.inside_component_children.take();
        if is_component {
            self.inside_component_children = Some(element.tag.clone());
        }
        for child in &element.children {
            self.resolve_view_node_refs(child, vctx);
        }
        self.inside_component_children = saved;
    }

    fn resolve_event_handler_refs(&mut self, handler: &EventHandler, vctx: &mut ValidationContext) {
        match handler {
            EventHandler::InlineFn { body, params, .. } => {
                // Push a temporary scope for inline fn params
                self.symbols.push_scope("inline_handler");
                for p in params {
                    self.declare_or_warn(Symbol {
                        name: p.name.clone(),
                        kind: SymbolKind::Variable,
                        type_ann: p.type_ann.as_ref().map(type_to_string),
                        mutable: false,
                        declared_at: 0,
                    });
                }
                self.resolve_block_refs(body, vctx);
                self.symbols.pop_scope();
            }
            EventHandler::FnRef(_) | EventHandler::StoreAction { .. } => {}
        }
    }

    fn resolve_if_directive_refs(&mut self, dir: &IfDirective, vctx: &mut ValidationContext) {
        self.resolve_expr_refs(&dir.condition, vctx);
        for child in &dir.body {
            self.resolve_view_node_refs(child, vctx);
        }
        for (condition, body) in &dir.else_ifs {
            self.resolve_expr_refs(condition, vctx);
            for child in body {
                self.resolve_view_node_refs(child, vctx);
            }
        }
        if let Some(ref else_body) = dir.else_body {
            for child in else_body {
                self.resolve_view_node_refs(child, vctx);
            }
        }
    }

    fn resolve_for_directive_refs(&mut self, dir: &ForDirective, vctx: &mut ValidationContext) {
        self.resolve_expr_refs(&dir.iterable, vctx);
        // Push a temporary scope for the loop variable
        self.symbols.push_scope("for_loop");
        self.declare_or_warn(Symbol {
            name: dir.item_name.clone(),
            kind: SymbolKind::Variable,
            type_ann: None,
            mutable: false,
            declared_at: 0,
        });
        // Resolve key_expr inside the loop scope (may reference loop variable)
        if let Some(ref key_expr) = dir.key_expr {
            self.resolve_expr_refs(key_expr, vctx);
        }
        for child in &dir.body {
            self.resolve_view_node_refs(child, vctx);
        }
        if let Some(ref empty_body) = dir.empty_body {
            for child in empty_body {
                self.resolve_view_node_refs(child, vctx);
            }
        }
        self.symbols.pop_scope();
    }
}
