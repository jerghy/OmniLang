//! Codegen module -- multi-target code generation from Intent IR.
//!
//! Currently supported targets:
//! - OmniView: JS/HTML/CSS with inline signal runtime
//! - Python, JavaScript, Rust, Dart: pure-logic codegen
//! - React, Vue, Dioxus, Flutter: framework-based codegen

pub mod event;
pub mod helpers;
pub mod json;
pub mod sanitize;
pub mod shared;
pub mod stdlib;
pub mod target;
pub mod traits;
pub mod writer;

#[cfg(any(feature = "codegen-omniview", feature = "codegen-vue"))]
pub use event::emit_event_object;
#[cfg(any(feature = "codegen-react", feature = "codegen-vue"))]
pub(crate) use helpers::generate_css_style_block;
pub use helpers::{
    binary_op_to_str, camel_to_kebab, camel_to_separated, capitalize_first, format_block_as_expr,
    format_css_value,
};
pub use json::{
    escape_literal_str, format_json_value, JsonArrayFmt, JsonFormatConfig, JsonObjectFmt,
    JSON_DART, JSON_DIOXUS, JSON_FLUTTER, JSON_JS, JSON_OMNIVIEW, JSON_PYTHON, JSON_REACT,
    JSON_RUST, JSON_VUE,
};
pub use sanitize::{sanitize_css_value, sanitize_js_ident, sanitize_module_name};
pub use shared::html::to_html_tag;
pub use shared::ui_codegen::UiCodegen;
pub use stdlib::{try_format_stdlib_call, validate_stdlib_imports};
pub use target::Target;
pub use traits::Codegen;
pub use writer::{CodeWriter, CompileOutput, CompiledFile};

// The `compile` dispatch function lives in the omnilang crate (facade).
// omnilang-core only provides shared codegen infrastructure — backends
// are implemented in separate crates (omnilang-codegen-omniview, etc.)

#[cfg(test)]
mod tests_property;

#[cfg(all(
    test,
    any(
        feature = "codegen-omniview",
        feature = "codegen-react",
        feature = "codegen-flutter"
    )
))]
mod tests_regression;
