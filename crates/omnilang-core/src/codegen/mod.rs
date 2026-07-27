//! Codegen module -- multi-target code generation from Intent IR.
//!
//! This is the core crate's shared codegen infrastructure.
//! Individual codegen backends live in separate crates.

pub mod event;
pub mod helpers;
pub mod json;
pub mod sanitize;
pub(crate) mod shared;
pub mod stdlib;
pub mod target;
pub mod traits;
pub mod writer;

use crate::ir::IRProgram;
pub use event::{emit_event_object, field_extract_js};
pub use helpers::{
    binary_op_to_str, camel_to_kebab, camel_to_separated, capitalize_first, format_block_as_expr,
    format_css_value,
};
pub use json::{
    escape_literal_str, format_json_value, JsonArrayFmt, JsonFormatConfig, JsonObjectFmt,
    JSON_DART, JSON_DIOXUS, JSON_FLUTTER, JSON_JS, JSON_OMNIVIEW, JSON_PYTHON, JSON_REACT,
    JSON_RUST, JSON_VUE,
};
use rustc_hash::FxHashMap;
pub use sanitize::{sanitize_css_value, sanitize_js_ident, sanitize_module_name};
pub use shared::html::{to_html_attr, to_html_tag};
pub use shared::ui_codegen::UiCodegen;
pub use stdlib::{try_format_stdlib_call, validate_stdlib_imports};
pub use target::Target;
pub use traits::Codegen;
pub use writer::{CodeWriter, CompileOutput, CompiledFile};

/// Compile IR to the specified target.
///
/// Routes to each target's codegen module. In the core crate, no backends
/// are compiled in. Backend crates provide their own `Codegen` implementations.
#[must_use]
pub fn compile(
    ir: &IRProgram,
    target: Target,
    #[allow(unused_variables)] source_files: &FxHashMap<String, String>,
    #[allow(unused_variables)] store_mode: Option<&str>,
) -> CompileOutput {
    // Validate stdlib imports for the target
    if let Err(errors) = validate_stdlib_imports(ir, target) {
        return CompileOutput {
            files: vec![],
            errors,
            source_map: vec![],
            warnings: vec![],
        };
    }
    #[allow(unreachable_patterns)]
    let mut output = match target {
        // No backends compiled into the core crate
        _ => CompileOutput::error(format!(
            "Backend for '{:?}' is not enabled (use the corresponding codegen crate)",
            target
        )),
    };
    // Thread source_map from IR through to CompileOutput
    output.source_map = ir.source_map.values().cloned().collect();
    output
}

// tests_property module removed — it contained backend-specific proptest code
// that is not part of the core crate.
