//! List rendering dispatch for view-tree processing.
//!
//! The `@for` rendering logic lives in the sibling `list.rs` at the
//! `codegen::omniview` level.  This module exists to keep the `view/`
//! directory structure complete and provides a clear boundary for any
//! future list-specific view-tree dispatch logic.
//!
//! Currently the dispatch is handled inline inside `view_to_html` via
//! a direct call to `self.emit_list_rendering_html()`.
