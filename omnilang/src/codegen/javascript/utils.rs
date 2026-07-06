//! JS/TS type annotation and code generation utilities.
//!
//! Provides `js_type` (pub) for mapping OMNI types to JS/TS type strings,
//! and internal helpers for generating structs, enums, type aliases, and
//! standalone functions.

use crate::codegen::shared::types::{FormatType, JsTypeFormatter};
use crate::ir::*;

/// Map OMNI type to JS/TS type annotation string.
pub fn js_type(kind: &str, type_args: &[TypeIR], ts_mode: bool) -> String {
    if !ts_mode {
        return String::new();
    }
    JsTypeFormatter.format_type(kind, type_args)
}

pub(super) fn format_type_ir_js(ty: &TypeIR, ts_mode: bool) -> String {
    if !ts_mode {
        return String::new();
    }
    JsTypeFormatter.format_type(&ty.kind, &ty.type_args)
}
