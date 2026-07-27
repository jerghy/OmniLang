//! Shared codegen infrastructure shared across multiple backend families.
//!
//! Contains the `FormatExpr`, `FormatStmt`, `FormatType` traits and their
//! language-family implementations.

pub mod expr;
pub(crate) mod generator;
pub(crate) mod html;
pub(crate) mod inline_handler;
pub(crate) mod stmt;
pub(crate) mod types;
pub mod ui_codegen;
pub(crate) mod visitor;
