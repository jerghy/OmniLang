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
pub use omnilang_core::codegen::event;
#[cfg(feature = "codegen-flutter")]
pub mod flutter;
pub(crate) use omnilang_core::codegen::helpers;
#[cfg(feature = "codegen-javascript")]
pub mod javascript;
pub use omnilang_core::codegen::json;
#[cfg(feature = "codegen-omniview")]
pub mod omniview;
#[cfg(feature = "codegen-python")]
pub mod python;
#[cfg(feature = "codegen-react")]
pub mod react;
#[cfg(feature = "codegen-rust")]
pub mod rust;
pub use omnilang_core::codegen::sanitize;
pub(crate) mod shared;
pub use omnilang_core::codegen::stdlib;
pub use omnilang_core::codegen::target;
pub use omnilang_core::codegen::traits;
#[cfg(feature = "codegen-vue")]
pub mod vue;
pub use omnilang_core::codegen::writer;

use crate::ir::IRProgram;
#[cfg(any(feature = "codegen-omniview", feature = "codegen-vue"))]
pub(crate) use event::emit_event_object;
#[cfg(any(feature = "codegen-react", feature = "codegen-vue"))]
pub(crate) use helpers::generate_css_style_block;
pub use helpers::{
    binary_op_to_str, camel_to_kebab, camel_to_separated, capitalize_first, format_block_as_expr,
    format_css_value, kebab_to_camel,
};
pub use json::{
    escape_literal_str, format_json_value, JsonArrayFmt, JsonFormatConfig, JsonObjectFmt,
    JSON_DART, JSON_DIOXUS, JSON_FLUTTER, JSON_JS, JSON_OMNIVIEW, JSON_PYTHON, JSON_REACT,
    JSON_RUST, JSON_VUE,
};
pub use omnilang_core::codegen::Target;
use rustc_hash::FxHashMap;
pub use sanitize::{sanitize_css_value, sanitize_js_ident, sanitize_module_name};
pub use stdlib::{try_format_stdlib_call, validate_stdlib_imports};
pub use traits::Codegen;
pub use writer::{CodeWriter, CodegenWarning, CompileOutput, CompiledFile};

/// Error emitted when design blocks are compiled to a target that does not
/// support them. Design blocks (semantic DSL) are only compiled by the
/// OmniView, React, and Flutter targets — all other targets fail with a hard
/// error rather than silently dropping design features.
#[cfg(any(
    feature = "codegen-python",
    feature = "codegen-javascript",
    feature = "codegen-rust",
    feature = "codegen-dart",
    feature = "codegen-vue",
    feature = "codegen-dioxus"
))]
const DESIGN_BLOCK_ERROR: &str =
    "Design blocks are only supported on the OmniView, React, and Flutter targets. \
    Design tokens, classes, and variants cannot be emitted for this target. \
    Compile with Target::OmniView, Target::React, or Target::Flutter for design system features.";

/// Check whether the IR program contains any non-empty design blocks.
#[cfg(any(
    feature = "codegen-python",
    feature = "codegen-javascript",
    feature = "codegen-rust",
    feature = "codegen-dart",
    feature = "codegen-vue",
    feature = "codegen-dioxus"
))]
fn has_design_blocks(ir: &IRProgram) -> bool {
    ir.modules.values().any(|m| !m.designs.is_empty())
}

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
    // Language array methods (filter/map/count) have cross-target mappings
    // only on the JS and Dart families (OmniView has its own generator).
    // On unmapped targets (Python, Rust, Dioxus) a verbatim `items.filter(..)`
    // would emit a nonexistent stdlib call — hard-gate with E_CODEGEN
    // (compiler MEDIUM, mirroring the design-block gate above).
    if let Err(errors) = crate::codegen::shared::expr::validate_array_method_floor(ir, target) {
        return CompileOutput {
            files: vec![],
            errors,
            source_map: vec![],
            warnings: vec![],
        };
    }
    #[allow(unreachable_patterns)]
    let mut output = match target {
        #[cfg(feature = "codegen-omniview")]
        Target::OmniView => omniview::OmniViewCodegen::default().generate(ir, source_files),
        #[cfg(feature = "codegen-python")]
        Target::Python => {
            if has_design_blocks(ir) {
                return CompileOutput::error(DESIGN_BLOCK_ERROR.into());
            }
            python::PythonCodegen.generate(ir, source_files)
        }
        #[cfg(feature = "codegen-javascript")]
        Target::JavaScript => {
            if has_design_blocks(ir) {
                return CompileOutput::error(DESIGN_BLOCK_ERROR.into());
            }
            javascript::JavaScriptCodegen::new(false).generate(ir, source_files)
        }
        #[cfg(feature = "codegen-javascript")]
        Target::TypeScript => {
            if has_design_blocks(ir) {
                return CompileOutput::error(DESIGN_BLOCK_ERROR.into());
            }
            javascript::JavaScriptCodegen::new(true).generate(ir, source_files)
        }
        #[cfg(feature = "codegen-rust")]
        Target::Rust => {
            if has_design_blocks(ir) {
                return CompileOutput::error(DESIGN_BLOCK_ERROR.into());
            }
            rust::RustCodegen.generate(ir, source_files)
        }
        #[cfg(feature = "codegen-dart")]
        Target::Dart => {
            if has_design_blocks(ir) {
                return CompileOutput::error(DESIGN_BLOCK_ERROR.into());
            }
            dart::DartCodegen.generate(ir, source_files)
        }
        #[cfg(feature = "codegen-react")]
        Target::React => react::ReactCodegen::default().generate(ir, source_files),
        #[cfg(feature = "codegen-vue")]
        Target::Vue => {
            if has_design_blocks(ir) {
                return CompileOutput::error(DESIGN_BLOCK_ERROR.into());
            }
            vue::VueCodegen.generate(ir, source_files)
        }
        #[cfg(feature = "codegen-dioxus")]
        Target::Dioxus => {
            if has_design_blocks(ir) {
                return CompileOutput::error(DESIGN_BLOCK_ERROR.into());
            }
            dioxus::DioxusCodegen.generate(ir, source_files)
        }
        // Flutter compiles design blocks (semantic DSL) — no gate. Unmapped
        // design features degrade to warnings emitted by the Flutter codegen.
        #[cfg(feature = "codegen-flutter")]
        Target::Flutter => flutter::FlutterCodegen.generate(ir, source_files),
        _ => CompileOutput::error(format!(
            "Backend for '{:?}' is not enabled (compile with the corresponding codegen-* feature flag)",
            target
        )),
    };
    // Thread source_map from IR through to CompileOutput — extend to preserve
    // per-element (NID-level) entries produced by OmniView codegen
    output.source_map.extend(ir.source_map.values().cloned());
    output
}

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
