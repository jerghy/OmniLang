use crate::parser::ast::view::{ComponentDef, Element, ElementProp, ViewNode};

/// Walk a ViewNode tree, calling `on_element` for every Element node.
/// Handles structural recursion into IfDirective and ForDirective branches,
/// slot directives (fallback content), and consumer slot projections
/// (`ElementProp::SlotContent`), ADR-053 D5.
pub fn walk_view_nodes(nodes: &[ViewNode], on_element: &mut impl FnMut(&Element)) {
    for node in nodes {
        match node {
            ViewNode::Element(el) => {
                on_element(el);
                walk_view_nodes(&el.children, on_element);
                // Consumer slot projections (header={<div/>}) hold view
                // content in the prop itself — recurse into it.
                for prop in &el.props {
                    if let ElementProp::SlotContent { content, .. } = prop {
                        walk_view_nodes(content, on_element);
                    }
                }
            }
            ViewNode::IfDirective(dir) => {
                walk_view_nodes(&dir.body, on_element);
                for (_, body) in &dir.else_ifs {
                    walk_view_nodes(body, on_element);
                }
                if let Some(ref b) = dir.else_body {
                    walk_view_nodes(b, on_element);
                }
            }
            ViewNode::ForDirective(dir) => {
                walk_view_nodes(&dir.body, on_element);
                if let Some(ref b) = dir.empty_body {
                    walk_view_nodes(b, on_element);
                }
            }
            ViewNode::SlotDirective {
                default_content, ..
            } => {
                // View-level @slot fallback content is real view content.
                if let Some(ref content) = default_content {
                    walk_view_nodes(content, on_element);
                }
            }
            // @load is a leaf directive — no element content to walk.
            ViewNode::LoadDirective { .. } | ViewNode::Text(_) | ViewNode::Interpolation(_) => {}
        }
    }
}

/// Walk all view content of a component: the view block children plus the
/// fallback content of body-level slot declarations (ADR-053 D5).
pub fn walk_component_view_nodes(comp: &ComponentDef, on_element: &mut impl FnMut(&Element)) {
    if let Some(ref view) = comp.view {
        walk_view_nodes(&view.children, on_element);
    }
    for slot in &comp.slots {
        if let Some(ref content) = slot.default_content {
            walk_view_nodes(content, on_element);
        }
    }
}
