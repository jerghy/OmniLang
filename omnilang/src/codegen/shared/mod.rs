//! Shared codegen infrastructure shared across multiple backend families.
//!
//! Contains the `FormatExpr` trait and language-family implementations
//! that eliminate expression formatting duplication across 8 backends.

pub mod expr;
pub(crate) mod stmt;
