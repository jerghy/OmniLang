//! Codegen module -- multi-target code generation from Intent IR.
//!
//! Currently supported targets:
//! - OmniView: JS/HTML/CSS with inline signal runtime
//! - Python, JavaScript, Rust, Dart: pure-logic codegen
//! - React, Vue, Dioxus, Flutter: framework-based codegen

#[cfg(feature = "codegen-dart")]
pub mod dart;
#[cfg(feature = "codegen-dioxus")]
pub mod dioxus;
pub mod event;
#[cfg(feature = "codegen-flutter")]
pub mod flutter;
pub mod helpers;
#[cfg(feature = "codegen-javascript")]
pub mod javascript;
pub mod json;
#[cfg(feature = "codegen-omniview")]
pub mod omniview;
#[cfg(feature = "codegen-python")]
pub mod python;
#[cfg(feature = "codegen-react")]
pub mod react;
#[cfg(feature = "codegen-rust")]
pub mod rust;
pub mod sanitize;
pub(crate) mod shared;
pub mod stdlib;
pub mod target;
pub mod traits;
#[cfg(feature = "codegen-vue")]
pub mod vue;
pub mod writer;

use crate::ir::IRProgram;
#[cfg(any(feature = "codegen-omniview", feature = "codegen-vue"))]
pub(crate) use event::emit_event_object;
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
use rustc_hash::FxHashMap;
pub use sanitize::{sanitize_css_value, sanitize_js_ident, sanitize_module_name};
pub use shared::ui_codegen::UiCodegen;
pub use stdlib::{try_format_stdlib_call, validate_stdlib_imports};
pub use target::Target;
pub use traits::Codegen;
pub use writer::{CodeWriter, CompileOutput, CompiledFile};

/// Compile IR to the specified target.
///
/// Routes to each target's codegen module. All 10 targets are now
/// implemented (OmniView, Python, JavaScript, TypeScript, Rust, Dart,
/// React, Vue, Dioxus, Flutter).
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
        };
    }
    #[allow(unreachable_patterns)]
    let mut output = match target {
        #[cfg(feature = "codegen-omniview")]
        Target::OmniView => omniview::compile_files(ir, source_files, false, false),
        #[cfg(feature = "codegen-python")]
        Target::Python => python::PythonCodegen.generate(ir, source_files),
        #[cfg(feature = "codegen-javascript")]
        Target::JavaScript => {
            javascript::JavaScriptCodegen::new(false).generate(ir, source_files)
        }
        #[cfg(feature = "codegen-javascript")]
        Target::TypeScript => {
            javascript::JavaScriptCodegen::new(true).generate(ir, source_files)
        }
        #[cfg(feature = "codegen-rust")]
        Target::Rust => rust::RustCodegen.generate(ir, source_files),
        #[cfg(feature = "codegen-dart")]
        Target::Dart => dart::DartCodegen.generate(ir, source_files),
        #[cfg(feature = "codegen-react")]
        Target::React => {
            let mode: react::ReactStoreMode = store_mode
                .unwrap_or("hooks")
                .parse()
                .unwrap_or_default();
            react::ReactCodegen { store_mode: mode }.generate(ir, source_files)
        }
        #[cfg(feature = "codegen-vue")]
        Target::Vue => vue::VueCodegen.generate(ir, source_files),
        #[cfg(feature = "codegen-dioxus")]
        Target::Dioxus => dioxus::DioxusCodegen.generate(ir, source_files),
        #[cfg(feature = "codegen-flutter")]
        Target::Flutter => flutter::FlutterCodegen.generate(ir, source_files),
        _ => CompileOutput::error(format!(
            "Backend for '{:?}' is not enabled (compile with the corresponding codegen-* feature flag)",
            target
        )),
    };
    // Thread source_map from IR through to CompileOutput
    output.source_map = ir.source_map.values().cloned().collect();
    output
}

#[cfg(test)]
mod tests_property;

#[cfg(all(test, feature = "all-backends"))]
mod tests_regression;
