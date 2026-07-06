use crate::parser::ast::view::{Element, ViewNode};

/// Walk a ViewNode tree, calling `on_element` for every Element node.
/// Handles structural recursion into IfDirective and ForDirective branches.
pub fn walk_view_nodes(nodes: &[ViewNode], on_element: &mut impl FnMut(&Element)) {
    for node in nodes {
        match node {
            ViewNode::Element(el) => {
                on_element(el);
                walk_view_nodes(&el.children, on_element);
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
            _ => {} // Text, Interpolation, SlotDirective: no children to recurse into
        }
    }
}
