//! Intent IR — types and lowering logic.
//!
//! Pipeline: AST → IR types → codegen / JSON serialization.

pub mod logic;
mod lower;
pub mod lower_logic;
mod types;
mod types_motion;
pub mod visibility;

pub use logic::*;
pub use lower::lower;
pub use types::*;
pub use types_motion::*;

pub(crate) use lower::{lower_block, lower_type};

// ── Private imports visible to child modules via `use super::*;` ──
use std::collections::HashMap;
