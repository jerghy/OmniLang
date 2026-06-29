//! Intent IR — types and lowering logic.
//!
//! Pipeline: AST → IR types → codegen / JSON serialization.

pub mod logic;
mod lower_ast;
pub mod lower_logic;
mod types;

pub use logic::*;
pub use lower_ast::lower;
pub use types::*;

pub(crate) use lower_ast::{lower_block, lower_expr, lower_type};

// ── Private imports visible to child modules via `use super::*;` ──
use std::collections::HashMap;
