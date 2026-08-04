//! OmniLang compiler — .omni source -> Intent IR -> JSON / OmniView codegen.
//!
//! Pipeline:
//!   1. Lexer  → token stream
//!   2. Parser → AST
//!   3. Semantic analysis → validated AST + symbol table + type map
//!   4. IR lowering → Intent IR
//!   5. Codegen / JSON serialization

pub mod analyse;
pub mod codegen;
pub mod discovery;
pub mod error;
pub mod ffi;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod path_mapping;
pub mod semantic;
pub mod sourcemap;
pub mod stdlib;
pub mod validate;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use ts_rs::TS;

/// A single symbol from semantic analysis — variable, signal, prop, store, etc.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_ann: Option<String>,
    pub mutable: bool,
}

/// Top-level compile result — returned as JSON by WASM/native bindings.
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct CompileResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "Record<string, unknown> | null")]
    pub ir: Option<ir::IRProgram>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<error::Diagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<SymbolInfo>,
    #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
    pub dependencies: FxHashMap<String, Vec<String>>,
}

/// Compile .omni source -> IR. Core function shared by WASM and native.
#[must_use]
pub fn compile(source: &str) -> CompileResult {
    let output = run_pipeline(source);
    CompileResult {
        success: !output.has_errors,
        ir: Some(output.ir),
        errors: output.diagnostics,
        symbols: output.symbols,
        dependencies: output.dependencies,
    }
}

// ═══════════════════════════════════════════════════════
// Shared Pipeline
// ═══════════════════════════════════════════════════════

/// Pipeline result after lex->parse->semantic->IR lowering.
/// Public so the facade crate can delegate to this single source of truth.
pub struct PipelineOutput {
    pub ir: ir::IRProgram,
    pub diagnostics: Vec<error::Diagnostic>,
    pub has_errors: bool,
    pub symbols: Vec<SymbolInfo>,
    pub dependencies: FxHashMap<String, Vec<String>>,
}

/// Run the shared compilation pipeline: lex -> parse -> semantic -> IR lowering.
/// Always returns a PipelineOutput with all diagnostics (parse errors, semantic
/// errors, etc.) bundled together. Parse errors are included in `diagnostics`.
pub fn run_pipeline(source: &str) -> PipelineOutput {
    let tokens = lexer::tokenize(source);
    let (program, parse_errors) = parser::parse(tokens, source);
    let mut diagnostics: Vec<error::Diagnostic> =
        parse_errors.into_iter().map(|e| e.into()).collect();

    let mut semantics = semantic::analyze(&program);

    // Extract symbols before ir::lower() which borrows semantics mutably
    let symbols: Vec<SymbolInfo> = semantics
        .symbols
        .scopes()
        .iter()
        .flat_map(|scope| scope.symbols.values())
        .map(|sym| SymbolInfo {
            name: sym.name.clone(),
            kind: format!("{:?}", sym.kind).to_lowercase(),
            type_ann: sym.type_ann.clone(),
            mutable: sym.mutable,
        })
        .collect();
    let dependencies = std::mem::take(&mut semantics.dependencies);

    diagnostics.extend(std::mem::take(&mut semantics.diagnostics));
    let mut ir = ir::lower(&program, &mut semantics, None);
    // Re-collect diagnostics emitted during lowering (validate_required_props,
    // validate_refs, validate_ref_ops, design validation).
    diagnostics.extend(std::mem::take(&mut semantics.diagnostics));
    crate::ir::optimize::optimize(&mut ir);

    // Program-level theme validation (@themes ↔ @theme pairing, design D1).
    // Runs on the full IR so cross-file pairing (design.omni +
    // ThemeStore.omni) is visible; also populates DesignIR.theme_signal.
    crate::semantic::validate_theme::validate_theme(&mut ir, &mut diagnostics);

    // Event-name validation (Task 4.2, D-B) — unknown event names warn with
    // the nearest legal suggestion. Registered here too (single-file scope:
    // custom `event` declarations and component tags visible in this
    // program), so compile(), compile_omniview(), compile_omniview_full()
    // and resolve_styles() — the WASM-exported paths — see the same
    // diagnostics as multi-file analysis. Without this, a single-file
    // `@onTotallyBogus={f}` still compiled "No issues found" (ai HIGH).
    crate::semantic::validate_event_names::validate_event_names(&ir, &mut diagnostics);

    // Phase: lint rules (same 10 rules as `analyse_multi`, so single-file
    // paths — compile(), compile_omniview(), resolve_styles() — see the
    // same diagnostics as multi-file analysis). Lint warnings fold into
    // diagnostics at Warning severity (matching how semantic-analysis
    // warnings already flow); lint errors fold in at Error severity and
    // therefore flip has_errors. `analyse_multi` is a disjoint pipeline
    // that never calls run_pipeline, so lints still run exactly once per
    // compile regardless of entry point.
    let mut lint_programs: FxHashMap<String, Arc<crate::parser::ast::Program>> =
        FxHashMap::default();
    lint_programs.insert("<input>".to_string(), Arc::new(program));
    for d in &crate::semantic::lints::run_all(&lint_programs, &ir) {
        let mut diag = crate::analyse::lint_to_diag(d);
        if crate::analyse::is_lint_error(d) {
            diag.severity = crate::error::Severity::Error;
        }
        diagnostics.push(diag);
    }

    // Recompute has_errors after the lint phase — lint errors must surface
    // through compile().
    let has_errors = diagnostics
        .iter()
        .any(|d| matches!(d.severity, crate::error::Severity::Error));
    PipelineOutput {
        ir,
        diagnostics,
        has_errors,
        symbols,
        dependencies,
    }
}

// ═══════════════════════════════════════════════════════
// OmniView / Style Resolution / Multi-Target Codegen
// ═══════════════════════════════════════════════════════
//
// Moved to individual backend crates and the omnilang facade crate.
// omnilang-core only provides shared types and infrastructure.

/// Internal compile result struct (avoids JSON round-trip for native consumers).
#[derive(Debug, Serialize, TS)]
pub struct InternalCompileResult {
    pub success: bool,
    #[ts(type = "Record<string, string>")]
    pub files: FxHashMap<String, String>,
    pub errors: Vec<crate::error::Diagnostic>,
    pub warnings: Vec<crate::error::Diagnostic>,
    /// Per-component JS deltas for hot-replace (keyed by component name).
    /// Only populated for OmniView target with codegen-omniview enabled.
    #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
    #[ts(type = "Record<string, { js: string }>")]
    pub components: FxHashMap<String, serde_json::Value>,
    /// Source map entries mapping generated code back to .omni source positions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(
        type = "Array<{ node_id: string; omni_span: { line: number; col: number; len: number }; target_spans: Array<{ file: string; line: number; col: number; len: number; kind: string }> }>"
    )]
    pub source_map: Vec<crate::sourcemap::SourceMapEntry>,
}

// ═══════════════════════════════════════════════════════
// Multi-target compile / OmniView + backend orchestration
// ═══════════════════════════════════════════════════════
//
// `compile_to_target_json`, `compile_multi_internal`, `compile_multi_internal_cached`,
// `compile_multi_internal_post`, and `compile_multi_to_target` have been moved to the
// omnilang facade crate. They depend on backend-specific codegen modules that are
// intentionally excluded from omnilang-core.

// ═══════════════════════════════════════════════════════
// TS Export (generates TypeScript type definitions)
// ═══════════════════════════════════════════════════════

// Export all `#[derive(TS)]` types as TypeScript definition files.
// ── Code-block parsing ──────────────────────────────────────

/// Result of parsing code-block source into structured AST statements.
/// NOTE: Code-block parsing returns empty stmts — structured code-block
/// AST parsing is deferred to the Projection Editor (方案 3).
/// Code blocks are currently stored and emitted as raw text via Monaco.
#[derive(Debug, Serialize)]
pub struct ParseStmtsResult {
    pub success: bool,
    pub stmts: Vec<serde_json::Value>,
    pub errors: Vec<error::Diagnostic>,
}

/// Parse a code-block source string into a list of AST statements.
/// Used by WASM/JS `parse_code_block()`.
///
/// Currently returns an empty success result. Structured code-block
/// parsing will be implemented as part of the Projection Editor.
pub fn parse_stmts(_source: &str) -> ParseStmtsResult {
    ParseStmtsResult {
        success: true,
        stmts: vec![],
        errors: vec![],
    }
}

///
/// Generated files go to `poiesis/src/commands/__generated__/`.
/// Dependencies of each root type are exported automatically.
/// Run: `cargo test export_bindings --lib`
#[cfg(test)]
mod type_exports {
    use crate::{error, CompileResult};
    use ts_rs::TS;

    #[test]
    fn export_bindings() {
        // Set the export directory relative to the crate root
        let export_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("poiesis")
            .join("src")
            .join("commands")
            .join("__generated__");
        std::fs::create_dir_all(&export_dir).expect("failed to create __generated__ directory");

        // Export CompileResult and all its dependencies (Diagnostic, Severity)
        CompileResult::export_all_to(&export_dir)
            .expect("failed to export CompileResult TS bindings");

        // Also export Diagnostic directly (in case it's not pulled in transitively)
        error::Diagnostic::export_all_to(&export_dir)
            .expect("failed to export Diagnostic TS bindings");
    }
}

#[cfg(test)]
mod source_map_integration_tests {
    use crate::ir::IRProgram;

    #[test]
    fn ir_program_has_source_map() {
        let ir = IRProgram::default();
        assert!(
            ir.source_map.is_empty(),
            "default IRProgram should have empty source_map"
        );
    }
}

#[cfg(test)]
mod pipeline_tests {
    use crate::compile;

    /// Regression: lower-phase diagnostics (validate_required_props,
    /// validate_refs, validate_ref_ops, design validation) must surface
    /// through compile() — has_errors must be recomputed after ir::lower().
    #[test]
    fn compile_surfaces_lower_phase_ref_op_errors() {
        let result = compile(
            "component App {\n    ref box: Element;\n    view {\n        <div @ref={box} />\n    }\n    fn go() {\n        box.getBoundingClientRect();\n    }\n}",
        );
        assert!(
            !result.success,
            "expected compile failure for illegal ref op"
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("getBoundingClientRect")),
            "expected getBoundingClientRect diagnostic, got: {:?}",
            result.errors
        );
    }

    /// Task 4.2 single-file path (review I-1): run_pipeline (and therefore
    /// compile(), compile_omniview(), compile_omniview_full(), resolve_styles
    /// — the WASM-exported entry points) must surface unknown-event warnings
    /// just like multi-file analysis. Regression: single-file
    /// `@onTotallyBogus={f}` used to compile clean ("No issues found"),
    /// keeping the ai HIGH silent no-op alive on the WASM export path.
    #[test]
    fn compile_surfaces_unknown_event_warning() {
        let result = compile(
            "component App {\n    fn f(e) {}\n    view {\n        <div @onTotallyBogus={f} />\n    }\n}",
        );
        let warn = result
            .errors
            .iter()
            .find(|d| {
                d.severity == crate::error::Severity::Warning
                    && d.message.contains("onTotallyBogus")
            });
        assert!(
            warn.is_some(),
            "single-file compile must warn on unknown event names, got: {:?}",
            result
                .errors
                .iter()
                .map(|d| (&d.message, &d.severity))
                .collect::<Vec<_>>()
        );
        let w = warn.expect("checked above");
        assert!(
            w.message.contains("@onTap") || w.message.contains("@onClick") || w.message.contains("@onPress"),
            "missing nearest suggestion: {}",
            w.message
        );
    }

    #[test]
    fn compile_accepts_legal_ref_ops() {
        let result = compile(
            "component App {\n    ref box: Element;\n    view {\n        <div @ref={box} />\n    }\n    fn go() {\n        box.focus();\n        let w = box.width;\n        let v = box.value;\n    }\n}",
        );
        assert!(
            result.success,
            "expected success for legal ref ops, got errors: {:?}",
            result.errors
        );
    }

    /// Design D3: `capturePointer(id)` / `releasePointer(id)` are parameterless
    /// — any argument fails the compile with the 域 1 诊断表 hint; the zero-arg
    /// forms stay legal.
    #[test]
    fn compile_rejects_capture_pointer_with_args_accepts_parameterless() {
        for (call, expected) in [
            ("box.capturePointer(1)", "capturePointer 无参数"),
            ("box.releasePointer(1)", "releasePointer 无参数"),
        ] {
            let source = format!(
                "component App {{\n    ref box: Element;\n    view {{\n        <div @ref={{box}} />\n    }}\n    fn go() {{\n        {};\n    }}\n}}",
                call
            );
            let result = compile(&source);
            assert!(!result.success, "`{}` must fail to compile", call);
            assert!(
                result.errors.iter().any(|e| {
                    e.message.contains(expected) && e.message.contains("多指针锁定不支持")
                }),
                "`{}` must diagnose with the D3 hint, got: {:?}",
                call,
                result.errors
            );
        }
        let result = compile(
            "component App {\n    ref box: Element;\n    view {\n        <div @ref={box} />\n    }\n    fn go() {\n        box.capturePointer();\n        box.releasePointer();\n    }\n}",
        );
        assert!(
            result.success,
            "parameterless capturePointer()/releasePointer() must compile, got: {:?}",
            result.errors
        );
    }

    /// Regression: the imperative ref write (`ref.value = x` / `ref.setValue(x)`)
    /// was removed (RefOpIR 11 → 10 ops) — the write forms must hard-diagnose
    /// with the bind:value migration hint and fail the compile.
    #[test]
    fn compile_diagnoses_ref_value_writes_with_bind_value_hint() {
        for write in ["box.value = \"x\"", "box.setValue(\"x\")"] {
            let source = format!(
                "component App {{\n    ref box: Element;\n    view {{\n        <div @ref={{box}} />\n    }}\n    fn go() {{\n        {};\n    }}\n}}",
                write
            );
            let result = compile(&source);
            assert!(
                !result.success,
                "write form `{}` must fail to compile",
                write
            );
            assert!(
                result
                    .errors
                    .iter()
                    .any(|e| e.message.contains("命令式写已删除")
                        && e.message.contains("bind:value")),
                "write form `{}` must diagnose with the bind:value hint, got: {:?}",
                write,
                result.errors
            );
        }
    }

    /// Design D2 (值绑定收敛): the `value={sig}` semantic-binding form is
    /// removed — `bind:value` is the sole two-way binding and `value={expr}`
    /// is a plain one-way prop. The bare-signal form must hard-diagnose with
    /// the migration hint and fail the compile (even with @ref present).
    #[test]
    fn compile_rejects_value_sig_semantic_binding_form() {
        let source = r#"
component InputComp {
    signal text: String = "";
    ref field: Element;
    view {
        <input ref:field value={text} />
    }
}
"#;
        let result = compile(source);
        assert!(
            !result.success,
            "value={{sig}} must fail to compile, got: {:?}",
            result.errors
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("bind:value") && e.message.contains("value prop")),
            "value={{sig}} must diagnose with the migration hint, got: {:?}",
            result.errors
        );
    }

    /// Design D2: `value={expr}` with a computed expression stays a plain
    /// one-way prop — no diagnostic, compile succeeds.
    #[test]
    fn compile_accepts_computed_value_prop() {
        let source = r#"
component InputComp {
    signal age: i32 = 25;
    view {
        <input type="number" value={age.to_string()} />
    }
}
"#;
        let result = compile(source);
        assert!(
            result.success,
            "computed value prop must compile, got: {:?}",
            result.errors
        );
        assert!(
            result
                .errors
                .iter()
                .all(|e| !matches!(e.severity, crate::error::Severity::Error)),
            "computed value prop must produce no error diagnostics, got: {:?}",
            result.errors
        );
    }

    /// Task 0.6 — lint rules must run in the shared single-file pipeline
    /// (lifted from analyse_multi) so that compile() surfaces them.
    #[test]
    fn compile_runs_lint_warnings() {
        // `unused` signal is never referenced → dead-code lint warning.
        let result = compile("component App { signal unused: i32 = 0; view { <div /> } }");
        assert!(
            result.success,
            "lint warnings must not flip success, got: {:?}",
            result.errors
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("never used")),
            "dead-code lint warning must surface through compile(), got: {:?}",
            result.errors
        );
    }

    /// Task 0.6 — lint-free inputs must stay diagnostic-free (no new
    /// warnings from the lifted lint phase).
    #[test]
    fn compile_clean_input_stays_clean() {
        let result = compile("component App { view { <div>ok</div> } }");
        assert!(
            result.success,
            "expected success, got errors: {:?}",
            result.errors
        );
        assert!(
            result.errors.is_empty(),
            "clean input must produce no diagnostics, got: {:?}",
            result.errors
        );
    }
}
