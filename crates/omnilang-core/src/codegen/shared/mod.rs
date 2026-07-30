//! Shared codegen infrastructure shared across multiple backend families.
//!
//! Contains the `FormatExpr`, `FormatStmt`, `FormatType` traits and their
//! language-family implementations.

pub mod easing;
pub mod expr;
pub mod generator;
pub mod html;
pub mod inline_handler;
pub mod stmt;
pub mod types;
pub mod view_tree;
pub mod visitor;
