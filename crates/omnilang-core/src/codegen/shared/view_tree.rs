use crate::ir::ElementIR;
use crate::ir::ViewTree;

/// Walk a ViewTree recursively. Returns true if any Element satisfies the predicate.
pub fn view_tree_any(view: &ViewTree, predicate: impl Fn(&ElementIR) -> bool) -> bool {
    view_tree_any_dyn(view, &predicate)
}

fn view_tree_any_dyn(view: &ViewTree, predicate: &dyn Fn(&ElementIR) -> bool) -> bool {
    match view {
        ViewTree::Element(el) => {
            if predicate(el) {
                return true;
            }
            el.children
                .iter()
                .any(|child| view_tree_any_dyn(child, predicate))
        }
        ViewTree::Conditional(cond) => {
            cond.branches.iter().any(|b| {
                b.body
                    .as_ref()
                    .is_some_and(|v| view_tree_any_dyn(v, predicate))
            }) || cond
                .fallback
                .as_ref()
                .is_some_and(|v| view_tree_any_dyn(v, predicate))
        }
        ViewTree::ListRendering(list) => {
            list.body
                .as_ref()
                .is_some_and(|v| view_tree_any_dyn(v, predicate))
                || list
                    .empty_body
                    .as_ref()
                    .is_some_and(|v| view_tree_any_dyn(v, predicate))
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) | ViewTree::Slot(_) => false,
    }
}
